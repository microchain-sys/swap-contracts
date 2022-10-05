contract;

use std::{
    address::*,
    assert::assert,
    block::*,
    chain::auth::*,
    context::{*, call_frames::*},
    contract_id::ContractId,
    hash::*,
    identity::Identity,
    math::*,
    result::*,
    revert::revert,
    storage::*,
    token::*,
    u128::U128,
};

use exchange_abi::{Exchange, PoolInfo, PositionInfo, PreviewInfo, RemoveLiquidityInfo};
use microchain_helpers::{
    get_msg_sender_address_or_panic,
    get_input_price,
    get_output_price,
    mutiply_div,
};

////////////////////////////////////////
// Constants
////////////////////////////////////////

const TOKEN_0_SLOT = 0x0000000000000000000000000000000000000000000000000000000000000000;
const TOKEN_1_SLOT = 0x0000000000000000000000000000000000000000000000000000000000000001;

/// Minimum ETH liquidity to open a pool.
const MINIMUM_LIQUIDITY = 1000;

////////////////////////////////////////
// Storage declarations
////////////////////////////////////////

storage {
    token0_reserve: u64 = 0,
    token1_reserve: u64 = 0,
    lp_token_supply: u64 = 0,
}

////////////////////////////////////////
// Helper functions
////////////////////////////////////////

#[storage(read)]fn get_input_amount(asset_id: b256, token_0_reserve: u64, token_1_reserve: u64) -> u64 {
    let (token0, token1) = get_tokens();
    let mut amount = 0;
    if (asset_id == token0) {
        amount = this_balance(~ContractId::from(token0)) - token_0_reserve;
    } else if (asset_id == token1) {
        amount = this_balance(~ContractId::from(token1)) - token_1_reserve;
    } else {
        revert(0);
    }
    amount
}

#[storage(read, write)]fn store_reserves(reserve0: u64, reserve1: u64) {
    storage.token0_reserve = reserve0;
    storage.token1_reserve = reserve1;
}

#[storage(read)]fn get_tokens() -> (b256, b256) {
    (
        get::<b256>(TOKEN_0_SLOT),
        get::<b256>(TOKEN_1_SLOT),
    )
}

