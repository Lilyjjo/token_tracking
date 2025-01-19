use std::{
    convert::TryFrom,
    str::FromStr,
};

use alloy::{
    primitives::{
        aliases::{
            I24,
            I256,
            U128,
            U160,
            U24,
            U256,
        },
        Address,
        Log as AbiLog,
        TxHash,
    },
    rpc::types::Log,
};
use bigdecimal::BigDecimal;
use diesel::{
    prelude::*,
    Insertable,
};
use eyre::{
    ContextCompat,
    Result,
};

use crate::{
    abi::{
        IUniswapV3Factory::PoolCreated,
        UniswapV3Pool::{
            Burn,
            Collect,
            Initialize,
            Mint,
            Swap,
        },
    },
    pool_sql::schema::*,
};

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = blocks)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct BlockRaw {
    pub block_number: i64,
    pub block_timestamp: i64,
}

#[derive(Debug)]
pub(crate) struct Block {
    pub block_number: u64,
    pub block_timestamp: u64,
}

impl TryFrom<BlockRaw> for Block {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: BlockRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            block_number: raw.block_number as u64,
            block_timestamp: raw.block_timestamp as u64,
        })
    }
}

impl TryFrom<Block> for BlockRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(block: Block) -> Result<Self, Self::Error> {
        Ok(Self {
            block_number: block.block_number as i64,
            block_timestamp: block.block_timestamp as i64,
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct TransactionRaw {
    pub transaction_hash: Vec<u8>,
    pub block_number: i64,
    pub transaction_index: i64,
    pub transaction_sender: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct Transaction {
    pub transaction_hash: TxHash,
    pub block_number: u64,
    pub transaction_index: u64,
    pub transaction_sender: Address,
}

impl TryFrom<TransactionRaw> for Transaction {
    type Error = &'static str;

    fn try_from(raw: TransactionRaw) -> Result<Self, Self::Error> {
        // Check for negative values
        if raw.block_number < 0 || raw.transaction_index < 0 {
            return Err("Negative values cannot be converted to unsigned integers");
        }

        // Convert Vec<u8> to TxHash
        let transaction_hash = TxHash::try_from(raw.transaction_hash.as_slice())
            .map_err(|_| "Failed to convert transaction hash")?;

        // Convert Vec<u8> to Address
        let transaction_sender = Address::try_from(raw.transaction_sender.as_slice())
            .map_err(|_| "Failed to convert sender address")?;

        Ok(Self {
            transaction_hash,
            block_number: raw.block_number as u64,
            transaction_index: raw.transaction_index as u64,
            transaction_sender,
        })
    }
}

impl TryFrom<Transaction> for TransactionRaw {
    type Error = &'static str;

    fn try_from(tx: Transaction) -> Result<Self, Self::Error> {
        // Check if u64 values fit within i64 range
        if tx.block_number > i64::MAX as u64 || tx.transaction_index > i64::MAX as u64 {
            return Err("Value too large for i64");
        }

        Ok(Self {
            transaction_hash: tx.transaction_hash.to_vec(),
            block_number: tx.block_number as i64,
            transaction_index: tx.transaction_index as i64,
            transaction_sender: tx.transaction_sender.to_vec(),
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = pool_create_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct PoolCreateEventRaw {
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    pub token0: Vec<u8>,
    pub token1: Vec<u8>,
    pub fee: BigDecimal,
    pub tick_spacing: BigDecimal,
    pub pool: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct PoolCreateEvent {
    pub transaction_hash: TxHash,
    pub log_index: u64,
    pub token0: Address,
    pub token1: Address,
    pub fee: U24,
    pub tick_spacing: I24,
    pub pool: Address,
}

impl TryFrom<PoolCreateEventRaw> for PoolCreateEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: PoolCreateEventRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: TxHash::try_from(raw.transaction_hash.as_slice())?,
            log_index: raw.log_index as u64,
            token0: Address::try_from(raw.token0.as_slice())?,
            token1: Address::try_from(raw.token1.as_slice())?,
            fee: U24::from_str(&raw.fee.to_string())?,
            tick_spacing: I24::from_str(&raw.tick_spacing.to_string())?,
            pool: Address::try_from(raw.pool.as_slice())?,
        })
    }
}

impl TryFrom<PoolCreateEvent> for PoolCreateEventRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(event: PoolCreateEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: event.transaction_hash.to_vec(),
            log_index: event.log_index as i64,
            token0: event.token0.to_vec(),
            token1: event.token1.to_vec(),
            fee: BigDecimal::from_str(&event.fee.to_string())?,
            tick_spacing: BigDecimal::from_str(&event.tick_spacing.to_string())?,
            pool: event.pool.to_vec(),
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = swap_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct SwapEventRaw {
    #[diesel(serialize_as = Vec<u8>)]
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub sender: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub recipient: Vec<u8>,
    pub amount0: BigDecimal,
    pub amount1: BigDecimal,
    pub sqrt_price_x96: BigDecimal,
    pub liquidity: BigDecimal,
    pub tick: BigDecimal,
}

#[derive(Debug)]
pub(crate) struct SwapEvent {
    pub transaction_hash: TxHash,
    pub log_index: u64,
    pub contract_address: Address,
    pub sender: Address,
    pub recipient: Address,
    pub amount0: I256,
    pub amount1: I256,
    pub sqrt_price_x96: U160,
    pub liquidity: U128,
    pub tick: I24,
}

impl TryFrom<SwapEventRaw> for SwapEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: SwapEventRaw) -> Result<Self, Self::Error> {
        // Check for negative log_index
        if raw.log_index < 0 {
            return Err("Negative log_index cannot be converted to unsigned integer".into());
        }

        // Convert transaction_hash Vec<u8> to TxHash
        let transaction_hash = TxHash::try_from(raw.transaction_hash.as_slice())
            .map_err(|e| format!("Failed to convert transaction hash: {}", e))?;

        // Convert addresses
        let contract_address = Address::try_from(raw.contract_address.as_slice())
            .map_err(|e| format!("Failed to convert contract address: {}", e))?;
        let sender = Address::try_from(raw.sender.as_slice())
            .map_err(|e| format!("Failed to convert sender address: {}", e))?;
        let recipient = Address::try_from(raw.recipient.as_slice())
            .map_err(|e| format!("Failed to convert recipient address: {}", e))?;

        // Convert BigDecimal to specific numeric types
        let amount0 = I256::from_dec_str(&raw.amount0.to_string())
            .map_err(|e| format!("Failed to convert amount0: {}", e))?;
        let amount1 = I256::from_dec_str(&raw.amount1.to_string())
            .map_err(|e| format!("Failed to convert amount1: {}", e))?;

        let sqrt_price_x96 = U160::from_str(&raw.sqrt_price_x96.to_string())
            .map_err(|e| format!("Failed to convert sqrt_price_x96: {}", e))?;

        let liquidity = U128::from_str(&raw.liquidity.to_string())
            .map_err(|e| format!("Failed to convert liquidity: {}", e))?;

        let tick = I24::from_dec_str(&raw.tick.to_string())
            .map_err(|e| format!("Failed to convert tick: {}", e))?;

        Ok(Self {
            transaction_hash,
            log_index: raw.log_index as u64,
            contract_address,
            sender,
            recipient,
            amount0,
            amount1,
            sqrt_price_x96,
            liquidity,
            tick,
        })
    }
}

impl TryFrom<SwapEvent> for SwapEventRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(event: SwapEvent) -> Result<Self, Self::Error> {
        // Check if u64 value fits within i64 range
        if event.log_index > i64::MAX as u64 {
            return Err("log_index too large for i64".into());
        }

        Ok(Self {
            transaction_hash: event.transaction_hash.to_vec(),
            log_index: event.log_index as i64,
            contract_address: event.contract_address.to_vec(),
            sender: event.sender.to_vec(),
            recipient: event.recipient.to_vec(),
            amount0: BigDecimal::from_str(&event.amount0.to_string())
                .map_err(|e| format!("Failed to convert amount0: {}", e))?,
            amount1: BigDecimal::from_str(&event.amount1.to_string())
                .map_err(|e| format!("Failed to convert amount1: {}", e))?,
            sqrt_price_x96: BigDecimal::from_str(&event.sqrt_price_x96.to_string())
                .map_err(|e| format!("Failed to convert sqrt_price_x96: {}", e))?,
            liquidity: BigDecimal::from_str(&event.liquidity.to_string())
                .map_err(|e| format!("Failed to convert liquidity: {}", e))?,
            tick: BigDecimal::from_str(&event.tick.to_string())
                .map_err(|e| format!("Failed to convert tick: {}", e))?,
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = initialization_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct InitializationEventRaw {
    #[diesel(serialize_as = Vec<u8>)]
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub creator: Vec<u8>,
    pub sqrt_price_x96: BigDecimal,
    pub tick: BigDecimal,
}

#[derive(Debug)]
pub(crate) struct InitializationEvent {
    pub transaction_hash: TxHash,
    pub log_index: u64,
    pub contract_address: Address,
    pub creator: Address,
    pub sqrt_price_x96: U160,
    pub tick: I24,
}

impl TryFrom<InitializationEventRaw> for InitializationEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: InitializationEventRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: TxHash::try_from(raw.transaction_hash.as_slice())?,
            log_index: raw.log_index as u64,
            contract_address: Address::try_from(raw.contract_address.as_slice())?,
            creator: Address::try_from(raw.creator.as_slice())?,
            sqrt_price_x96: U160::from_str(&raw.sqrt_price_x96.to_string())?,
            tick: I24::from_dec_str(&raw.tick.to_string())?,
        })
    }
}

impl TryFrom<InitializationEvent> for InitializationEventRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(event: InitializationEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: event.transaction_hash.to_vec(),
            log_index: event.log_index as i64,
            contract_address: event.contract_address.to_vec(),
            creator: event.creator.to_vec(),
            sqrt_price_x96: BigDecimal::from_str(&event.sqrt_price_x96.to_string())?,
            tick: BigDecimal::from_str(&event.tick.to_string())?,
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = mint_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct MintEventRaw {
    #[diesel(serialize_as = Vec<u8>)]
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub sender: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub owner: Vec<u8>,
    pub tick_lower: BigDecimal,
    pub tick_upper: BigDecimal,
    pub amount: BigDecimal,
    pub amount0: BigDecimal,
    pub amount1: BigDecimal,
}

#[derive(Debug)]
pub(crate) struct MintEvent {
    pub transaction_hash: TxHash,
    pub log_index: u64,
    pub contract_address: Address,
    pub sender: Address,
    pub owner: Address,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub amount: U128,
    pub amount0: U256,
    pub amount1: U256,
}

impl TryFrom<MintEventRaw> for MintEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: MintEventRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: TxHash::try_from(raw.transaction_hash.as_slice())?,
            log_index: raw.log_index as u64,
            contract_address: Address::try_from(raw.contract_address.as_slice())?,
            sender: Address::try_from(raw.sender.as_slice())?,
            owner: Address::try_from(raw.owner.as_slice())?,
            tick_lower: I24::from_dec_str(&raw.tick_lower.to_string())?,
            tick_upper: I24::from_dec_str(&raw.tick_upper.to_string())?,
            amount: U128::from_str(&raw.amount.to_string())?,
            amount0: U256::from_str(&raw.amount0.to_string())?,
            amount1: U256::from_str(&raw.amount1.to_string())?,
        })
    }
}

