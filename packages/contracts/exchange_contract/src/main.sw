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

use exchange_abi::{Exchange, Observation, PoolInfo, PositionInfo, PreviewInfo, RemoveLiquidityInfo};
use swayswap_helpers::get_msg_sender_address_or_panic;

////////////////////////////////////////
// Constants
////////////////////////////////////////

/// Token ID of Ether
const ETH_ID = 0x0000000000000000000000000000000000000000000000000000000000000000;

/// Contract ID of the token on the other side of the pool.
/// Modify at compile time for different pool.
const TOKEN_ID = 0xeb4f49ab76e1866ba27bdc9392373ad868d1efc88bbcd0eb67c8b066308a060d;

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
    twap_buffer_size: u64 = 0,
    twap_buffer_current_element: u64 = 0,
    twap_buffer: StorageMap<u64, Observation> = StorageMap {},
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

#[storage(read, write)]fn store_reserves(reserve0: u64, reserve1: u64, prev_reserve0: u64, prev_reserve1: u64) {
    assert(storage.twap_buffer_size > 0);
    let mut next_observation_slot = (storage.twap_buffer_current_element + 1) % storage.twap_buffer_size;

    let mut last_observation = storage.twap_buffer.get(storage.twap_buffer_current_element);
    if (last_observation.timestamp <= 1) {
        next_observation_slot = storage.twap_buffer_current_element;
        last_observation = Observation {
            timestamp: height(),
            price0CumulativeLast: 0,
            price1CumulativeLast: 0,
        };
    }

    let timeElapsed = height() - last_observation.timestamp;
    if (timeElapsed != 0 && prev_reserve0 != 0 && prev_reserve1 != 0) {
        storage.twap_buffer.insert(next_observation_slot, Observation {
            timestamp: height(),
            price0CumulativeLast: last_observation.price0CumulativeLast + ((prev_reserve1 / prev_reserve0) * timeElapsed),
            price1CumulativeLast: last_observation.price1CumulativeLast + ((prev_reserve0 / prev_reserve1) * timeElapsed),
        });
    }

    storage.token0_reserve = reserve0;
    storage.token1_reserve = reserve1;
}

/// Pricing function for converting between ETH and Tokens.
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

