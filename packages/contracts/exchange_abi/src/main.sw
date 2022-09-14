library exchange_abi;

use std::{
    contract_id::ContractId,
    identity::Identity,
};

pub struct RemoveLiquidityInfo {
    token_0_amount: u64,
    token_1_amount: u64,
}

pub struct PositionInfo {
    token_0_amount: u64,
    token_1_amount: u64,
}

pub struct PoolInfo {
    token_0_reserve: u64,
    token_1_reserve: u64,
    lp_token_supply: u64,
}

pub struct PreviewInfo {
    amount: u64,
    has_liquidity: bool,
}

abi Exchange {
    ////////////////////
    // Read only
    ////////////////////
    /// Get information on the liquidity pool.
    #[storage(read)]fn get_pool_info() -> PoolInfo;
    /// Get information on the liquidity pool.
    #[storage(read)]fn get_add_liquidity_token_amount(token_0_amount: u64) -> u64;
    ////////////////////
    // Actions
    ////////////////////
    /// Deposit ETH and Tokens at current ratio to mint SWAYSWAP tokens.
    #[storage(read, write)]fn add_liquidity(min_liquidity: u64, recipient: Identity) -> u64;
    /// Burn SWAYSWAP tokens to withdraw ETH and Tokens at current ratio.
    #[storage(read, write)]fn remove_liquidity(min_token_0: u64, min_token_1: u64, recipient: Identity) -> RemoveLiquidityInfo;
    /// Swap ETH <-> Tokens and tranfers to sender.
    #[storage(read, write)]fn swap_with_minimum(asset_id: b256, min: u64, recipient: Identity) -> u64;
    /// Swap ETH <-> Tokens and tranfers to sender.
    #[storage(read, write)]fn swap_with_maximum(asset_id: b256, amount: u64, recipient: Identity) -> u64;
    #[storage(read, write)]fn swap(asset_id: b256, recipient: Identity) -> u64;
    /// Get the minimum amount of coins that will be received for a swap_with_minimum.
    #[storage(read)]fn get_swap_with_minimum(amount: u64) -> PreviewInfo;
    /// Get required amount of coins for a swap_with_maximum.
    #[storage(read)]fn get_swap_with_maximum(amount: u64) -> PreviewInfo;
}