impl TryFrom<MintEvent> for MintEventRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(event: MintEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: event.transaction_hash.to_vec(),
            log_index: event.log_index as i64,
            contract_address: event.contract_address.to_vec(),
            sender: event.sender.to_vec(),
            owner: event.owner.to_vec(),
            tick_lower: BigDecimal::from_str(&event.tick_lower.to_string())?,
            tick_upper: BigDecimal::from_str(&event.tick_upper.to_string())?,
            amount: BigDecimal::from_str(&event.amount.to_string())?,
            amount0: BigDecimal::from_str(&event.amount0.to_string())?,
            amount1: BigDecimal::from_str(&event.amount1.to_string())?,
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = burn_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct BurnEventRaw {
    #[diesel(serialize_as = Vec<u8>)]
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub owner: Vec<u8>,
    pub tick_lower: BigDecimal,
    pub tick_upper: BigDecimal,
    pub amount: BigDecimal,
    pub amount0: BigDecimal,
    pub amount1: BigDecimal,
}

#[derive(Debug)]
pub(crate) struct BurnEvent {
    pub transaction_hash: TxHash,
    pub log_index: u64,
    pub contract_address: Address,
    pub owner: Address,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub amount: U128,
    pub amount0: U256,
    pub amount1: U256,
}

impl TryFrom<BurnEventRaw> for BurnEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: BurnEventRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: TxHash::try_from(raw.transaction_hash.as_slice())?,
            log_index: raw.log_index as u64,
            contract_address: Address::try_from(raw.contract_address.as_slice())?,
            owner: Address::try_from(raw.owner.as_slice())?,
            tick_lower: I24::from_dec_str(&raw.tick_lower.to_string())?,
            tick_upper: I24::from_dec_str(&raw.tick_upper.to_string())?,
            amount: U128::from_str(&raw.amount.to_string())?,
            amount0: U256::from_str(&raw.amount0.to_string())?,
            amount1: U256::from_str(&raw.amount1.to_string())?,
        })
    }
}