// ////////////////////////////////////////
// // ABI definitions
// ////////////////////////////////////////
impl Exchange for Contract {
    #[storage(read)]fn get_pool_info() -> PoolInfo {
        PoolInfo {
            token_0_reserve: storage.token0_reserve,
            token_1_reserve: storage.token1_reserve,
            lp_token_supply: storage.lp_token_supply,
        }
    }

    #[storage(read)]fn get_add_liquidity_token_amount(token_0_amount: u64) -> u64 {
        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let token_1_amount = mutiply_div(token_0_amount, token_1_reserve, token_0_reserve);
        token_1_amount
    }

    #[storage(read, write)]fn add_liquidity(min_liquidity: u64, recipient: Identity) -> u64 {
        let (token0, token1) = get_tokens();
        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = storage.lp_token_supply;

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let current_token_0_amount = this_balance(~ContractId::from(token0)) - token_0_reserve;
        let current_token_1_amount = this_balance(~ContractId::from(token1)) - token_1_reserve;

        assert(current_token_0_amount > 0);
        assert(current_token_1_amount > 0);

        let mut minted: u64 = 0;
        if total_liquidity > 0 {
            assert(min_liquidity > 0);
            let token_1_amount = mutiply_div(current_token_0_amount, token_1_reserve, token_0_reserve);
            let liquidity_minted = mutiply_div(current_token_0_amount, total_liquidity, token_0_reserve);

            assert(liquidity_minted >= min_liquidity);

            // if token ratio is correct, proceed with liquidity operation
            // otherwise, return current user balances in contract
            if (current_token_1_amount >= token_1_amount) {
                // Add fund to the reserves
                store_reserves(token_0_reserve + current_token_0_amount, token_1_reserve + token_1_amount);

                // Mint LP token
                mint(liquidity_minted);
                storage.lp_token_supply = total_liquidity + liquidity_minted;

                transfer(liquidity_minted, contract_id(), recipient);

                // If user sent more than the correct ratio, we deposit back the extra tokens
                let token_extra = current_token_1_amount - token_1_amount;
                if (token_extra > 0) {
                    transfer(token_extra, ~ContractId::from(token1), recipient);
                }

                minted = liquidity_minted;
            } else {
                transfer(current_token_1_amount, ~ContractId::from(token1), recipient);
                transfer(current_token_0_amount, ~ContractId::from(token0), recipient);
                minted = 0;
            }
        } else {
            let initial_liquidity = (current_token_0_amount * current_token_1_amount).sqrt() - MINIMUM_LIQUIDITY;

            // Add fund to the reserves
            store_reserves(current_token_0_amount, current_token_1_amount);

            // Mint LP token
            mint(initial_liquidity);
            storage.lp_token_supply = initial_liquidity + MINIMUM_LIQUIDITY;

            transfer(initial_liquidity, contract_id(), recipient);

            minted = initial_liquidity;
        };

        minted
    }

    #[storage(read, write)]fn remove_liquidity(min_token_0: u64, min_token_1: u64, recipient: Identity) -> RemoveLiquidityInfo {
        let (token0, token1) = get_tokens();
        assert(msg_amount() > 0);
        assert(msg_asset_id().into() == (contract_id()).into());
        assert(min_token_0 > 0 && min_token_1 > 0);

        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = storage.lp_token_supply;
        assert(total_liquidity > 0);

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let token_0_amount = mutiply_div(msg_amount(), token_0_reserve, total_liquidity);
        let token_1_amount = mutiply_div(msg_amount(), token_1_reserve, total_liquidity);

        assert((token_0_amount >= min_token_0) && (token_1_amount >= min_token_1));

        burn(msg_amount());
        storage.lp_token_supply = total_liquidity - msg_amount();

        // Remove funds to the reserves
        store_reserves(token_0_reserve - token_0_amount, token_1_reserve - token_1_amount);

        // Send tokens back
        transfer(token_0_amount, ~ContractId::from(token0), recipient);
        transfer(token_1_amount, ~ContractId::from(token1), recipient);

        RemoveLiquidityInfo {
            token_0_amount: token_0_amount,
            token_1_amount: token_1_amount,
        }
    }

    #[storage(read, write)]fn swap_with_minimum(asset_id: b256, min: u64, recipient: Identity) -> u64 {
        let (token0, token1) = get_tokens();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let input_amount = get_input_amount(asset_id, token_0_reserve, token_1_reserve);
        assert(input_amount > 0 && min > 0);

        let mut bought = 0;
        if (asset_id == token0) {
            bought = get_input_price(input_amount, token_0_reserve, token_1_reserve);
            assert(bought >= min);
            transfer(bought, ~ContractId::from(token1), recipient);
            // Update reserve
            store_reserves(token_0_reserve + input_amount, token_1_reserve - bought);
        } else {
            bought = get_input_price(input_amount, token_1_reserve, token_0_reserve);
            assert(bought >= min);
            transfer(bought, ~ContractId::from(token0), recipient);
            // Update reserve
            store_reserves(token_0_reserve - bought, token_1_reserve + bought);
        };
        bought
    }

    #[storage(read, write)]fn swap_with_maximum(asset_id: b256, amount: u64, recipient: Identity) -> u64 {
        let (token0, token1) = get_tokens();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let input_amount = get_input_amount(asset_id, token_0_reserve, token_1_reserve);
        assert(amount > 0 && input_amount > 0);

        let mut sold = 0;
        if (asset_id == token0) {
            sold = get_output_price(amount, token_0_reserve, token_1_reserve);
            assert(input_amount >= sold);
            let refund = input_amount - sold;
            if refund > 0 {
                transfer(refund, ~ContractId::from(token0), recipient);
            };
            transfer(amount, ~ContractId::from(token1), recipient);
            // Update reserve
            store_reserves(token_0_reserve + sold, token_1_reserve - amount);
        } else {
            sold = get_output_price(amount, token_1_reserve, token_0_reserve);
            assert(input_amount >= sold);
            let refund = input_amount - sold;
            if refund > 0 {
                transfer(refund, ~ContractId::from(token1), recipient);
            };
            transfer(amount, ~ContractId::from(token0), recipient);
            // Update reserve
            store_reserves(token_0_reserve - amount, token_1_reserve + sold);
        };
        sold
    }

    #[storage(read, write)]fn swap(asset_id: b256, recipient: Identity) -> u64 {
        let (token0, token1) = get_tokens();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let input_amount = get_input_amount(asset_id, token_0_reserve, token_1_reserve);
        assert(input_amount > 0);

        let mut bought = 0;
        if (asset_id == token0) {
            bought = get_input_price(input_amount, token_0_reserve, token_1_reserve);
            transfer(bought, ~ContractId::from(token1), recipient);
            // Update reserve
            store_reserves(token_0_reserve + input_amount, token_1_reserve - bought);
        } else {
            bought = get_input_price(input_amount, token_1_reserve, token_0_reserve);
            transfer(bought, ~ContractId::from(token0), recipient);
            // Update reserve
            store_reserves(token_0_reserve - bought, token_1_reserve + bought);
        };
        bought
    }

    #[storage(read)]fn get_tokens() -> (b256, b256) {
        get_tokens()
    }

    #[storage(read)]fn get_swap_with_minimum(amount: u64) -> PreviewInfo {
        let (token0,) = get_tokens();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let mut sold = 0;
        let mut has_liquidity = true;
        if (msg_asset_id().into() == token0) {
            sold = get_input_price(amount, token_0_reserve, token_1_reserve);
            has_liquidity = sold < token_1_reserve;
        } else {
            sold = get_input_price(amount, token_1_reserve, token_0_reserve);
            has_liquidity = sold < token_0_reserve;
        }
        PreviewInfo {
            amount: sold,
            has_liquidity: has_liquidity,
        }
    }

    #[storage(read)]fn get_swap_with_maximum(amount: u64) -> PreviewInfo {
        let (token0, token1) = get_tokens();
        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let mut sold = 0;
        let mut has_liquidity = true;
        if (msg_asset_id().into() == token0) {
            sold = get_output_price(amount, token_0_reserve, token_1_reserve);
            has_liquidity = sold < token_0_reserve;
        } else {
            sold = get_output_price(amount, token_1_reserve, token_0_reserve);
            has_liquidity = sold < token_1_reserve;
        }
        PreviewInfo {
            amount: sold,
            has_liquidity: has_liquidity,
        }
    }
}
