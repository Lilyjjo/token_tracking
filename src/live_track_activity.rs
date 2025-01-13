use std::{
    sync::Arc,
    time::Duration,
};

// ABI
use alloy::primitives::Log as AbiLog;
use alloy::{
    consensus::TxReceipt,
    eips::{
        BlockId,
        BlockNumberOrTag,
    },
    network::{
        AnyNetwork,
        AnyReceiptEnvelope,
    },
    primitives::Address,
    providers::{
        Provider,
        ProviderBuilder,
        RootProvider,
        WsConnect,
    },
    pubsub::PubSubFrontend,
    rpc::types::{
        serde_helpers::WithOtherFields,
        Log,
        TransactionReceipt,
    },
    sol_types::SolEvent,
};
use eyre::{
    bail,
    Result,
    WrapErr,
};
use futures_util::StreamExt;
use tracing::{
    debug,
    error,
    info,
};
use IERC20Minimal::Transfer;
use UniswapV3Pool::{
    Burn,
    Initialize,
    Mint,
    Swap,
};

use crate::abi::*;

pub(crate) async fn websocket_connection(
    rpc_url: String,
) -> Result<Arc<RootProvider<PubSubFrontend, AnyNetwork>>> {
    let ws = WsConnect::new(rpc_url);
    info!("Connecting to WebSocket provider...");
    Ok(Arc::new(
        ProviderBuilder::new()
            .network::<AnyNetwork>()
            .on_ws(ws)
            .await
            .context("Failed to connect to provider")?,
    ))
}

