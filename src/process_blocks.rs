use std::{
    collections::{
        HashMap,
        HashSet,
    },
    time::Duration,
};

use alloy::{
    consensus::TxReceipt,
    network::{
        AnyNetwork,
        AnyReceiptEnvelope,
        Network,
    },
    primitives::{
        Address,
        Log as AbiLog,
        TxHash,
    },
    providers::Provider,
    rpc::types::{
        serde_helpers::WithOtherFields,
        Log,
        TransactionReceipt,
    },
    sol_types::SolEvent,
};
use diesel::PgConnection;
use eyre::{
    bail,
    Result,
    WrapErr,
};
use futures_util::StreamExt;
use tracing::{
    debug,
    info,
};
use UniswapV3Pool::{
    Burn,
    Collect,
    Initialize,
    Mint,
    Swap,
};

use crate::{
    abi::{
        IUniswapV3Factory::PoolCreated,
        UniswapV3Pool,
    },
    pool_sql::{
        database_interactions::{
            establish_connection,
            insert_block_events,
        },
        types::{
            Block,
            BurnEvent,
            CollectEvent,
            InitializationEvent,
            MintEvent,
            PoolCreateEvent,
            SwapEvent,
            Transaction,
        },
    },
    rpc::{
        fetch_block_data_batched,
        http_connection,
        websocket_connection,
        RetryConfig,
    },
};

pub(crate) async fn single_block(
    http_url: String,
    block_number: u64,
    uniswap_v3_factory_address: Address,
    pool_deployer_addresses: &HashSet<Address>,
    pools: &mut HashSet<Address>,
    retry_config: RetryConfig,
) -> Result<()> {
    let client = http_connection(http_url)
        .await
        .wrap_err("failed to build http")?;

    // fetch block data
    let (receipts, block) =
        match fetch_block_data_batched(&client, block_number, &retry_config).await {
            Ok((receipts, block)) => {
                debug!(
                    "Successfully grabbed receipts for block {}, receipts length: {}",
                    block_number,
                    receipts.len()
                );
                (receipts, block)
            }
            Err(e) => {
                bail!("Failed to grab data for block {}: {}", block_number, e);
            }
        };

    // process block for desired events
    match get_and_store_events(
        pool_deployer_addresses,
        pools,
        uniswap_v3_factory_address,
        receipts,
        block,
    )
    .await
    {
        Ok(_) => {}
        Err(e) => {
            bail!(
                "Failed to process block's position activity {}: {}",
                block_number,
                e
            );
        }
    }

    Ok(())
}

pub(crate) async fn blocks_from(
    http_url: String,
    start_block: u64,
    end_block: u64,
    uniswap_v3_factory_address: Address,
    pool_deployer_addresses: &HashSet<Address>,
    pools: &mut HashSet<Address>,
    retry_config: RetryConfig,
    delay_ms: u64,
) -> Result<()> {
    if start_block > end_block {
        bail!("Start block must be less than end block");
    }

    let client = http_connection(http_url)
        .await
        .wrap_err("failed to build http")?;

    info!(
        "Processing blocks from {} to {} ({} blocks)",
        start_block,
        end_block,
        end_block.saturating_sub(start_block)
    );

    for block_number in start_block..end_block {
        // fetch block data
        let (receipts, block) =
            match fetch_block_data_batched(&client, block_number, &retry_config).await {
                Ok((receipts, block)) => {
                    debug!(
                        "Successfully grabbed receipts for block {}, receipts length: {}",
                        block_number,
                        receipts.len()
                    );
                    (receipts, block)
                }
                Err(e) => {
                    bail!("Failed to grab data for block {}: {}", block_number, e);
                }
            };

        // process block for desired events
        match get_and_store_events(
            pool_deployer_addresses,
            pools,
            uniswap_v3_factory_address,
            receipts,
            block,
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                bail!(
                    "Failed to process block's position activity {}: {}",
                    block_number,
                    e
                );
            }
        }
        if delay_ms > 0 {
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }
    }
    info!(
        "Successfully processed blocks from {} to {}",
        start_block, end_block
    );
    Ok(())
}

