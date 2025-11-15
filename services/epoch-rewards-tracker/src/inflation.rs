// TODO: For each validator load a stake account that has a long history
use crate::{EpochRewardsTrackerError, retry_utils::retry_with_exponential_backoff, rpc_utils};
use futures::stream::{self, StreamExt};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use sqlx::{Pool, Postgres};
use stakenet_simulator_db::{inflation_rewards::InflationReward, stake_accounts::StakeAccount};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

pub async fn gather_inflation_rewards(
    db_connection: &Pool<Postgres>,
    rpc_client: &RpcClient,
) -> Result<(), EpochRewardsTrackerError> {
    let stake_account_keys = StakeAccount::get_all_pubkeys(db_connection).await?;
    let stake_account_keys: Vec<Pubkey> = stake_account_keys
        .into_iter()
        .filter_map(|x| Pubkey::from_str(&x).ok())
        .collect();

    // Fetch current epoch from RPC
    let current_epoch = rpc_client.get_epoch_info().await?.epoch;
    let end_epoch = current_epoch.saturating_sub(1); // Use current_epoch - 1 as the end
    let start_epoch = 700u64; // Keep the same start epoch for now

    info!(
        "Fetching inflation rewards from epoch {} to {} (current epoch: {})",
        start_epoch, end_epoch, current_epoch
    );

    // We have to limit the number of concurrent requests to prevent RPC rate limits
    let semaphore = Arc::new(Semaphore::new(10));
    let db_connection = Arc::new(db_connection.clone());
    let rpc_client = Arc::new(rpc_client);

    let tasks: Vec<_> = stake_account_keys
        .chunks(30)
        .flat_map(|stake_accounts| {
            let stake_accounts = stake_accounts.to_vec();
            let semaphore = semaphore.clone();
            let db_connection = db_connection.clone();
            let rpc_client = rpc_client.clone();

            (start_epoch..=end_epoch).map(move |epoch| {
                let semaphore = semaphore.clone();
                let db_connection = db_connection.clone();
                let rpc_client = rpc_client.clone();
                let stake_accounts = stake_accounts.clone();

                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    process_batch_epoch(&db_connection, &rpc_client, &stake_accounts, epoch).await
                }
            })
        })
        .collect();

    info!("Starting parallel processing of {} tasks", tasks.len());

    let results: Vec<_> = stream::iter(tasks).buffer_unordered(50).collect().await;

    for (i, result) in results.iter().enumerate() {
        if let Err(e) = result {
            warn!("Task {} failed: {:?}", i, e);
        }
    }

    Ok(())
}

async fn process_batch_epoch(
    db_connection: &Pool<Postgres>,
    rpc_client: &RpcClient,
    stake_accounts: &[Pubkey],
    epoch: u64,
) -> Result<(), EpochRewardsTrackerError> {
    let result = retry_with_exponential_backoff(
        3,     // max_retries
        1000,  // initial_delay_ms (1 second)
        10000, // max_delay_ms (10 seconds)
        || async {
            info!(
                "Fetching inflation rewards for {} stake accounts in epoch {}",
                stake_accounts.len(),
                epoch
            );

            let rewards =
                rpc_utils::get_inflation_rewards(rpc_client, stake_accounts, epoch).await?;

            let records: Vec<InflationReward> = rewards
                .into_iter()
                .zip(stake_accounts)
                .filter_map(
                    |(maybe_inflation_reward, stake_account)| match maybe_inflation_reward {
                        Some(reward) => Some(InflationReward::from_rpc_inflation_reward(
                            reward,
                            stake_account,
                        )),
                        None => {
                            debug!(
                                "No inflation reward found for stake account {} in epoch {}",
                                stake_account, epoch
                            );
                            None
                        }
                    },
                )
                .collect();

            if !records.is_empty() {
                InflationReward::bulk_insert(db_connection, records).await?;
            }

            Ok::<(), EpochRewardsTrackerError>(())
        },
    )
    .await;

    if let Err(e) = &result {
        error!(
            "Failed to process stake accounts {:?} for epoch {} after retries: {:?}",
            stake_accounts.iter().take(3).collect::<Vec<_>>(),
            epoch,
            e
        );
    }

    result
}
