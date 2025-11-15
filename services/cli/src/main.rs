use crate::error::CliError;
use clap::{Parser, Subcommand};
use commands::backtest::*;
use solana_client::nonblocking::rpc_client::RpcClient;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use tracing::Level;
use tracing_subscriber::EnvFilter;

pub mod commands;
pub mod error;
pub mod macros;
pub mod steward_utils;
mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[arg(short, long, env)]
    pub rpc_url: Option<String>,

    #[arg(
        long,
        env,
        default_value = "postgresql://postgres:postgres@127.0.0.1:54322/postgres"
    )]
    pub db_connection_url: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Backtest {
        #[command(flatten)]
        args: BacktestArgs,
    },
}

#[tokio::main]
async fn main() -> Result<(), CliError> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    // Init logger
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

    // Parse CLI args
    let cli: Cli = Cli::parse();

    // Build DB connection pool
    let db_conn_pool = Arc::new(
        PgPoolOptions::new()
            .max_connections(10)
            .connect(&cli.db_connection_url)
            .await
            .unwrap(),
    );

    match cli.command {
        Commands::Backtest { args } => {
            let rpc_url = cli.rpc_url.as_ref().ok_or(CliError::InvalidRPCUrl)?;
            let rpc_client = RpcClient::new(rpc_url.to_string());

            // Query max epoch from epoch_rewards table
            // This is limited by the data available in data.csv which is used to populate epoch_rewards.
            // The epoch_rewards table contains historical inflation, MEV, and priority fee data required
            // for accurate backtest simulations.
            let current_epoch: u16 =
                sqlx::query_scalar::<_, i64>("SELECT MAX(epoch) FROM epoch_rewards")
                    .fetch_one(db_conn_pool.as_ref())
                    .await
                    .map_err(|e| CliError::Custom(format!("Failed to query max epoch: {}", e)))?
                    .try_into()
                    .map_err(|e| CliError::Custom(format!("Invalid epoch value: {}", e)))?;

            tracing::info!("Using max epoch {} from epoch_rewards table", current_epoch);

            // TODO: Determine how this should be passed. The number of epochs to look back
            let look_back_period = 100;

            handle_backtest(
                args,
                &db_conn_pool,
                &rpc_client,
                current_epoch,
                look_back_period,
            )
            .await?;
            Ok(())
        }
    }
}