pub(crate) async fn live_blocks(
    http_url: String,
    wss_url: String,
    uniswap_v3_factory_address: Address,
    pool_deployer_addresses: &HashSet<Address>,
    pools: &mut HashSet<Address>,
    retry_config: RetryConfig,
) -> Result<()> {
    let client = http_connection(http_url)
        .await
        .wrap_err("failed to build http")?;

    let provider = websocket_connection(wss_url).await?;

    info!("Connected to provider, subscribing to blocks...");
    let mut block_stream = provider
        .subscribe_blocks()
        .await
        .context("Failed to subscribe to blocks")?
        .into_stream();

    info!("Successfully subscribed to block stream");

    while let Some(block) = block_stream.next().await {
        let block_number = block.number;
        // fetch block data
        let (receipts, block) =
            match fetch_block_data_batched(&client, block_number, &retry_config).await {
                Ok((receipts, block)) => {
                    debug!(
                        "Successfully grabbed receipts for block {}, receipts length: {}",
                        block_number,
                        receipts.len()
                    );
                    (receipts, block)
                }
                Err(e) => {
                    bail!("Failed to grab data for block {}: {}", block_number, e);
                }
            };

        // process block for desired events
        match get_and_store_events(
            pool_deployer_addresses,
            pools,
            uniswap_v3_factory_address,
            receipts,
            block,
        )
        .await
        {
            Ok(_) => {}
            Err(e) => {
                bail!(
                    "Failed to process block's position activity {}: {}",
                    block_number,
                    e
                );
            }
        }
    }

    Ok(())
}

