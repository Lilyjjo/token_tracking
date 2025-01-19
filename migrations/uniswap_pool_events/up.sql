-- Your SQL goes here
CREATE TABLE blocks (
    block_number BIGINT PRIMARY KEY, -- u64
    block_timestamp BIGINT NOT NULL -- u64
);

CREATE TABLE transactions (
    transaction_hash BYTEA PRIMARY KEY,    -- TxHash
    block_number BIGINT NOT NULL REFERENCES blocks(block_number),
    transaction_index BIGINT NOT NULL,    -- u64
    transaction_sender BYTEA NOT NULL   -- Address
);

CREATE TABLE swap_events (
    transaction_hash BYTEA NOT NULL REFERENCES transactions(transaction_hash),
    log_index BIGINT NOT NULL,           -- u64
    contract_address BYTEA NOT NULL,       -- Address
    sender BYTEA NOT NULL,                 -- Address
    recipient BYTEA NOT NULL,              -- Address
    amount0 NUMERIC(78, 0) NOT NULL,             -- Signed<256, 4>
    amount1 NUMERIC(78, 0) NOT NULL,             -- Signed<256, 4>
    sqrt_price_x96 NUMERIC(78, 0) NOT NULL,      -- Uint<160, 3>
    liquidity NUMERIC(78, 0) NOT NULL,           -- u128
    tick NUMERIC(78, 0) NOT NULL,               -- Signed<24, 1>
    PRIMARY KEY(transaction_hash, log_index)
);

CREATE TABLE initialization_events (
   transaction_hash BYTEA NOT NULL REFERENCES transactions(transaction_hash),
   log_index BIGINT NOT NULL,
   contract_address BYTEA NOT NULL,
   creator BYTEA NOT NULL,
   sqrt_price_x96 NUMERIC(78, 0) NOT NULL,
   tick NUMERIC(78, 0) NOT NULL,
   PRIMARY KEY(transaction_hash, log_index)
);

CREATE TABLE mint_events (
   transaction_hash BYTEA NOT NULL REFERENCES transactions(transaction_hash),
   log_index BIGINT NOT NULL,
   contract_address BYTEA NOT NULL,
   sender BYTEA NOT NULL,
   owner BYTEA NOT NULL,
   tick_lower NUMERIC(78, 0) NOT NULL,
   tick_upper NUMERIC(78, 0) NOT NULL,
   amount NUMERIC(78, 0) NOT NULL,
   amount0 NUMERIC(78, 0) NOT NULL,
   amount1 NUMERIC(78, 0) NOT NULL,
   PRIMARY KEY(transaction_hash, log_index)
);

CREATE TABLE burn_events (
   transaction_hash BYTEA NOT NULL REFERENCES transactions(transaction_hash),
   log_index BIGINT NOT NULL,
   contract_address BYTEA NOT NULL,
   owner BYTEA NOT NULL,
   tick_lower NUMERIC(78, 0) NOT NULL,
   tick_upper NUMERIC(78, 0) NOT NULL,
   amount NUMERIC(78, 0) NOT NULL,
   amount0 NUMERIC(78, 0) NOT NULL,
   amount1 NUMERIC(78, 0) NOT NULL,
   PRIMARY KEY(transaction_hash, log_index)
);

CREATE TABLE collect_events (
   transaction_hash BYTEA NOT NULL REFERENCES transactions(transaction_hash),
   log_index BIGINT NOT NULL,
   contract_address BYTEA NOT NULL,
   owner BYTEA NOT NULL,
   recipient BYTEA NOT NULL,
   tick_lower NUMERIC(78, 0) NOT NULL,
   tick_upper NUMERIC(78, 0) NOT NULL,
   amount0 NUMERIC(78, 0) NOT NULL,
   amount1 NUMERIC(78, 0) NOT NULL,
   PRIMARY KEY(transaction_hash, log_index)
);

-- contract addresses
CREATE INDEX swap_events_contract_address_idx ON swap_events(contract_address);
CREATE INDEX mint_events_contract_address_idx ON mint_events(contract_address);
CREATE INDEX burn_events_contract_address_idx ON burn_events(contract_address);
CREATE INDEX collect_events_contract_address_idx ON collect_events(contract_address);

-- transaction and blocks
CREATE INDEX transactions_block_number_idx ON transactions(block_number);
CREATE INDEX blocks_timestamp_idx ON blocks(block_timestamp);

-- liquidity events
CREATE INDEX mint_events_owner_idx ON mint_events(owner);
CREATE INDEX burn_events_owner_idx ON burn_events(owner);
CREATE INDEX collect_events_owner_idx ON collect_events(owner);

-- cross-table indexes
CREATE INDEX swap_events_contract_time_idx ON swap_events(contract_address, transaction_hash, log_index);
CREATE INDEX mint_events_contract_time_idx ON mint_events(contract_address, transaction_hash, log_index);
CREATE INDEX burn_events_contract_time_idx ON burn_events(contract_address, transaction_hash, log_index);
CREATE INDEX collect_events_contract_time_idx ON collect_events(contract_address, transaction_hash, log_index);
