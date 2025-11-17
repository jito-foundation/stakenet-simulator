use clap::Parser;
use serde::{Deserialize, Serialize};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::clock::Slot;
use solana_sdk::epoch_schedule::EpochSchedule;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct EpochSlotInfo {
    epoch: u64,
    start_slot: Slot,
    end_slot: Slot,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Generate epoch to slot mapping for Solana")]
struct Args {
    /// RPC URL for Solana cluster
    #[arg(
        long,
        env = "RPC_URL",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc_url: String,

    /// Starting epoch number
    #[arg(long, default_value = "0")]
    start_epoch: u64,

    /// Ending epoch number (inclusive). If not specified, uses current epoch + 1
    #[arg(long)]
    end_epoch: Option<u64>,

    /// Output pretty-printed JSON
    #[arg(long)]
    pretty: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    let args = Args::parse();

    // Create RPC client
    let rpc_client = RpcClient::new(args.rpc_url.clone());

    // Get epoch schedule
    let epoch_schedule = rpc_client.get_epoch_schedule().await?;

    // Get current epoch info if end_epoch not specified
    let end_epoch = if let Some(end) = args.end_epoch {
        end
    } else {
        let epoch_info = rpc_client.get_epoch_info().await?;
        epoch_info.epoch + 1 // Include current epoch + 1
    };

    if args.start_epoch > end_epoch {
        eprintln!(
            "Error: start_epoch ({}) cannot be greater than end_epoch ({})",
            args.start_epoch, end_epoch
        );
        std::process::exit(1);
    }

    // Generate epoch slot mappings
    let mut epoch_slots = Vec::new();

    for epoch in args.start_epoch..=end_epoch {
        let start_slot = get_first_slot_in_epoch(&epoch_schedule, epoch);
        let end_slot = get_last_slot_in_epoch(&epoch_schedule, epoch);

        epoch_slots.push(EpochSlotInfo {
            epoch,
            start_slot,
            end_slot,
        });
    }

    // Output as JSON
    let json_output = if args.pretty {
        serde_json::to_string_pretty(&epoch_slots)?
    } else {
        serde_json::to_string(&epoch_slots)?
    };

    println!("{}", json_output);

    Ok(())
}

/// Get the first slot of an epoch
fn get_first_slot_in_epoch(epoch_schedule: &EpochSchedule, epoch: u64) -> Slot {
    if epoch == 0 {
        0
    } else {
        // For epochs during warmup
        if epoch < epoch_schedule.first_normal_epoch {
            let mut slot = 0;
            for e in 0..epoch {
                slot += epoch_schedule.get_slots_in_epoch(e);
            }
            slot
        } else {
            // For normal epochs (after warmup)
            let normal_epoch_index = epoch - epoch_schedule.first_normal_epoch;
            let warmup_slots = epoch_schedule.first_normal_slot;
            warmup_slots + (normal_epoch_index * epoch_schedule.slots_per_epoch)
        }
    }
}

/// Get the last slot of an epoch (inclusive)
fn get_last_slot_in_epoch(epoch_schedule: &EpochSchedule, epoch: u64) -> Slot {
    get_first_slot_in_epoch(epoch_schedule, epoch + 1) - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epoch_calculations() {
        // Create a test epoch schedule
        let epoch_schedule = EpochSchedule::default();

        // Test epoch 0
        assert_eq!(get_first_slot_in_epoch(&epoch_schedule, 0), 0);

        // Test that last slot of epoch N is one less than first slot of epoch N+1
        for epoch in 0..10 {
            let last_slot = get_last_slot_in_epoch(&epoch_schedule, epoch);
            let next_first_slot = get_first_slot_in_epoch(&epoch_schedule, epoch + 1);
            assert_eq!(last_slot + 1, next_first_slot);
        }
    }
}
