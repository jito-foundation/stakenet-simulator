use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use sqlx::postgres::PgPoolOptions;
use std::{env, error::Error, str::FromStr, sync::Arc};
use steward_simulator_cli::commands::{BacktestArgs, handle_backtest};
use tracing::info;

const EPOCH_DURATION_SECS: i64 = 2 * 24 * 3600;
const SOLANA_GENESIS_TIME: &str = "2020-03-16T00:00:00Z";

#[derive(Debug, Deserialize)]
struct ApiResponse {
    apy: Vec<ApyRecord>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ApyRecord {
    data: f64,
    date: String,
}

#[derive(Debug, Serialize)]
struct ApyWithEpoch {
    data: f64,
    date: String,
    epoch: u64,
}

fn estimate_epoch_from_time(date_str: &str) -> Result<i64, Box<dyn Error>> {
    let genesis = DateTime::parse_from_rfc3339(SOLANA_GENESIS_TIME)?.with_timezone(&Utc);
    let dt = DateTime::parse_from_rfc3339(date_str)?.with_timezone(&Utc);
    let diff = dt.timestamp() - genesis.timestamp();
    Ok((diff / EPOCH_DURATION_SECS).max(0))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing subscriber to enable logging at INFO level
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let db_url = env::var("DB_CONNECTION_URL")
        .unwrap_or_else(|_| "postgresql://postgres:postgres@127.0.0.1:54322/postgres".to_string());

    let rpc_url =
        env::var("RPC_URL").unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let db_conn_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(10)
            .connect(&db_url)
            .await?,
    );
    info!("Connected to DB: {}", db_url);

    let rpc_client = RpcClient::new(rpc_url.clone());
    info!("RPC client initialized at {}", rpc_url);

    let steward_config_str = env::var("STEWARD_CONFIG")
        .unwrap_or_else(|_| "jitoVjT9jRUyeXHzvCwzPgHj7yWNRhLcUoXtes4wtjv".to_string());
    let steward_config_pubkey = Pubkey::from_str(&steward_config_str)?;
    info!("Using steward config: {}", steward_config_pubkey);

    let client = Client::new();
    let current_date = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);

    let payload = serde_json::json!({
        "bucket_type": "Daily",
        "range_filter": {
            "start": "2022-10-31T00:00:00Z",
            "end": current_date
        },
        "sort_by": {
            "field": "BlockTime",
            "order": "Asc"
        }
    });

    let jito_resp = client
        .post("https://kobe.mainnet.jito.network/api/v1/stake_pool_stats")
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if !jito_resp.status().is_success() {
        return Err(format!("Jito API request failed: {}", jito_resp.status()).into());
    }

    let jito_json: ApiResponse = jito_resp.json().await?;
    let rpc_payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getEpochInfo"
    });

    let rpc_resp = client
        .post(&rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send()
        .await?;

    if !rpc_resp.status().is_success() {
        return Err(format!("RPC request failed: {}", rpc_resp.status()).into());
    }

    let rpc_val: Value = rpc_resp.json().await?;
    let rpc_epoch = rpc_val["result"]["epoch"]
        .as_i64()
        .ok_or("RPC: failed to parse result.epoch")?;

    let est_now = estimate_epoch_from_time(&Utc::now().to_rfc3339())?;
    let offset = rpc_epoch - est_now;

    // Query max epoch from epoch_rewards table
    // This is limited by the data available in data.csv which is used to populate epoch_rewards
    let max_epoch: i64 =
        sqlx::query_scalar::<_, sqlx::types::BigDecimal>("SELECT MAX(epoch) FROM epoch_rewards")
            .fetch_one(db_conn_pool.as_ref())
            .await?
            .to_string()
            .parse::<i64>()?;

    info!("Max epoch in epoch_rewards table: {}", max_epoch);

    // Generate epoch ranges dynamically based on max available epoch
    // Testing various 50-100 epoch windows to compare Jito APY vs simulated APY
    // Note: Epochs before 735 have delinquency_score = 0 causing all validators to score 0.
    // Use epoch 740 as minimum to ensure accurate scoring (see CONSIDERATIONS.md)
    const MIN_EPOCH: i64 = 740;
    let epoch_ranges = vec![(max_epoch - 100, max_epoch), (max_epoch - 50, max_epoch)]
        .into_iter()
        .filter(|(start, _)| *start >= MIN_EPOCH)
        .collect::<Vec<_>>();

    for (start_epoch, end_epoch) in epoch_ranges {
        let apy_with_epochs: Vec<ApyWithEpoch> = jito_json
            .apy
            .iter()
            .map(|rec| {
                let est = estimate_epoch_from_time(&rec.date).unwrap_or(0);
                ApyWithEpoch {
                    data: rec.data,
                    date: rec.date.clone(),
                    epoch: (est + offset).max(0) as u64,
                }
            })
            .collect();

        let filtered_apy: Vec<ApyWithEpoch> = apy_with_epochs
            .into_iter()
            .filter(|r| r.epoch >= start_epoch as u64 && r.epoch <= end_epoch as u64)
            .collect();

        let avg_apy = if !filtered_apy.is_empty() {
            filtered_apy.iter().map(|r| r.data).sum::<f64>() / filtered_apy.len() as f64
        } else {
            0.0
        };

        println!(
            "Epochs {}-{} => Jito APY: {:.4}%",
            start_epoch,
            end_epoch,
            avg_apy * 100.0
        );

        let args = BacktestArgs::default();
        let calculated_apy = handle_backtest(
            args,
            &db_conn_pool,
            &rpc_client,
            end_epoch as u16,
            (end_epoch - start_epoch) as u16,
            &steward_config_pubkey,
        )
        .await?;
        println!(
            "Epochs {}-{} => Backtest APY: {:.4}%",
            start_epoch,
            end_epoch,
            calculated_apy * 100.0
        );
    }

    Ok(())
}
