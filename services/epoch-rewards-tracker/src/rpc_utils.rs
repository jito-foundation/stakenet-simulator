use anchor_lang::prelude::SlotHistory;
use regex::Regex;
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::{
    client_error::{ClientError, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcBlockConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    rpc_request::RpcError,
    rpc_response::RpcInflationReward,
};
use solana_sdk::{
    account::from_account,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    slot_history,
    stake::{program::ID as STAKE_PROGRAM_ID, state::StakeStateV2},
};
use solana_transaction_status_client_types::{
    TransactionDetails, UiConfirmedBlock, UiTransactionEncoding,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcUtilsError {
    #[error("SolanaClientError error: {0}")]
    SolanaClientError(#[from] ClientError),
    #[error(transparent)]
    RpcError(#[from] RpcError),
    #[error("Block was skipped")]
    SkippedBlock,
    #[error("Slot {0} not found. SlotHistory not up to date or slot in future")]
    SlotInFuture(u64),
    #[error("Slot {0} not found on RPC, but on SlotHistory sysvar")]
    InSlotHistoryNotOnRpc(u64),
    #[error("Custom: {0}")]
    Custom(String),
}

use crate::EpochRewardsTrackerError;

pub async fn get_inflation_rewards(
    rpc_client: &RpcClient,
    stake_accounts: &[Pubkey],
    epoch: u64,
) -> Result<Vec<Option<RpcInflationReward>>, EpochRewardsTrackerError> {
    let res = rpc_client
        .get_inflation_reward(stake_accounts, Some(epoch))
        .await?;
    Ok(res)
}

pub async fn fetch_stake_accounts_for_validator(
    client: &RpcClient,
    vote_pubkey: &Pubkey,
) -> Result<Vec<(Pubkey, StakeStateV2)>, EpochRewardsTrackerError> {
    let discriminator_filter =
        RpcFilterType::Memcmp(Memcmp::new_base58_encoded(0, &2u32.to_le_bytes()));
    let vote_pubkey_filter = RpcFilterType::Memcmp(Memcmp::new(
        4 + 120, // u32 enum + size of Meta
        MemcmpEncodedBytes::Base58(vote_pubkey.to_string()),
    ));
    let blank_decativation_epoch = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        4 + 120 + 48, // u32 enum + size of Meta + offset in Satke
        &u64::MAX.to_le_bytes(),
    ));
    let config = RpcProgramAccountsConfig {
        filters: Some(vec![
            discriminator_filter,
            vote_pubkey_filter,
            blank_decativation_epoch,
        ]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64Zstd),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: None,
        sort_results: Some(true),
    };
    let accounts = client
        .get_program_accounts_with_config(&STAKE_PROGRAM_ID, config)
        .await?;

    Ok(accounts
        .into_iter()
        .map(|(pubkey, account)| {
            let mut data: &[u8] = &account.data;
            let bond = <StakeStateV2 as borsh::BorshDeserialize>::deserialize(&mut data).unwrap();
            (pubkey, bond)
        })
        .collect())
}

/// Wrapper on Solana RPC get_block, but propagates skipped blocks as RpcUtilsError
pub async fn get_block(
    client: &RpcClient,
    slot: u64,
    slot_history: &SlotHistory,
) -> Result<UiConfirmedBlock, RpcUtilsError> {
    let block_res = client
        .get_block_with_config(
            slot,
            RpcBlockConfig {
                encoding: Some(UiTransactionEncoding::Json),
                transaction_details: Some(TransactionDetails::None),
                rewards: Some(true),
                commitment: Some(CommitmentConfig::finalized()),
                max_supported_transaction_version: Some(0),
            },
        )
        .await;
    match block_res {
        Ok(block) => Ok(block),
        Err(err) => match err.kind {
            ClientErrorKind::RpcError(client_rpc_err) => match client_rpc_err {
                RpcError::RpcResponseError {
                    code,
                    message,
                    data,
                } => {
                    // These slot skipped errors come from RpcCustomError::SlotSkipped or
                    //  RpcCustomError::LongTermStorageSlotSkipped and may not always mean
                    //  there is no block for a given slot. The additional context are:
                    //  "...or missing due to ledger jump to recent snapshot"
                    //  "...or missing in long-term storage"
                    // Meaning they can arise from RPC issues or lack of history (limit ledger
                    //  space, no big table) accesible  by an RPC. This is why we check
                    // SlotHistory and then follow up with redundant RPC checks.
                    let slot_skipped_regex = Regex::new(r"^Slot [\d]+ was skipped").unwrap();
                    if slot_skipped_regex.is_match(&message) {
                        match slot_history.check(slot) {
                            slot_history::Check::Future => {
                                return Err(RpcUtilsError::SlotInFuture(slot));
                            }
                            slot_history::Check::NotFound => {
                                return Err(RpcUtilsError::SkippedBlock);
                            }
                            slot_history::Check::TooOld | slot_history::Check::Found => {
                                return Err(RpcUtilsError::InSlotHistoryNotOnRpc(slot));
                            }
                        }
                    }
                    Err(RpcUtilsError::RpcError(RpcError::RpcResponseError {
                        code,
                        message,
                        data,
                    }))
                }
                _ => Err(RpcUtilsError::RpcError(client_rpc_err)),
            },
            _ => Err(RpcUtilsError::SolanaClientError(err)),
        },
    }
}

pub async fn fetch_slot_history(
    client: &RpcClient,
) -> Result<slot_history::SlotHistory, RpcUtilsError> {
    let account_data = client
        .get_account(&solana_sdk::sysvar::slot_history::ID)
        .await
        .map_err(|e| RpcUtilsError::Custom(format!("Failed to fetch SlotHistory: {}", e)))?;
    let slot_history = from_account::<slot_history::SlotHistory, _>(&account_data)
        .ok_or_else(|| RpcUtilsError::Custom("Failed to deserialize SlotHistory".to_string()))?;
    Ok(slot_history)
}
