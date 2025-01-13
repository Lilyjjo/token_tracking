use alloy::primitives::Address;
use clap::{
    Parser,
    ValueEnum,
};
use eyre::{
    Result,
    WrapErr,
};
use tracing::error;
use tracing_subscriber::{
    fmt::format::FmtSpan,
    EnvFilter,
};

mod abi;
use std::str::FromStr;
mod live_track_activity;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Which processing mode to use
    #[arg(value_enum)]
    mode: Mode,

    /// Block number for single block processing
    #[arg(long, required_if_eq("mode", "single"))]
    block_number: Option<u64>,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Process a single block
    Single,
    /// Process blocks live
    Live,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .with_thread_ids(false)
        .with_target(false)
        .with_span_events(FmtSpan::NONE)
        .with_line_number(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set tracing subscriber")?;

    // Set token and pool addresses above
    let rpc_url = std::env::var("RPC_URL").expect("RPC_URL is required");
    let token = Address::from_str(&std::env::var("TOKEN").expect("TOKEN is required"))
        .expect("token address error");
    let pool = Address::from_str(&std::env::var("POOL").expect("POOL is required"))
        .expect("pool address error");

    // Parse command line arguments
    let cli = Cli::parse();

    match cli.mode {
        Mode::Single => {
            let block_number = cli
                .block_number
                .expect("Block number is required for single mode");
            match live_track_activity::process_single_block(rpc_url, block_number, token, pool)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    error!("Block processing error {}", e);
                }
            }
        }
        Mode::Live => match live_track_activity::live_process_blocks(rpc_url, token, pool).await {
            Ok(_) => {}
            Err(e) => {
                error!("Block processing error {}", e);
            }
        },
    }

    Ok(())
}
