use alloy::sol;

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc, abi)]
    interface IUniswapV3Factory {
        /// @notice Emitted when a pool is created
        /// @param token0 The first token of the pool by address sort order
        /// @param token1 The second token of the pool by address sort order
        /// @param fee The fee collected upon every swap in the pool, denominated in hundredths of a bip
        /// @param tickSpacing The minimum number of ticks between initialized ticks
        /// @param pool The address of the created pool
        event PoolCreated(
            address indexed token0,
            address indexed token1,
            uint24 indexed fee,
            int24 tickSpacing,
            address pool
        );
    }
}

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[sol(rpc, abi)]
    interface UniswapV3Pool {
        /// @notice Emitted exactly once by a pool when #initialize is first called on the pool
        /// @dev Mint/Burn/Swap cannot be emitted by the pool before Initialize
        /// @param sqrtPriceX96 The initial sqrt price of the pool, as a Q64.96
        /// @param tick The initial tick of the pool, i.e. log base 1.0001 of the starting price of the pool
        event Initialize(uint160 sqrtPriceX96, int24 tick);

        /// @notice Emitted when liquidity is minted for a given position
        /// @param sender The address that minted the liquidity
        /// @param owner The owner of the position and recipient of any minted liquidity
        /// @param tickLower The lower tick of the position
        /// @param tickUpper The upper tick of the position
        /// @param amount The amount of liquidity minted to the position range
        /// @param amount0 How much token0 was required for the minted liquidity
        /// @param amount1 How much token1 was required for the minted liquidity
        event Mint(
            address sender,
            address indexed owner,
            int24 indexed tickLower,
            int24 indexed tickUpper,
            uint128 amount,
            uint256 amount0,
            uint256 amount1
        );

        /// @notice Emitted when fees are collected by the owner of a position
        /// @dev Collect events may be emitted with zero amount0 and amount1 when the caller chooses not to collect fees
        /// @param owner The owner of the position for which fees are collected
        /// @param tickLower The lower tick of the position
        /// @param tickUpper The upper tick of the position
        /// @param amount0 The amount of token0 fees collected
        /// @param amount1 The amount of token1 fees collected
        event Collect(
            address indexed owner,
            address recipient,
            int24 indexed tickLower,
            int24 indexed tickUpper,
            uint128 amount0,
            uint128 amount1
        );

        /// @notice Emitted when a position's liquidity is removed
        /// @dev Does not withdraw any fees earned by the liquidity position, which must be withdrawn via #collect
        /// @param owner The owner of the position for which liquidity is removed
        /// @param tickLower The lower tick of the position
        /// @param tickUpper The upper tick of the position
        /// @param amount The amount of liquidity to remove
        /// @param amount0 The amount of token0 withdrawn
        /// @param amount1 The amount of token1 withdrawn
        event Burn(
            address indexed owner,
            int24 indexed tickLower,
            int24 indexed tickUpper,
            uint128 amount,
            uint256 amount0,
            uint256 amount1
        );

        /// @notice Emitted by the pool for any swaps between token0 and token1
        /// @param sender The address that initiated the swap call, and that received the callback
        /// @param recipient The address that received the output of the swap
        /// @param amount0 The delta of the token0 balance of the pool
        /// @param amount1 The delta of the token1 balance of the pool
        /// @param sqrtPriceX96 The sqrt(price) of the pool after the swap, as a Q64.96
        /// @param liquidity The liquidity of the pool after the swap
        /// @param tick The log base 1.0001 of price of the pool after the swap
        event Swap(
            address indexed sender,
            address indexed recipient,
            int256 amount0,
            int256 amount1,
            uint160 sqrtPriceX96,
            uint128 liquidity,
            int24 tick
        );
    }
}