/// Fetch block from provider
pub(crate) async fn fetch_block_receipts(
    provider: &Arc<RootProvider<PubSubFrontend, AnyNetwork>>,
    block_number: u64,
) -> Result<Vec<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>>> {
    // Get block receipts
    let mut retry_count: i32 = 3;
    while retry_count > 0 {
        match provider
            .get_block_receipts(BlockId::Number(BlockNumberOrTag::Number(
                block_number.into(),
            )))
            .await
        {
            Err(e) => {
                retry_count = retry_count.saturating_sub(1);
                debug!(
                    "Failed to grab receipts for block {}: {}, retrying: {}",
                    block_number,
                    e,
                    retry_count > 0
                );
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Ok(result) => {
                if let Some(receipts) = result {
                    return Ok(receipts);
                } else {
                    retry_count = retry_count.saturating_sub(1);
                    debug!(
                        "Receipts empty for block {}, retrying: {}",
                        block_number,
                        retry_count > 0
                    );
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    }

    bail!("Failed to grab receipts for block: {}", block_number);
}

pub(crate) async fn process_single_block(
    rpc_url: String,
    block_number: u64,
    token: Address,
    pool: Address,
) -> Result<()> {
    let provider = websocket_connection(rpc_url).await?;

    let receipts = match fetch_block_receipts(&provider, block_number).await {
        Ok(receipts) => {
            debug!(
                "Successfully grabbed receipts for block {}, receipts: {}",
                block_number,
                receipts.len()
            );
            receipts
        }
        Err(e) => {
            bail!("Failed to grab receipts for block {}: {}", block_number, e);
        }
    };

    // Process block for desired info
    match get_token_activity(token, pool, block_number, receipts).await {
        Ok(_) => {}
        Err(e) => {
            bail!(
                "Failed to process block's token activity {}: {}",
                block_number,
                e
            );
        }
    }

    Ok(())
}

pub(crate) async fn live_process_blocks(
    rpc_url: String,
    token: Address,
    pool: Address,
) -> Result<()> {
    let provider = websocket_connection(rpc_url).await?;

    info!("Connected to provider, subscribing to blocks...");
    let mut block_stream = provider
        .subscribe_blocks()
        .await
        .context("Failed to subscribe to blocks")?
        .into_stream();

    info!("Successfully subscribed to block stream");

    while let Some(block) = block_stream.next().await {
        let block_number = block.number;
        // Grab the block receipts
        let receipts = match fetch_block_receipts(&provider, block_number).await {
            Ok(receipts) => {
                debug!(
                    "Successfully grabbed receipts for block {}, receipts: {}",
                    block_number,
                    receipts.len()
                );
                receipts
            }
            Err(e) => {
                error!(
                    "Failed to grab receipts for block due to {}: {}",
                    block_number, e
                );
                continue;
            }
        };

        // Process block for desired info
        match get_token_activity(token, pool, block_number, receipts).await {
            Ok(_) => {}
            Err(e) => {
                error!(
                    "Failed to process block's token activity {}: {}",
                    block_number, e
                );
                continue;
            }
        }
    }

    Ok(())
}

async fn get_token_activity(
    token: Address,
    pool: Address,
    block_number: u64,
    block_receipts: Vec<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>>,
) -> Result<()> {
    // Filter receipts that interact with target contract
    let filtered_receipts: Vec<_> = block_receipts
        .into_iter()
        .filter(
            |receipt: &WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>| {
                // Check if the transaction is to our target contract
                if receipt.inner.to == Some(token) || receipt.inner.to == Some(pool) {
                    return true;
                }

                // Check if any logs are from our target contract
                receipt
                    .inner
                    .inner
                    .inner
                    .logs()
                    .iter()
                    .any(|log| log.address() == pool || log.address() == token)
            },
        )
        .collect();

    // Process the receipts to search for transfers from the
    for tx in filtered_receipts {
        let tx_hash = tx.inner.transaction_hash;
        let mut transfer_logs = Vec::<AbiLog<Transfer>>::new();
        let mut initialize_logs = Vec::<AbiLog<Initialize>>::new();
        let mut mint_logs = Vec::<AbiLog<Mint>>::new();
        let mut burn_logs = Vec::<AbiLog<Burn>>::new();
        let mut swap_logs = Vec::<AbiLog<Swap>>::new();
        for log in tx.inner.inner.inner.logs() {
            if log.inner.topics().is_empty()
                || !(log.inner.topics()[0] != Transfer::SIGNATURE_HASH
                    || log.inner.topics()[0] != Initialize::SIGNATURE_HASH
                    || log.inner.topics()[0] != Mint::SIGNATURE_HASH
                    || log.inner.topics()[0] != Burn::SIGNATURE_HASH
                    || log.inner.topics()[0] != Swap::SIGNATURE_HASH)
            {
                continue;
            }

            // create log object
            if let Some(abi_log) = AbiLog::new(
                log.address(),
                log.topics().to_vec(),
                log.data().data.clone(),
            ) {
                match log.inner.topics()[0] {
                    Transfer::SIGNATURE_HASH => {
                        if let Ok(transfer_log) = Transfer::decode_log(&abi_log, true) {
                            // have transfer log value
                            if log.address() != token {
                                continue;
                            }
                            transfer_logs.push(transfer_log.into());
                        }
                    }
                    Initialize::SIGNATURE_HASH => {
                        if let Ok(initialize_log) = Initialize::decode_log(&abi_log, true) {
                            // have initialize log value
                            if log.address() != pool {
                                continue;
                            }
                            initialize_logs.push(initialize_log.into());
                        }
                    }
                    Mint::SIGNATURE_HASH => {
                        if let Ok(mint_log) = Mint::decode_log(&abi_log, true) {
                            // have mint log value
                            if log.address() != pool {
                                continue;
                            }
                            mint_logs.push(mint_log.into());
                        }
                    }
                    Burn::SIGNATURE_HASH => {
                        if let Ok(burn_log) = Burn::decode_log(&abi_log, true) {
                            // have burn log value
                            if log.address() != pool {
                                continue;
                            }
                            burn_logs.push(burn_log.into());
                        }
                    }
                    Swap::SIGNATURE_HASH => {
                        if let Ok(burn_log) = Swap::decode_log(&abi_log, true) {
                            // have burn log value
                            if log.address() != pool {
                                continue;
                            }
                            swap_logs.push(burn_log.into());
                        }
                    }
                    _ => {}
                }
            }
        }
        if !transfer_logs.is_empty()
            || !initialize_logs.is_empty()
            || !mint_logs.is_empty()
            || !burn_logs.is_empty()
            || !swap_logs.is_empty()
        {
            info!(
                "Found relevant logs for tx: {:?} in block {}",
                tx_hash, block_number
            );
            if !transfer_logs.is_empty() {
                info!("number of transfers: {}", transfer_logs.len());
                info!("number of swap logs: {}", swap_logs.len());
            }
            for initialize_log in initialize_logs {
                info!("initialize_log: {:?}", initialize_log);
            }
            for mint_log in mint_logs {
                info!("mint_log: {:?}", mint_log);
            }
            for burn_log in burn_logs {
                info!("burn_log: {:?}", burn_log);
            }
            for swap_log in swap_logs {
                info!("swap_log: {:?}", swap_log);
            }
        }
    }

    Ok(())
}
