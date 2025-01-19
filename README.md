# Uniswap V3 Pool Data Grabber 

This is a tool to grab all event data from target Uniswap V3 pools and store it in a Postgres database for later analysis. The code pulls data from an RPC node on a block basis and then stores it in a SQL format in the database. You can see the schema in the `src/schema.rs` file.

## Requirements
This tool uses [diesel](https://github.com/diesel-rs/diesel) to interact with a postgres database. These are the instructions I used to install postgres and diesel on my mac.
```
brew install postgresql@17
brew services start postgresql@17
cargo install diesel_cli
```

## Setup
1. Copy the `local.env.example` file to `.env` and set the environment variables (can use `just copy-env` to do this).
2. Set up a new Postgres database and create a database user with the appropriate permissions and put the access parameters in the `.env` file.
This is how I did my local setup:
```bash
# create a new empty database, put location in .env
createdb my_example_database

# check that the database is created 
psql -l
```
3. Setup the database schema with diesel and configure the tables. This will create the necessary schema file at `src/pool_sql/schema.rs`.
```bash
diesel migration run
```

## Usage

### For live processing of new blocks
```bash
just live_blocks
```

### For processing a single block
```bash
just single_block 24985835
```

### For processing a range of blocks
```bash
just blocks_from 24985835 24985846
```
Note: If your RPC is slow, you can add a delay between blocks by setting the `BLOCK_FROM_RPC_DELAY` environment variable.

### To toggle log level (default is info)
```bash
just live debug
```