library exchange_abi;

use std::{
    contract_id::ContractId,
    identity::Identity,
    u256::U256,
};

// Events

pub struct LiquidityAdded {
    sender: b256,
    amount_0: u64,
    amount_1: u64,
    lp_tokens: u64,
    recipient: b256,
}

pub struct LiquidityRemoved {
    sender: b256,
    amount_0: u64,
    amount_1: u64,
    lp_tokens: u64,
    recipient: b256,
}

pub struct Swap {
    sender: b256,
    amount_0_in: u64,
    amount_1_in: u64,
    amount_0_out: u64,
    amount_1_out: u64,
    recipient: b256,
}

pub struct UpdateReserves {
    amount_0: u64,
    amount_1: u64,
}

pub struct ProtocolFeeCollected {
    sender: b256,
    amount_0: u64,
    amount_1: u64,
}

pub struct ProtocolFeeWithdrawn {
    amount_0: u64,
    amount_1: u64,
}


// Returns

pub struct RemoveLiquidityInfo {
    token_0_amount: u64,
    token_1_amount: u64,
}

pub struct PoolInfo {
    token_0_reserve: u64,
    token_1_reserve: u64,
    lp_token_supply: u64,
    twap_buffer_size: u64,
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

pub struct FeeInfo {
    current_fee: u16,
    change_rate: u16,
    update_time: u32,
}

pub struct Observation {
    timestamp: u64,
    price_0_cumulative_last: U256,
    price_1_cumulative_last: U256,
}

abi Exchange {
    ////////////////////
    // Read only
    ////////////////////
    /// Get information on the liquidity pool.
    #[storage(read)]fn get_pool_info() -> PoolInfo;
    #[storage(read)]fn get_vault_info() -> VaultInfo;
    #[storage(read)]fn get_fee_info() -> FeeInfo;
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
    /// Increase the size of the TWAP buffer to the given size
    #[storage(read, write)]fn expand_twap_buffer(new_slots: u64);
}