impl TryFrom<BurnEvent> for BurnEventRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(event: BurnEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: event.transaction_hash.to_vec(),
            log_index: event.log_index as i64,
            contract_address: event.contract_address.to_vec(),
            owner: event.owner.to_vec(),
            tick_lower: BigDecimal::from_str(&event.tick_lower.to_string())?,
            tick_upper: BigDecimal::from_str(&event.tick_upper.to_string())?,
            amount: BigDecimal::from_str(&event.amount.to_string())?,
            amount0: BigDecimal::from_str(&event.amount0.to_string())?,
            amount1: BigDecimal::from_str(&event.amount1.to_string())?,
        })
    }
}

#[derive(Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = collect_events)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[derive(Clone)]
pub(crate) struct CollectEventRaw {
    #[diesel(serialize_as = Vec<u8>)]
    pub transaction_hash: Vec<u8>,
    pub log_index: i64,
    #[diesel(serialize_as = Vec<u8>)]
    pub contract_address: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub owner: Vec<u8>,
    #[diesel(serialize_as = Vec<u8>)]
    pub recipient: Vec<u8>,
    pub tick_lower: BigDecimal,
    pub tick_upper: BigDecimal,
    pub amount0: BigDecimal,
    pub amount1: BigDecimal,
}

#[derive(Debug)]
pub(crate) struct CollectEvent {
    pub transaction_hash: TxHash,
    pub log_index: u64,
    pub contract_address: Address,
    pub owner: Address,
    pub recipient: Address,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub amount0: U256,
    pub amount1: U256,
}