// TODO: refactor this to be more modular
async fn get_and_store_events(
    pool_deployer_addresses: &HashSet<Address>,
    pools: &mut HashSet<Address>,
    uniswap_v3_factory_address: Address,
    block_receipts: Vec<WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>>,
    block: <AnyNetwork as Network>::BlockResponse,
) -> Result<()> {
    // Filter receipts that interact with target pool contracts
    let filtered_receipts: Vec<_> = block_receipts
        .into_iter()
        .filter(
            |receipt: &WithOtherFields<TransactionReceipt<AnyReceiptEnvelope<Log>>>| {
                // Check if any logs are from our target contract
                receipt.inner.inner.inner.logs().iter().any(|log| {
                    pools.contains(&log.address()) || log.address() == uniswap_v3_factory_address
                })
            },
        )
        .collect();

    let block = Block::new(block.inner.header.number, block.inner.header.timestamp);
    let mut transactions = HashMap::<TxHash, Transaction>::new();
    let mut pool_create_events = Vec::<PoolCreateEvent>::new();
    let mut swaps = Vec::<SwapEvent>::new();
    let mut initialize_events = Vec::<InitializationEvent>::new();
    let mut mint_events = Vec::<MintEvent>::new();
    let mut burn_events = Vec::<BurnEvent>::new();
    let mut collect_events = Vec::<CollectEvent>::new();
    // Process the receipts to search for transfers from the
    for tx in filtered_receipts {
        for log in tx.inner.inner.inner.logs() {
            if log.inner.topics().is_empty()
                || (log.inner.topics()[0] != Swap::SIGNATURE_HASH
                    && log.inner.topics()[0] != Mint::SIGNATURE_HASH
                    && log.inner.topics()[0] != Burn::SIGNATURE_HASH
                    && log.inner.topics()[0] != Collect::SIGNATURE_HASH
                    && log.inner.topics()[0] != Initialize::SIGNATURE_HASH
                    && log.inner.topics()[0] != PoolCreated::SIGNATURE_HASH)
            {
                continue;
            }

            // create log object and processes events
            if let Some(abi_log) = AbiLog::new(
                log.address(),
                log.topics().to_vec(),
                log.data().data.clone(),
            ) {
                match log.inner.topics()[0] {
                    PoolCreated::SIGNATURE_HASH => {
                        if let Ok(pool_create_event) = PoolCreated::decode_log(&abi_log, true) {
                            if log.address() != uniswap_v3_factory_address {
                                // event not from target factory
                                continue;
                            }
                            if let Some(pool_address) = tx.inner.to {
                                if !pool_deployer_addresses.contains(&pool_address) {
                                    // pool not from target deployers
                                    continue;
                                }
                            } else {
                                // pool not from target deployers
                                continue;
                            }

                            // build transaction data struct if not already in map
                            transactions.entry(tx.inner.transaction_hash).or_insert({
                                let transaction_data = Transaction::new(tx.inner.from, log.clone());
                                if let Ok(transaction_data) = transaction_data {
                                    transaction_data
                                } else {
                                    bail!("Failed to create transaction data from: {:?}", log);
                                }
                            });

                            debug!("pool_create_event: {:?}", pool_create_event);
                            let pool_create_event =
                                PoolCreateEvent::new(log.clone(), pool_create_event);

                            if let Ok(pool_create_event) = pool_create_event {
                                // track pool in pools set
                                pools.insert(pool_create_event.pool);

                                // add to pool create events
                                pool_create_events.push(pool_create_event);
                            } else {
                                bail!("Failed to create pool create event from: {:?}", log);
                            }
                        }
                    }
                    Initialize::SIGNATURE_HASH => {
                        if let Ok(initialize_event) = Initialize::decode_log(&abi_log, true) {
                            if !pools.contains(&log.address()) {
                                continue;
                            }
                            debug!("initialize_event: {:?}", initialize_event);

                            // build transaction data struct if not already in map
                            transactions.entry(tx.inner.transaction_hash).or_insert({
                                let transaction_data = Transaction::new(tx.inner.from, log.clone());
                                if let Ok(transaction_data) = transaction_data {
                                    transaction_data
                                } else {
                                    bail!("Failed to create transaction data from: {:?}", log);
                                }
                            });

                            // build initialization event
                            let initialize_event = InitializationEvent::new(
                                log.clone(),
                                initialize_event,
                                tx.inner.from,
                            );
                            if let Ok(initialize_event) = initialize_event {
                                initialize_events.push(initialize_event);
                            } else {
                                bail!("Failed to create initialize event from: {:?}", log);
                            }
                        }
                    }
                    Swap::SIGNATURE_HASH => {
                        if let Ok(swap_event) = Swap::decode_log(&abi_log, true) {
                            if !pools.contains(&log.address()) {
                                continue;
                            }
                            debug!("swap_event: {:?}", swap_event);
                            // build transaction data struct if not already in map
                            transactions.entry(tx.inner.transaction_hash).or_insert({
                                let transaction_data = Transaction::new(tx.inner.from, log.clone());
                                if let Ok(transaction_data) = transaction_data {
                                    transaction_data
                                } else {
                                    bail!("Failed to create transaction data from: {:?}", log);
                                }
                            });

                            // build swap event
                            let swap_event = SwapEvent::new(log.clone(), swap_event);
                            if let Ok(swap_event) = swap_event {
                                swaps.push(swap_event);
                            } else {
                                bail!("Failed to create swap event from: {:?}", log);
                            }
                        }
                    }
                    Mint::SIGNATURE_HASH => {
                        if let Ok(mint_event) = Mint::decode_log(&abi_log, true) {
                            if !pools.contains(&log.address()) {
                                continue;
                            }
                            debug!("mint_event: {:?}", mint_event);

                            // build transaction data struct if not already in map
                            transactions.entry(tx.inner.transaction_hash).or_insert({
                                let transaction_data = Transaction::new(tx.inner.from, log.clone());
                                if let Ok(transaction_data) = transaction_data {
                                    transaction_data
                                } else {
                                    bail!("Failed to create transaction data from: {:?}", log);
                                }
                            });

                            // build mint event
                            let mint_event = MintEvent::new(log.clone(), mint_event);
                            if let Ok(mint_event) = mint_event {
                                mint_events.push(mint_event);
                            } else {
                                bail!("Failed to create mint event from: {:?}", log);
                            }
                        }
                    }
                    Burn::SIGNATURE_HASH => {
                        if let Ok(burn_event) = Burn::decode_log(&abi_log, true) {
                            if !pools.contains(&log.address()) {
                                continue;
                            }
                            debug!("burn_event: {:?}", burn_event);
                            // build transaction data struct if not already in map
                            transactions.entry(tx.inner.transaction_hash).or_insert({
                                let transaction_data = Transaction::new(tx.inner.from, log.clone());
                                if let Ok(transaction_data) = transaction_data {
                                    transaction_data
                                } else {
                                    bail!("Failed to create transaction data from: {:?}", log);
                                }
                            });

                            // build burn event
                            let burn_event = BurnEvent::new(log.clone(), burn_event);
                            if let Ok(burn_event) = burn_event {
                                burn_events.push(burn_event);
                            } else {
                                bail!("Failed to create burn event from: {:?}", log);
                            }
                        }
                    }
                    Collect::SIGNATURE_HASH => {
                        if let Ok(collect_event) = Collect::decode_log(&abi_log, true) {
                            if !pools.contains(&log.address()) {
                                continue;
                            }
                            debug!("collect_event: {:?}", collect_event);

                            // build transaction data struct if not already in map
                            transactions.entry(tx.inner.transaction_hash).or_insert({
                                let transaction_data = Transaction::new(tx.inner.from, log.clone());
                                if let Ok(transaction_data) = transaction_data {
                                    transaction_data
                                } else {
                                    bail!("Failed to create transaction data from: {:?}", log);
                                }
                            });

                            // build collect event
                            let collect_event = CollectEvent::new(log.clone(), collect_event);
                            if let Ok(collect_event) = collect_event {
                                collect_events.push(collect_event);
                            } else {
                                bail!("Failed to create collect event from: {:?}", log);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    let mut db_connection = establish_connection()?;

    // insert events into db if swaps exist
    if !swaps.is_empty()
        || !initialize_events.is_empty()
        || !mint_events.is_empty()
        || !burn_events.is_empty()
        || !collect_events.is_empty()
    {
        info!(
            "Found in block {}:\n  pool_create_events: {}\n  swaps: {}\n  mint_events: {}\n  \
             burn_events: {}\n  collect_events: {}\n  initialize_events: {}",
            block.block_number,
            pool_create_events.len(),
            swaps.len(),
            mint_events.len(),
            burn_events.len(),
            collect_events.len(),
            initialize_events.len()
        );
        let result = put_events_into_db(
            block,
            transactions,
            pool_create_events,
            swaps,
            initialize_events,
            mint_events,
            burn_events,
            collect_events,
            &mut db_connection,
        );
        if result.is_err() {
            bail!(
                "Failed to put swap events into db: {}",
                result.err().unwrap()
            );
        }
    } else {
        info!("No events found in block {}", block.block_number);
    }

    Ok(())
}

fn put_events_into_db(
    block: Block,
    transactions: HashMap<TxHash, Transaction>,
    pool_create_events: Vec<PoolCreateEvent>,
    swap_events: Vec<SwapEvent>,
    initialize_events: Vec<InitializationEvent>,
    mint_events: Vec<MintEvent>,
    burn_events: Vec<BurnEvent>,
    collect_events: Vec<CollectEvent>,
    db_connection: &mut PgConnection,
) -> Result<()> {
    // convert swapevents to swapeventraw
    let block_raw = block.try_into().unwrap();
    let pool_create_events_raw = pool_create_events
        .into_iter()
        .map(|pool_create_event| pool_create_event.try_into().unwrap())
        .collect();
    let swap_events_raw = swap_events
        .into_iter()
        .map(|swap_event| swap_event.try_into().unwrap())
        .collect();
    let transactions_raw = transactions
        .into_iter()
        .map(|(_, transaction)| transaction.try_into().unwrap())
        .collect();
    let initialize_events_raw = initialize_events
        .into_iter()
        .map(|initialize_event| initialize_event.try_into().unwrap())
        .collect();
    let mint_events_raw = mint_events
        .into_iter()
        .map(|mint_event| mint_event.try_into().unwrap())
        .collect();
    let burn_events_raw = burn_events
        .into_iter()
        .map(|burn_event| burn_event.try_into().unwrap())
        .collect();
    let collect_events_raw = collect_events
        .into_iter()
        .map(|collect_event| collect_event.try_into().unwrap())
        .collect();
    insert_block_events(
        block_raw,
        transactions_raw,
        pool_create_events_raw,
        swap_events_raw,
        initialize_events_raw,
        mint_events_raw,
        burn_events_raw,
        collect_events_raw,
        db_connection,
    )
}
