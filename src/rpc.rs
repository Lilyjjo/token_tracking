use std::{
    future::Future,
    sync::Arc,
    time::Duration,
};

use alloy::{
    network::{
        AnyNetwork,
        AnyReceiptEnvelope,
        Network,
    },
    providers::{
        ProviderBuilder,
        RootProvider,
        WsConnect,
    },
    pubsub::PubSubFrontend,
    rpc::{
        client::{
            ClientBuilder,
            RpcClient,
        },
        types::{
            serde_helpers::WithOtherFields,
            Log,
            TransactionReceipt,
        },
    },
    transports::http::{
        reqwest,
        Http,
    },
};
use eyre::{
    bail,
    Error,
    Result,
    WrapErr,
};
use serde_json::{
    json,
    Value,
};
use tracing::{
    info,
    warn,
};

pub(crate) async fn websocket_connection(
    ws_url: String,
) -> Result<Arc<RootProvider<PubSubFrontend, AnyNetwork>>> {
    let ws = WsConnect::new(ws_url);
    info!("Connecting to WebSocket provider...");

    Ok(Arc::new(
        ProviderBuilder::new()
            .network::<AnyNetwork>()
            .on_ws(ws)
            .await
            .context("Failed to connect to provider")?,
    ))
}

pub(crate) async fn http_connection(
    http_url: String,
) -> Result<Arc<RpcClient<Http<reqwest::Client>>>> {
    info!("Connecting to HTTP client...");

    Ok(Arc::new(ClientBuilder::default().http(
        http_url.parse().context("Failed to parse HTTP URL")?,
    )))
}

/// Retry configuration
#[derive(Clone, Debug)]
pub(crate) struct RetryConfig {
    pub max_attempts: u32,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
    pub backoff_multiplier: f64,
}

impl RetryConfig {
    pub(crate) fn new(
        max_attempts: u32,
        initial_backoff: u64,
        max_backoff: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            max_attempts,
            initial_backoff: Duration::from_millis(initial_backoff),
            max_backoff: Duration::from_millis(max_backoff),
            backoff_multiplier,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(10),
            backoff_multiplier: 2.0,
        }
    }
}

/// Determine if an error should trigger a retry
fn should_retry(error: &Error) -> bool {
    match error {
        // TODO: Add more specific error handling
        _ => true,
    }
}

/// Retry a future with exponential backoff
pub(crate) async fn retry_with_backoff<F, Fut, T>(operation: F, config: &RetryConfig) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempts = 0;
    let mut backoff = config.initial_backoff;

    loop {
        attempts += 1;
        match operation().await {
            Ok(value) => return Ok(value),
            Err(error) => {
                if !should_retry(&error) || attempts >= config.max_attempts {
                    return Err(error);
                }

                warn!(
                    "Request failed (attempt {}/{}), retrying in {:?}: {:?}",
                    attempts, config.max_attempts, backoff, error
                );

                tokio::time::sleep(backoff).await;

                // Calculate next backoff duration
                backoff = Duration::from_secs_f64(
                    (backoff.as_secs_f64() * config.backoff_multiplier)
                        .min(config.max_backoff.as_secs_f64()),
                );
            }
        }
    }
}

/// Fetch block from provider
pub(crate) async fn fetch_block_data_batched(
    client: &Arc<RpcClient<Http<reqwest::Client>>>,
    block_number: u64,
    retry_config: &RetryConfig,
) -> Result<(
    Vec<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>>,
    <AnyNetwork as Network>::BlockResponse,
)> {
    // Execute the batch request
    let (receipts, block) = retry_with_backoff(
        || async {
            // Execute the batch request
            let mut batch_requests = client.new_batch();
            let block_call = batch_requests.add_call(
                "eth_getBlockByNumber",
                &[json!(format!("0x{:x}", block_number)), json!(false)],
            )?;
            let receipts_call = batch_requests.add_call(
                "eth_getBlockReceipts",
                &[Value::String(format!("0x{:x}", block_number))],
            )?;
            batch_requests.await?;

            // TODO figure out if this is the correct way to handle the errors in the batch request
            match (receipts_call.await, block_call.await) {
                (Ok(receipts), Ok(block)) => {
                    return Ok((receipts, block));
                }
                (Err(reciept_err), Ok(_)) => {
                    warn!(
                        "failed to grab receipts for block {}: {}",
                        block_number, reciept_err
                    );
                    bail!(
                        "failed to grab receipts for block {}: {}",
                        block_number,
                        reciept_err
                    );
                }
                (Ok(_), Err(block_err)) => {
                    warn!(
                        "failed to grab block for block {}: {}",
                        block_number, block_err
                    );
                    bail!(
                        "failed to grab block for block {}: {}",
                        block_number,
                        block_err
                    );
                }
                (Err(reciept_err), Err(block_err)) => {
                    warn!(
                        "failed to grab receipts and block for block {}: {}, {}",
                        block_number, reciept_err, block_err
                    );
                    bail!(
                        "failed to grab receipts and block for block {}: {}, {}",
                        block_number,
                        reciept_err,
                        block_err
                    );
                }
            }
        },
        retry_config,
    )
    .await?;

    Ok((receipts, block))
}