impl TryFrom<CollectEventRaw> for CollectEvent {
    type Error = Box<dyn std::error::Error>;

    fn try_from(raw: CollectEventRaw) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: TxHash::try_from(raw.transaction_hash.as_slice())?,
            log_index: raw.log_index as u64,
            contract_address: Address::try_from(raw.contract_address.as_slice())?,
            owner: Address::try_from(raw.owner.as_slice())?,
            recipient: Address::try_from(raw.recipient.as_slice())?,
            tick_lower: I24::from_dec_str(&raw.tick_lower.to_string())?,
            tick_upper: I24::from_dec_str(&raw.tick_upper.to_string())?,
            amount0: U256::from_str(&raw.amount0.to_string())?,
            amount1: U256::from_str(&raw.amount1.to_string())?,
        })
    }
}

impl TryFrom<CollectEvent> for CollectEventRaw {
    type Error = Box<dyn std::error::Error>;

    fn try_from(event: CollectEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            transaction_hash: event.transaction_hash.to_vec(),
            log_index: event.log_index as i64,
            contract_address: event.contract_address.to_vec(),
            owner: event.owner.to_vec(),
            recipient: event.recipient.to_vec(),
            tick_lower: BigDecimal::from_str(&event.tick_lower.to_string())?,
            tick_upper: BigDecimal::from_str(&event.tick_upper.to_string())?,
            amount0: BigDecimal::from_str(&event.amount0.to_string())?,
            amount1: BigDecimal::from_str(&event.amount1.to_string())?,
        })
    }
}

