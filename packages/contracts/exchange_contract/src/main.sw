contract;

use std::{
    address::*,
    assert::assert,
    block::*,
    chain::auth::*,
    context::{*, call_frames::*},
    contract_id::ContractId,
    hash::*,
    result::*,
    revert::revert,
    storage::*,
    token::*,
    u128::U128,
};

use exchange_abi::{Exchange, PoolInfo, PositionInfo, PreviewInfo, RemoveLiquidityInfo};
use swayswap_helpers::get_msg_sender_address_or_panic;

////////////////////////////////////////
// Constants
////////////////////////////////////////

const TOKEN_0 = 0x0000000000000000000000000000000000000000000000000000000000000000;

/// Modify at compile time for different pool.
const TOKEN_1 = 0xeb4f49ab76e1866ba27bdc9392373ad868d1efc88bbcd0eb67c8b066308a060d;

/// Minimum ETH liquidity to open a pool.
const MINIMUM_LIQUIDITY = 1; //A more realistic value would be 1000000000;
// Liquidity miner fee apply to all swaps
const LIQUIDITY_MINER_FEE = 333;

////////////////////////////////////////
// Storage declarations
////////////////////////////////////////

storage {
    token0_reserve: u64 = 0,
    token1_reserve: u64 = 0,
    lp_token_supply: u64 = 0,
    deposits: StorageMap<(Address, ContractId), u64> = StorageMap {},
}

////////////////////////////////////////
// Helper functions
////////////////////////////////////////

// Calculate 0.3% fee
fn calculate_amount_with_fee(amount: u64) -> u64 {
    let fee: u64 = (amount / LIQUIDITY_MINER_FEE);
    amount - fee
}

fn mutiply_div(a: u64, b: u64, c: u64) -> u64 {
    let calculation = (~U128::from(0, a) * ~U128::from(0, b));
    let result_wrapped = (calculation / ~U128::from(0, c)).as_u64();

    // TODO remove workaround once https://github.com/FuelLabs/sway/pull/1671 lands.
    match result_wrapped {
        Result::Ok(inner_value) => inner_value, _ => revert(0), 
    }
}

/// Pricing function for converting between tokens.
fn get_input_price(input_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    assert(input_reserve > 0 && output_reserve > 0);
    let input_amount_with_fee: u64 = calculate_amount_with_fee(input_amount);
    let numerator = ~U128::from(0, input_amount_with_fee) * ~U128::from(0, output_reserve);
    let denominator = ~U128::from(0, input_reserve) + ~U128::from(0, input_amount_with_fee);
    let result_wrapped = (numerator / denominator).as_u64();
    // TODO remove workaround once https://github.com/FuelLabs/sway/pull/1671 lands.
    match result_wrapped {
        Result::Ok(inner_value) => inner_value, _ => revert(0), 
    }
}

/// Pricing function for converting between tokens.
fn get_output_price(output_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    assert(input_reserve > 0 && output_reserve > 0);
    let numerator = ~U128::from(0, input_reserve) * ~U128::from(0, output_amount);
    let denominator = ~U128::from(0, calculate_amount_with_fee(output_reserve - output_amount));
    let result_wrapped = (numerator / denominator).as_u64();
    if denominator > numerator {
        // Emulate Infinity Value
        ~u64::max()
    } else {
        // TODO remove workaround once https://github.com/FuelLabs/sway/pull/1671 lands.
        match result_wrapped {
            Result::Ok(inner_value) => inner_value + 1, _ => revert(0), 
        }
    }
}

#[storage(read, write)]fn store_reserves(reserve0: u64, reserve1: u64) {
    storage.token0_reserve = reserve0;
    storage.token1_reserve = reserve1;
}

