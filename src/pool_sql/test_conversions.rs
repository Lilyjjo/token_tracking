use std::{
    convert::TryFrom,
    str::FromStr,
};

use alloy::primitives::{
    aliases::{
        I24,
        I256,
        U128,
        U160,
    },
    Address,
    TxHash,
};
use bigdecimal::BigDecimal;

use crate::pool_sql::types::*;

#[test]
fn test_raw_to_transaction_conversion() {
    let raw_tx = TransactionRaw {
        transaction_hash: vec![1; 32], // Assuming 32 bytes for hash
        block_number: 12345,
        transaction_index: 67890,
        transaction_sender: vec![2; 20], // Assuming 20 bytes for address
    };

    let tx: Transaction = raw_tx.try_into().unwrap();
    assert_eq!(tx.block_number, 12345);
    assert_eq!(tx.transaction_index, 67890);
}

#[test]
fn test_transaction_to_raw_conversion() {
    let tx = Transaction {
        transaction_hash: TxHash::try_from(vec![1; 32].as_slice()).unwrap(),
        block_number: 12345,
        transaction_index: 67890,
        transaction_sender: Address::try_from(vec![2; 20].as_slice()).unwrap(),
    };

    let raw_tx: TransactionRaw = tx.try_into().unwrap();
    assert_eq!(raw_tx.block_number, 12345);
    assert_eq!(raw_tx.transaction_index, 67890);
}

#[test]
fn test_invalid_raw_to_transaction() {
    let raw_tx = TransactionRaw {
        transaction_hash: vec![1; 31], // Invalid length for hash
        block_number: -1,              // Negative value
        transaction_index: 67890,
        transaction_sender: vec![2; 20],
    };

    let result: Result<Transaction, _> = raw_tx.try_into();
    assert!(result.is_err());
}

#[test]
fn test_invalid_transaction_to_raw() {
    let tx = Transaction {
        transaction_hash: TxHash::try_from(vec![1; 32].as_slice()).unwrap(),
        block_number: u64::MAX, // Too large for i64
        transaction_index: 67890,
        transaction_sender: Address::try_from(vec![2; 20].as_slice()).unwrap(),
    };

    let result: Result<TransactionRaw, _> = tx.try_into();
    assert!(result.is_err());
}

#[test]
fn test_raw_to_swap_event_conversion() {
    let raw_event = SwapEventRaw {
        transaction_hash: vec![1; 32],
        log_index: 12345,
        contract_address: vec![2; 20],
        sender: vec![3; 20],
        recipient: vec![4; 20],
        amount0: BigDecimal::from(100),
        amount1: BigDecimal::from(-100),
        sqrt_price_x96: BigDecimal::from(1000000),
        liquidity: BigDecimal::from(500000),
        tick: BigDecimal::from(-5),
    };

    let event: SwapEvent = raw_event.try_into().unwrap();
    assert_eq!(event.log_index, 12345);
}

#[test]
fn test_swap_event_to_raw_conversion() {
    let event = SwapEvent {
        transaction_hash: TxHash::try_from(vec![1; 32].as_slice()).unwrap(),
        log_index: 12345,
        contract_address: Address::try_from(vec![2; 20].as_slice()).unwrap(),
        sender: Address::try_from(vec![3; 20].as_slice()).unwrap(),
        recipient: Address::try_from(vec![4; 20].as_slice()).unwrap(),
        amount0: I256::from_str("100").unwrap(),
        amount1: I256::from_str("-100").unwrap(),
        sqrt_price_x96: U160::from(1000000u64),
        liquidity: U128::from(500000u64),
        tick: I24::from_str("-5").unwrap(),
    };

    let raw_event: SwapEventRaw = event.try_into().unwrap();
    assert_eq!(raw_event.log_index, 12345);
}

#[test]
fn test_invalid_raw_to_swap_event() {
    let raw_event = SwapEventRaw {
        transaction_hash: vec![1; 31], // Invalid length
        log_index: -1,                 // Negative value
        contract_address: vec![2; 20],
        sender: vec![3; 20],
        recipient: vec![4; 20],
        amount0: BigDecimal::from(100),
        amount1: BigDecimal::from(-100),
        sqrt_price_x96: BigDecimal::from(1000000),
        liquidity: BigDecimal::from(500000),
        tick: BigDecimal::from(-5),
    };

    let result: Result<SwapEvent, _> = raw_event.try_into();
    assert!(result.is_err());
}

#[test]
fn test_invalid_swap_event_to_raw() {
    let event = SwapEvent {
        transaction_hash: TxHash::try_from(vec![1; 32].as_slice()).unwrap(),
        log_index: u64::MAX, // Too large for i64
        contract_address: Address::try_from(vec![2; 20].as_slice()).unwrap(),
        sender: Address::try_from(vec![3; 20].as_slice()).unwrap(),
        recipient: Address::try_from(vec![4; 20].as_slice()).unwrap(),
        amount0: I256::from_str("100").unwrap(),
        amount1: I256::from_str("-100").unwrap(),
        sqrt_price_x96: U160::from(1000000u64),
        liquidity: U128::from(500000u64),
        tick: I24::from_str("-5").unwrap(),
    };

    let result: Result<SwapEventRaw, _> = event.try_into();
    assert!(result.is_err());
}
