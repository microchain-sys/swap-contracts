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

pub struct VaultInfo {
    vault: b256,
    token0_protocol_fees_collected: u64,
    token1_protocol_fees_collected: u64,
    current_fee: u16,
    change_rate: u16,
    update_time: u32,
}

// Packed into a single 8-byte slot
pub struct VaultFee {
    // These values should be divided by 1,000,000 to get the rate. So 10,000 = 1%
    stored_fee: u16, // 1 byte
    change_rate: u16, // 1 byte
    // TODO: directional changes
    // fee_increasing: bool, // 1 byte
    update_time: u32, // 4 bytes
}

abi Exchange {
    ////////////////////
    // Read only
    ////////////////////
    /// Get information on the liquidity pool.
    #[storage(read)]fn get_pool_info() -> PoolInfo;
    #[storage(read)]fn get_vault_info() -> VaultInfo;
    /// Get information on the liquidity pool.
    #[storage(read)]fn get_add_liquidity_token_amount(token_0_amount: u64) -> u64;
    /// Get the minimum amount of coins that will be received for a swap_with_minimum.
    #[storage(read)]fn get_swap_with_minimum(amount: u64) -> PreviewInfo;
    /// Get required amount of coins for a swap_with_maximum.
    #[storage(read)]fn get_swap_with_maximum(amount: u64) -> PreviewInfo;
    /// Get the two tokens held in the pool
    #[storage(read)]fn get_tokens() -> (b256, b256);
    ////////////////////
    // Actions
    ////////////////////
    #[storage(read, write)]fn initialize(new_vault: b256);
    #[storage(read, write)]fn cache_vault_fees();
    /// Deposit ETH and Tokens at current ratio to mint SWAYSWAP tokens.
    #[storage(read, write)]fn add_liquidity(recipient: Identity) -> u64;
    /// Burn SWAYSWAP tokens to withdraw ETH and Tokens at current ratio.
    #[storage(read, write)]fn remove_liquidity( recipient: Identity) -> RemoveLiquidityInfo;
    #[storage(read, write)]fn swap(amount_0_out: u64, amount_1_out: u64, recipient: Identity);
    #[storage(read, write)]fn withdraw_protocol_fees(recipient: Identity) -> (u64, u64);
}
