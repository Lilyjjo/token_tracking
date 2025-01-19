use diesel::{
    pg::PgConnection,
    prelude::*,
    result::Error,
};
use eyre::Result;

use crate::pool_sql::types::*;

impl BlockRaw {
    pub fn find_by_number(number: i64, conn: &mut PgConnection) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::blocks::dsl::*;
        blocks
            .filter(block_number.eq(number))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<Self, Error> {
        use crate::pool_sql::schema::blocks::dsl::*;

        if let Some(existing_block) = Self::find_by_number(self.block_number, conn)? {
            return Ok(existing_block);
        }

        diesel::insert_into(blocks).values(&self).execute(conn)?;

        Ok(self)
    }
}

impl TransactionRaw {
    pub fn find_by_hash(hash: &[u8], conn: &mut PgConnection) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::transactions::dsl::*;

        transactions
            .filter(transaction_hash.eq(hash))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<Self, Error> {
        use crate::pool_sql::schema::transactions::dsl::*;

        // Check if transaction already exists
        if let Some(existing_tx) = Self::find_by_hash(&self.transaction_hash, conn)? {
            return Ok(existing_tx);
        }

        // Insert if it doesn't exist
        diesel::insert_into(transactions)
            .values(&self)
            .execute(conn)?;

        Ok(self)
    }
}

impl SwapEventRaw {
    pub fn find_by_tx_and_log(
        tx_hash: &[u8],
        log_idx: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::swap_events::dsl::*;

        swap_events
            .filter(transaction_hash.eq(tx_hash))
            .filter(log_index.eq(log_idx))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<(), Error> {
        use crate::pool_sql::schema::swap_events::dsl::*;

        // Check if swap event already exists
        if let Some(_) = Self::find_by_tx_and_log(&self.transaction_hash, self.log_index, conn)? {
            return Ok(());
        }

        // Insert if it doesn't exist
        diesel::insert_into(swap_events)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl InitializationEventRaw {
    pub fn find_by_tx_and_log(
        tx_hash: &[u8],
        log_idx: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::initialization_events::dsl::*;

        initialization_events
            .filter(transaction_hash.eq(tx_hash))
            .filter(log_index.eq(log_idx))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<(), Error> {
        use crate::pool_sql::schema::initialization_events::dsl::*;

        // Check if initialization event already exists
        if let Some(_) = Self::find_by_tx_and_log(&self.transaction_hash, self.log_index, conn)? {
            return Ok(());
        }

        diesel::insert_into(initialization_events)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl MintEventRaw {
    pub fn find_by_tx_and_log(
        tx_hash: &[u8],
        log_idx: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::mint_events::dsl::*;

        mint_events
            .filter(transaction_hash.eq(tx_hash))
            .filter(log_index.eq(log_idx))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<(), Error> {
        use crate::pool_sql::schema::mint_events::dsl::*;

        // Check if mint event already exists
        if let Some(_) = Self::find_by_tx_and_log(&self.transaction_hash, self.log_index, conn)? {
            return Ok(());
        }

        diesel::insert_into(mint_events)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl BurnEventRaw {
    pub fn find_by_tx_and_log(
        tx_hash: &[u8],
        log_idx: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::burn_events::dsl::*;

        burn_events
            .filter(transaction_hash.eq(tx_hash))
            .filter(log_index.eq(log_idx))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<(), Error> {
        use crate::pool_sql::schema::burn_events::dsl::*;

        // Check if burn event already exists
        if let Some(_) = Self::find_by_tx_and_log(&self.transaction_hash, self.log_index, conn)? {
            return Ok(());
        }

        diesel::insert_into(burn_events)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

impl CollectEventRaw {
    pub fn find_by_tx_and_log(
        tx_hash: &[u8],
        log_idx: i64,
        conn: &mut PgConnection,
    ) -> Result<Option<Self>, Error> {
        use crate::pool_sql::schema::collect_events::dsl::*;

        collect_events
            .filter(transaction_hash.eq(tx_hash))
            .filter(log_index.eq(log_idx))
            .first(conn)
            .optional()
    }

    pub fn insert_if_not_exists(self, conn: &mut PgConnection) -> Result<(), Error> {
        use crate::pool_sql::schema::collect_events::dsl::*;

        // Check if collect event already exists
        if let Some(_) = Self::find_by_tx_and_log(&self.transaction_hash, self.log_index, conn)? {
            return Ok(());
        }

        diesel::insert_into(collect_events)
            .values(self)
            .execute(conn)?;

        Ok(())
    }
}

// Function to insert a transaction and multiple swap events
pub(crate) fn insert_block_events(
    block: BlockRaw,
    transactions: Vec<TransactionRaw>,
    swaps: Vec<SwapEventRaw>,
    initialize_events: Vec<InitializationEventRaw>,
    mint_events: Vec<MintEventRaw>,
    burn_events: Vec<BurnEventRaw>,
    collect_events: Vec<CollectEventRaw>,
    conn: &mut PgConnection,
) -> Result<()> {
    conn.transaction(|conn| {
        block.insert_if_not_exists(conn)?;

        // First ensure the transactions exist
        for transaction in transactions {
            transaction.insert_if_not_exists(conn)?;
        }

        // Then insert all swap events
        for swap in swaps {
            swap.insert_if_not_exists(conn)?;
        }

        // Then insert all initialize events
        for initialize in initialize_events {
            initialize.insert_if_not_exists(conn)?;
        }

        // Then insert all mint events
        for mint in mint_events {
            mint.insert_if_not_exists(conn)?;
        }

        // Then insert all burn events
        for burn in burn_events {
            burn.insert_if_not_exists(conn)?;
        }

        // Then insert all collect events
        for collect in collect_events {
            collect.insert_if_not_exists(conn)?;
        }

        Ok(())
    })
}
