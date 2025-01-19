use alloy::primitives::Address;
use clap::{
    Parser,
    ValueEnum,
};
use eyre::{
    Result,
    WrapErr,
};
use tracing::{
    error,
    info,
};
use tracing_subscriber::{
    fmt::format::FmtSpan,
    EnvFilter,
};

mod abi;
use std::{
    collections::HashSet,
    str::FromStr,
};
mod pool_sql;
mod process_blocks;
mod rpc;
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Which processing mode to use
    #[arg(value_enum)]
    mode: Mode,

    /// Block number for single block processing
    #[arg(long, required_if_eq("mode", "single_block"))]
    block_number: Option<u64>,

    /// Start block for blocks from mode
    #[arg(long, required_if_eq("mode", "blocks_from"))]
    start_block: Option<u64>,

    /// End block for blocks from mode
    #[arg(long, required_if_eq("mode", "blocks_from"))]
    end_block: Option<u64>,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Process a single block
    SingleBlock,
    /// Process blocks live
    BlocksFrom,
    /// Live track new blocks
    LiveTrack,
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

    let retry_config = rpc::RetryConfig::new(
        std::env::var("RETRY_MAX_ATTEMPTS")
            .expect("RETRY_MAX_ATTEMPTS is required")
            .parse()
            .expect("RETRY_MAX_ATTEMPTS must be a number"),
        std::env::var("RETRY_INITIAL_BACKOFF_MS")
            .expect("RETRY_INITIAL_BACKOFF_MS is required")
            .parse()
            .expect("RETRY_INITIAL_BACKOFF_MS must be a number"),
        std::env::var("RETRY_MAX_BACKOFF_MS")
            .expect("RETRY_MAX_BACKOFF_MS is required")
            .parse()
            .expect("RETRY_MAX_BACKOFF_MS must be a number"),
        std::env::var("RETRY_BACKOFF_MULTIPLIER")
            .expect("RETRY_BACKOFF_MULTIPLIER is required")
            .parse()
            .expect("RETRY_BACKOFF_MULTIPLIER must be a number"),
    );

    let pool_deployer_addresses = std::env::var("POOL_DEPLOYER_CONTRACT_ADDRESSES")
        .expect("POOL_DEPLOYER_CONTRACT_ADDRESSES is required");
    let pool_deployer_addresses: HashSet<Address> = pool_deployer_addresses
        .split(',')
        .map(|p| Address::from_str(p).expect("pool address error"))
        .collect();

    info!("Pool deployer addresses: {:?}", pool_deployer_addresses);

    let uniswap_v3_factory_address = std::env::var("UNISWAP_V3_FACTORY_ADDRESS")
        .expect("UNISWAP_V3_FACTORY_ADDRESS is required");
    let uniswap_v3_factory_address: Address = uniswap_v3_factory_address
        .parse()
        .expect("UNISWAP_V3_FACTORY_ADDRESS must be a valid address");

    // Set token and pool addresses above
    let http_url = std::env::var("HTTP_URL").expect("HTTP_URL is required");
    let wss_url = std::env::var("WSS_URL").expect("WSS_URL is required");
    let delay_ms = std::env::var("BLOCK_FROM_RPC_DELAY")
        .expect("BLOCK_FROM_RPC_DELAY is required")
        .parse()
        .expect("BLOCK_FROM_RPC_DELAY must be a number");
    // Parse command line arguments
    let cli = Cli::parse();

    // Get all pools already being tracked in the database
    let mut conn = pool_sql::database_interactions::establish_connection()?;
    let mut pools: HashSet<Address> =
        pool_sql::database_interactions::find_all_tracked_pools(&mut conn)?
            .into_iter()
            .collect();

    match cli.mode {
        Mode::SingleBlock => {
            let block_number = cli
                .block_number
                .expect("Block number is required for single mode");
            match process_blocks::single_block(
                http_url,
                block_number,
                uniswap_v3_factory_address,
                &pool_deployer_addresses,
                &mut pools,
                retry_config,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    error!("Block processing error {}", e);
                }
            }
        }
        Mode::BlocksFrom => {
            let start_block = cli
                .start_block
                .expect("Start block is required for blocks from mode");
            let end_block = cli
                .end_block
                .expect("End block is required for blocks from mode");
            match process_blocks::blocks_from(
                http_url,
                start_block,
                end_block,
                uniswap_v3_factory_address,
                &pool_deployer_addresses,
                &mut pools,
                retry_config,
                delay_ms,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    error!("Block processing error {}", e);
                }
            }
        }
        Mode::LiveTrack => {
            match process_blocks::live_blocks(
                http_url,
                wss_url,
                uniswap_v3_factory_address,
                &pool_deployer_addresses,
                &mut pools,
                retry_config,
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    error!("Block processing error {}", e);
                }
            }
        }
    }

    Ok(())
}
