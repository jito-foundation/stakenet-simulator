use anchor_lang::AccountDeserialize;
use jito_steward::Config;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;

use crate::error::CliError;

pub async fn fetch_config(
    rpc_client: &RpcClient,
    steward_config_pubkey: &Pubkey,
) -> Result<Config, CliError> {
    let account = rpc_client.get_account(steward_config_pubkey).await?;
    let mut data: &[u8] = &account.data;
    Config::try_deserialize(&mut data).map_err(|_| CliError::AnchorDeserializeError)
}