impl Block {
    pub(crate) fn new(block_number: u64, block_timestamp: u64) -> Self {
        Self {
            block_number,
            block_timestamp,
        }
    }
}

impl Transaction {
    pub(crate) fn new(sender: Address, log: Log) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            block_number: log.block_number.wrap_err("block_number is missing")?,
            transaction_index: log
                .transaction_index
                .wrap_err("transaction_index is missing")?,
            transaction_sender: sender,
        })
    }
}

impl PoolCreateEvent {
    pub(crate) fn new(log: Log, pool_create_event: AbiLog<PoolCreated>) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            log_index: log.log_index.wrap_err("log_index is missing")?,
            token0: pool_create_event.token0,
            token1: pool_create_event.token1,
            fee: pool_create_event.fee,
            tick_spacing: pool_create_event.tickSpacing,
            pool: pool_create_event.pool,
        })
    }
}

impl SwapEvent {
    pub(crate) fn new(log: Log, swap_event: AbiLog<Swap>) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            log_index: log.log_index.wrap_err("log_index is missing")?,
            contract_address: swap_event.address,
            sender: swap_event.sender,
            recipient: swap_event.recipient,
            amount0: swap_event.amount0,
            amount1: swap_event.amount1,
            sqrt_price_x96: swap_event.sqrtPriceX96,
            liquidity: U128::from(swap_event.liquidity),
            tick: swap_event.tick,
        })
    }
}

impl InitializationEvent {
    pub(crate) fn new(
        log: Log,
        initialization_event: AbiLog<Initialize>,
        creator: Address,
    ) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            log_index: log.log_index.wrap_err("log_index is missing")?,
            contract_address: initialization_event.address,
            creator,
            sqrt_price_x96: initialization_event.sqrtPriceX96,
            tick: initialization_event.tick,
        })
    }
}

impl MintEvent {
    pub(crate) fn new(log: Log, mint_event: AbiLog<Mint>) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            log_index: log.log_index.wrap_err("log_index is missing")?,
            contract_address: mint_event.address,
            sender: mint_event.sender,
            owner: mint_event.owner,
            tick_lower: mint_event.tickLower,
            tick_upper: mint_event.tickUpper,
            amount: U128::from(mint_event.amount),
            amount0: mint_event.amount0,
            amount1: mint_event.amount1,
        })
    }
}

impl BurnEvent {
    pub(crate) fn new(log: Log, burn_event: AbiLog<Burn>) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            log_index: log.log_index.wrap_err("log_index is missing")?,
            contract_address: burn_event.address,
            owner: burn_event.owner,
            tick_lower: burn_event.tickLower,
            tick_upper: burn_event.tickUpper,
            amount: U128::from(burn_event.amount),
            amount0: burn_event.amount0,
            amount1: burn_event.amount1,
        })
    }
}

impl CollectEvent {
    pub(crate) fn new(log: Log, collect_event: AbiLog<Collect>) -> Result<Self> {
        Ok(Self {
            transaction_hash: log
                .transaction_hash
                .wrap_err("transaction_hash is missing")?,
            log_index: log.log_index.wrap_err("log_index is missing")?,
            contract_address: collect_event.address,
            owner: collect_event.owner,
            recipient: collect_event.recipient,
            tick_lower: collect_event.tickLower,
            tick_upper: collect_event.tickUpper,
            amount0: U256::from_str(&collect_event.amount0.to_string())?,
            amount1: U256::from_str(&collect_event.amount1.to_string())?,
        })
    }
}