// ////////////////////////////////////////
// // ABI definitions
// ////////////////////////////////////////
impl Exchange for Contract {
    #[storage(read)]fn get_balance(asset_id: ContractId) -> u64 {
        let sender = get_msg_sender_address_or_panic();
        storage.deposits.get((sender, asset_id))
    }

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

    #[storage(read, write)]fn deposit() {
        assert(msg_asset_id().into() == TOKEN_0 || msg_asset_id().into() == TOKEN_1);

        let sender = get_msg_sender_address_or_panic();

        let total_amount = storage.deposits.get((sender, msg_asset_id())) + msg_amount();
        storage.deposits.insert((sender, msg_asset_id()), total_amount);
    }

    #[storage(read, write)]fn withdraw(amount: u64, asset_id: ContractId) {
        assert(asset_id.into() == TOKEN_0 || asset_id.into() == TOKEN_1);

        let sender = get_msg_sender_address_or_panic();

        let deposited_amount = storage.deposits.get((sender, asset_id));
        assert(deposited_amount >= amount);

        let new_amount = deposited_amount - amount;
        storage.deposits.insert((sender, asset_id), new_amount);

        transfer_to_output(amount, asset_id, sender)
    }

    #[storage(read, write)]fn add_liquidity(min_liquidity: u64, deadline: u64) -> u64 {
        assert(msg_amount() == 0);
        assert(deadline > height());
        assert(msg_asset_id().into() == TOKEN_0 || msg_asset_id().into() == TOKEN_1);

        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = storage.lp_token_supply;

        let current_token_0_amount = storage.deposits.get((sender, ~ContractId::from(TOKEN_0)));
        let current_token_1_amount = storage.deposits.get((sender, ~ContractId::from(TOKEN_1)));

        assert(current_token_0_amount > 0);

        let mut minted: u64 = 0;
        if total_liquidity > 0 {
            assert(min_liquidity > 0);

            let token_0_reserve = storage.token0_reserve;
            let token_1_reserve = storage.token1_reserve;
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

                transfer_to_output(liquidity_minted, contract_id(), sender);

                // If user sent more than the correct ratio, we deposit back the extra tokens
                let token_extra = current_token_1_amount - token_1_amount;
                if (token_extra > 0) {
                    transfer_to_output(token_extra, ~ContractId::from(TOKEN_1), sender);
                }

                minted = liquidity_minted;
            } else {
                transfer_to_output(current_token_1_amount, ~ContractId::from(TOKEN_1), sender);
                transfer_to_output(current_token_0_amount, ~ContractId::from(TOKEN_0), sender);
                minted = 0;
            }
        } else {
            assert(current_token_0_amount > MINIMUM_LIQUIDITY);

            let initial_liquidity = current_token_0_amount;

            // Add fund to the reserves
            store_reserves(current_token_0_amount, current_token_1_amount);

            // Mint LP token
            mint(initial_liquidity);
            storage.lp_token_supply = initial_liquidity;

            transfer_to_output(initial_liquidity, contract_id(), sender);

            minted = initial_liquidity;
        };

        // Clear user contract balances after finishing add/create liquidity
        storage.deposits.insert((sender, ~ContractId::from(TOKEN_1)), 0);
        storage.deposits.insert((sender, ~ContractId::from(TOKEN_0)), 0);

        minted
    }

    #[storage(read, write)]fn remove_liquidity(min_token_0: u64, min_token_1: u64, deadline: u64) -> RemoveLiquidityInfo {
        assert(msg_amount() > 0);
        assert(msg_asset_id().into() == (contract_id()).into());
        assert(deadline > height());
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
        transfer_to_output(token_0_amount, ~ContractId::from(TOKEN_0), sender);
        transfer_to_output(token_1_amount, ~ContractId::from(TOKEN_1), sender);

        RemoveLiquidityInfo {
            token_0_amount: token_0_amount,
            token_1_amount: token_1_amount,
        }
    }

    #[storage(read, write)]fn swap_with_minimum(min: u64, deadline: u64) -> u64 {
        let asset_id = msg_asset_id().into();
        let input_amount = msg_amount();

        assert(deadline >= height());
        assert(input_amount > 0 && min > 0);
        assert(asset_id == TOKEN_0 || asset_id == TOKEN_1);

        let sender = get_msg_sender_address_or_panic();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let mut bought = 0;
        if (asset_id == TOKEN_0) {
            bought = get_input_price(input_amount, token_0_reserve, token_1_reserve);
            assert(bought >= min);
            transfer_to_output(bought, ~ContractId::from(TOKEN_1), sender);
            // Update reserve
            store_reserves(token_0_reserve + input_amount, token_1_reserve - bought);
        } else {
            bought = get_input_price(input_amount, token_1_reserve, token_0_reserve);
            assert(bought >= min);
            transfer_to_output(bought, ~ContractId::from(TOKEN_0), sender);
            // Update reserve
            store_reserves(token_0_reserve - bought, token_1_reserve + bought);
        };
        bought
    }

    #[storage(read, write)]fn swap_with_maximum(amount: u64, deadline: u64) -> u64 {
        let asset_id = msg_asset_id().into();
        let input_amount = msg_amount();

        assert(deadline >= height());
        assert(amount > 0 && input_amount > 0);
        assert(asset_id == TOKEN_0 || asset_id == TOKEN_1);

        let sender = get_msg_sender_address_or_panic();
        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let mut sold = 0;
        if (asset_id == TOKEN_0) {
            sold = get_output_price(amount, token_0_reserve, token_1_reserve);
            assert(input_amount >= sold);
            let refund = input_amount - sold;
            if refund > 0 {
                transfer_to_output(refund, ~ContractId::from(TOKEN_0), sender);
            };
            transfer_to_output(amount, ~ContractId::from(TOKEN_1), sender);
            // Update reserve
            store_reserves(token_0_reserve + sold, token_1_reserve - amount);
        } else {
            sold = get_output_price(amount, token_1_reserve, token_0_reserve);
            assert(input_amount >= sold);
            let refund = input_amount - sold;
            if refund > 0 {
                transfer_to_output(refund, ~ContractId::from(TOKEN_1), sender);
            };
            transfer_to_output(amount, ~ContractId::from(TOKEN_0), sender);
            // Update reserve
            store_reserves(token_0_reserve - amount, token_1_reserve + sold);
        };
        sold
    }

    #[storage(read)]fn get_swap_with_minimum(amount: u64) -> PreviewInfo {
        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let mut sold = 0;
        let mut has_liquidity = true;
        if (msg_asset_id().into() == TOKEN_0) {
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
        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let mut sold = 0;
        let mut has_liquidity = true;
        if (msg_asset_id().into() == TOKEN_0) {
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
