use crate::{
    cluster_history::load_and_record_cluster_history, config::Config,
    errors::EpochRewardsTrackerError, inflation::gather_inflation_rewards,
    priority_fees::gather_priority_fee_data_for_epoch, rpc_utils::fetch_slot_history,
    stake_accounts::gather_stake_accounts,
    validator_history_utils::load_and_record_validator_history,
};
use clap::{Parser, Subcommand};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use sqlx::postgres::PgPoolOptions;
use std::{str::FromStr, sync::Arc};
use tracing::Level;
use tracing_subscriber::EnvFilter;

mod cluster_history;
mod config;
mod dune;
mod errors;
mod fetch_active_stake;
mod fetch_inactive_stake;
mod inflation;
mod priority_fees;
mod retry_utils;
mod rpc_utils;
mod stake_accounts;
mod validator_history_utils;
mod withdraw_and_deposit_sol;
mod withdraw_and_deposits;

#[derive(Parser, Debug)]
pub struct GlobalArgs {
    #[arg(short, long, env)]
    pub rpc_url: String,

    #[arg(
        long,
        env,
        default_value = "postgresql://postgres:postgres@127.0.0.1:54322/postgres"
    )]
    pub db_connection_url: String,

    #[arg(long, env, default_value_t = validator_history::ID.to_string())]
    pub validator_history_program_id: String,

    #[arg(long, env, default_value_t = 60)]
    pub epoch_check_cycle_sec: u64,
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(flatten)]
    pub globals: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    FetchValidatorHistory,
    FetchClusterHistory,
    GetStakeAccounts,
    GetInflationRewards,
    WithdrawAndDepositStake,
    WithdrawAndDepositSol,
    FetchActiveStake,
    FetchInactiveStake,
    GetPriorityFeeDataForEpoch { epoch: u64 },
}

#[tokio::main]
async fn main() -> Result<(), EpochRewardsTrackerError> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    let level = std::env::var("RUST_LOG").unwrap_or(Level::INFO.to_string());
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::new(level))
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let cli: Cli = Cli::parse();
    let config = Config {
        rpc_url: cli.globals.rpc_url,
        validator_history_program_id: cli.globals.validator_history_program_id,
        db_connection_url: cli.globals.db_connection_url,
    };

    let db_conn_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(5)
            .connect(&config.db_connection_url)
            .await
            .map_err(|_| EpochRewardsTrackerError::DatabaseConnectionError)?,
    );

    let rpc_client = Arc::new(RpcClient::new(config.rpc_url.clone()));
    let validator_history_program_id = Pubkey::from_str(&config.validator_history_program_id)
        .map_err(|_| EpochRewardsTrackerError::InvalidPubkeyError)?;
    match cli.command {
        Commands::FetchValidatorHistory => {
            load_and_record_validator_history(
                &db_conn_pool,
                &rpc_client,
                validator_history_program_id,
            )
            .await?
        }
        Commands::FetchClusterHistory => {
            load_and_record_cluster_history(&db_conn_pool, &rpc_client).await?
        }
        Commands::GetStakeAccounts => gather_stake_accounts(&db_conn_pool, &rpc_client).await?,
        Commands::GetInflationRewards => {
            gather_inflation_rewards(&db_conn_pool, &rpc_client).await?
        }
        Commands::GetPriorityFeeDataForEpoch { epoch } => {
            gather_priority_fee_data_for_epoch(
                &db_conn_pool,
                &rpc_client,
                epoch,
                &rpc_client.get_epoch_schedule().await?,
                &fetch_slot_history(&rpc_client).await?,
            )
            .await?
        }
        // THESE DO NOT REQUIRE AN RPC CLIENT
        Commands::FetchActiveStake => fetch_active_stake::fetch_active_stake(&db_conn_pool).await?,
        Commands::FetchInactiveStake => {
            fetch_inactive_stake::fetch_inactive_stake(&db_conn_pool).await?
        }
        Commands::WithdrawAndDepositStake => {
            withdraw_and_deposits::withdraw_and_deposits(&db_conn_pool).await?
        }
        Commands::WithdrawAndDepositSol => {
            withdraw_and_deposit_sol::withdraw_and_deposit_sol(&db_conn_pool).await?
        }
    }

    Ok(())
}
