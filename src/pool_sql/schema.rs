// @generated automatically by Diesel CLI.

diesel::table! {
    blocks (block_number) {
        block_number -> Int8,
        block_timestamp -> Int8,
    }
}

diesel::table! {
    burn_events (transaction_hash, log_index) {
        transaction_hash -> Bytea,
        log_index -> Int8,
        contract_address -> Bytea,
        owner -> Bytea,
        tick_lower -> Numeric,
        tick_upper -> Numeric,
        amount -> Numeric,
        amount0 -> Numeric,
        amount1 -> Numeric,
    }
}

diesel::table! {
    collect_events (transaction_hash, log_index) {
        transaction_hash -> Bytea,
        log_index -> Int8,
        contract_address -> Bytea,
        owner -> Bytea,
        recipient -> Bytea,
        tick_lower -> Numeric,
        tick_upper -> Numeric,
        amount0 -> Numeric,
        amount1 -> Numeric,
    }
}

diesel::table! {
    initialization_events (transaction_hash, log_index) {
        transaction_hash -> Bytea,
        log_index -> Int8,
        contract_address -> Bytea,
        creator -> Bytea,
        sqrt_price_x96 -> Numeric,
        tick -> Numeric,
    }
}

diesel::table! {
    mint_events (transaction_hash, log_index) {
        transaction_hash -> Bytea,
        log_index -> Int8,
        contract_address -> Bytea,
        sender -> Bytea,
        owner -> Bytea,
        tick_lower -> Numeric,
        tick_upper -> Numeric,
        amount -> Numeric,
        amount0 -> Numeric,
        amount1 -> Numeric,
    }
}

diesel::table! {
    swap_events (transaction_hash, log_index) {
        transaction_hash -> Bytea,
        log_index -> Int8,
        contract_address -> Bytea,
        sender -> Bytea,
        recipient -> Bytea,
        amount0 -> Numeric,
        amount1 -> Numeric,
        sqrt_price_x96 -> Numeric,
        liquidity -> Numeric,
        tick -> Numeric,
    }
}

diesel::table! {
    transactions (transaction_hash) {
        transaction_hash -> Bytea,
        block_number -> Int8,
        transaction_index -> Int8,
        transaction_sender -> Bytea,
    }
}

diesel::joinable!(burn_events -> transactions (transaction_hash));
diesel::joinable!(collect_events -> transactions (transaction_hash));
diesel::joinable!(initialization_events -> transactions (transaction_hash));
diesel::joinable!(mint_events -> transactions (transaction_hash));
diesel::joinable!(swap_events -> transactions (transaction_hash));
diesel::joinable!(transactions -> blocks (block_number));

diesel::allow_tables_to_appear_in_same_query!(
    blocks,
    burn_events,
    collect_events,
    initialization_events,
    mint_events,
    swap_events,
    transactions,
);