/// Pricing function for converting between ETH and Tokens.
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
            eth_reserve: storage.token0_reserve,
            token_reserve: storage.token1_reserve,
            lp_token_supply: storage.lp_token_supply,
            twap_buffer_size: storage.twap_buffer_size,
        }
    }

    #[storage(read)]fn get_add_liquidity_token_amount(eth_amount: u64) -> u64 {
        let eth_reserve = storage.token0_reserve;
        let token_reserve = storage.token1_reserve;
        let token_amount = mutiply_div(eth_amount, token_reserve, eth_reserve);
        token_amount
    }

    #[storage(read, write)]fn deposit() {
        assert(msg_asset_id().into() == ETH_ID || msg_asset_id().into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let total_amount = storage.deposits.get((sender, msg_asset_id())) + msg_amount();
        storage.deposits.insert((sender, msg_asset_id()), total_amount);
    }

    #[storage(read, write)]fn withdraw(amount: u64, asset_id: ContractId) {
        assert(asset_id.into() == ETH_ID || asset_id.into() == TOKEN_ID);

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
        assert(msg_asset_id().into() == ETH_ID || msg_asset_id().into() == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = storage.lp_token_supply;

        let current_eth_amount = storage.deposits.get((sender, ~ContractId::from(ETH_ID)));
        let current_token_amount = storage.deposits.get((sender, ~ContractId::from(TOKEN_ID)));

        assert(current_eth_amount > 0);

        let mut minted: u64 = 0;
        if total_liquidity > 0 {
            assert(min_liquidity > 0);

            let eth_reserve = storage.token0_reserve;
            let token_reserve = storage.token1_reserve;
            let token_amount = mutiply_div(current_eth_amount, token_reserve, eth_reserve);
            let liquidity_minted = mutiply_div(current_eth_amount, total_liquidity, eth_reserve);

            assert(liquidity_minted >= min_liquidity);

            // if token ratio is correct, proceed with liquidity operation
            // otherwise, return current user balances in contract
            if (current_token_amount >= token_amount) {
                // Add fund to the reserves
                store_reserves(current_eth_amount + eth_reserve, token_amount + token_reserve, eth_reserve, token_reserve);

                // Mint LP token
                mint(liquidity_minted);
                storage.lp_token_supply = total_liquidity + liquidity_minted;

                transfer_to_output(liquidity_minted, contract_id(), sender);

                // If user sent more than the correct ratio, we deposit back the extra tokens
                let token_extra = current_token_amount - token_amount;
                if (token_extra > 0) {
                    transfer_to_output(token_extra, ~ContractId::from(TOKEN_ID), sender);
                }

                minted = liquidity_minted;
            } else {
                transfer_to_output(current_token_amount, ~ContractId::from(TOKEN_ID), sender);
                transfer_to_output(current_eth_amount, ~ContractId::from(ETH_ID), sender);
                minted = 0;
            }
        } else {
            assert(current_eth_amount > MINIMUM_LIQUIDITY);

            let initial_liquidity = current_eth_amount;

            // Add fund to the reserves
            store_reserves(current_eth_amount, current_token_amount, 0, 0);

            // Mint LP token
            mint(initial_liquidity);
            storage.lp_token_supply = initial_liquidity;

            transfer_to_output(initial_liquidity, contract_id(), sender);

            minted = initial_liquidity;
        };

        // Clear user contract balances after finishing add/create liquidity
        storage.deposits.insert((sender, ~ContractId::from(TOKEN_ID)), 0);
        storage.deposits.insert((sender, ~ContractId::from(ETH_ID)), 0);

        minted
    }

    #[storage(read, write)]fn remove_liquidity(min_eth: u64, min_tokens: u64, deadline: u64) -> RemoveLiquidityInfo {
        assert(msg_amount() > 0);
        assert(msg_asset_id().into() == (contract_id()).into());
        assert(deadline > height());
        assert(min_eth > 0 && min_tokens > 0);

        let sender = get_msg_sender_address_or_panic();

        let total_liquidity = storage.lp_token_supply;
        assert(total_liquidity > 0);

        let eth_reserve = storage.token0_reserve;
        let token_reserve = storage.token1_reserve;
        let eth_amount = mutiply_div(msg_amount(), eth_reserve, total_liquidity);
        let token_amount = mutiply_div(msg_amount(), token_reserve, total_liquidity);

        assert((eth_amount >= min_eth) && (token_amount >= min_tokens));

        burn(msg_amount());
        storage.lp_token_supply = total_liquidity - msg_amount();

        // Add fund to the reserves
        store_reserves(eth_reserve - eth_amount, token_reserve - token_amount, eth_reserve, token_reserve);

        // Send tokens back
        transfer_to_output(eth_amount, ~ContractId::from(ETH_ID), sender);
        transfer_to_output(token_amount, ~ContractId::from(TOKEN_ID), sender);

        RemoveLiquidityInfo {
            eth_amount: eth_amount,
            token_amount: token_amount,
        }
    }

    #[storage(read, write)]fn swap_with_minimum(min: u64, deadline: u64) -> u64 {
        let asset_id = msg_asset_id().into();
        let forwarded_amount = msg_amount();

        assert(deadline >= height());
        assert(forwarded_amount > 0 && min > 0);
        assert(asset_id == ETH_ID || asset_id == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();

        let eth_reserve = storage.token0_reserve;
        let token_reserve = storage.token1_reserve;

        let mut bought = 0;
        if (asset_id == ETH_ID) {
            let tokens_bought = get_input_price(forwarded_amount, eth_reserve, token_reserve);
            assert(tokens_bought >= min);
            transfer_to_output(tokens_bought, ~ContractId::from(TOKEN_ID), sender);
            bought = tokens_bought;
            // Update reserve
            store_reserves(eth_reserve + forwarded_amount, token_reserve - tokens_bought, eth_reserve, token_reserve);
        } else {
            let eth_bought = get_input_price(forwarded_amount, token_reserve, eth_reserve);
            assert(eth_bought >= min);
            transfer_to_output(eth_bought, ~ContractId::from(ETH_ID), sender);
            bought = eth_bought;
            // Update reserve
            store_reserves(eth_reserve - eth_bought, token_reserve + bought, eth_reserve, token_reserve);
        };
        bought
    }

    #[storage(read, write)]fn swap_with_maximum(amount: u64, deadline: u64) -> u64 {
        let asset_id = msg_asset_id().into();
        let forwarded_amount = msg_amount();

        assert(deadline >= height());
        assert(amount > 0 && forwarded_amount > 0);
        assert(asset_id == ETH_ID || asset_id == TOKEN_ID);

        let sender = get_msg_sender_address_or_panic();
        let eth_reserve = storage.token0_reserve;
        let token_reserve = storage.token1_reserve;

        let mut sold = 0;
        if (asset_id == ETH_ID) {
            let eth_sold = get_output_price(amount, eth_reserve, token_reserve);
            assert(forwarded_amount >= eth_sold);
            let refund = forwarded_amount - eth_sold;
            if refund > 0 {
                transfer_to_output(refund, ~ContractId::from(ETH_ID), sender);
            };
            transfer_to_output(amount, ~ContractId::from(TOKEN_ID), sender);
            sold = eth_sold;
            // Update reserve
            store_reserves(eth_reserve + eth_sold, token_reserve - amount, eth_reserve, token_reserve);
        } else {
            let tokens_sold = get_output_price(amount, token_reserve, eth_reserve);
            assert(forwarded_amount >= tokens_sold);
            let refund = forwarded_amount - tokens_sold;
            if refund > 0 {
                transfer_to_output(refund, ~ContractId::from(TOKEN_ID), sender);
            };
            transfer_to_output(amount, ~ContractId::from(ETH_ID), sender);
            sold = tokens_sold;
            // Update reserve
            store_reserves(eth_reserve - amount, token_reserve + tokens_sold, eth_reserve, token_reserve);
        };
        sold
    }

    #[storage(read, write)]fn get_swap_with_minimum(amount: u64) -> PreviewInfo {
        let eth_reserve = storage.token0_reserve;
        let token_reserve = storage.token1_reserve;
        let mut sold = 0;
        let mut has_liquidity = true;
        if (msg_asset_id().into() == ETH_ID) {
            sold = get_input_price(amount, eth_reserve, token_reserve);
            has_liquidity = sold < token_reserve;
        } else {
            sold = get_input_price(amount, token_reserve, eth_reserve);
            has_liquidity = sold < eth_reserve;
        }
        PreviewInfo {
            amount: sold,
            has_liquidity: has_liquidity,
        }
    }

    #[storage(read, write)]fn get_swap_with_maximum(amount: u64) -> PreviewInfo {
        let eth_reserve = storage.token0_reserve;
        let token_reserve = storage.token1_reserve;
        let mut sold = 0;
        let mut has_liquidity = true;
        if (msg_asset_id().into() == ETH_ID) {
            sold = get_output_price(amount, eth_reserve, token_reserve);
            has_liquidity = sold < eth_reserve;
        } else {
            sold = get_output_price(amount, token_reserve, eth_reserve);
            has_liquidity = sold < token_reserve;
        }
        PreviewInfo {
            amount: sold,
            has_liquidity: has_liquidity,
        }
    }

    #[storage(read, write)]fn expand_twap_buffer(new_slots: u64) {
        let next_slot = storage.twap_buffer_size + 1;
        let mut i = 0;

        while i < new_slots {
            // The goal here is to initialize these slots (write a value), without
            // providing actual data. Observations will be aware that timestamp=1 means
            // initialized, but not yet used
            storage.twap_buffer.insert(next_slot, Observation {
                timestamp: 1,
                price0CumulativeLast: 1,
                price1CumulativeLast: 1,
            });
            i += 1;
        }

        storage.twap_buffer_size += new_slots;
    }
}
