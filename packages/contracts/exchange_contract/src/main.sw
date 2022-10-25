contract;

use std::{
    address::*,
    assert::assert,
    block::*,
    chain::auth::*,
    constants::ZERO_B256,
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

use exchange_abi::{
    Exchange,
    PoolInfo,
    PositionInfo,
    PreviewInfo,
    RemoveLiquidityInfo,
    VaultFee,
    VaultInfo,
};
use microchain_helpers::{
    get_msg_sender_address_or_panic,
    get_input_price,
    get_output_price,
    mutiply_div,
};
use vault_abi::Vault;

enum Error {
    AlreadyInitialized: (),
    InsufficentOutput: (),
    InsufficentLiquidity: (),
    InsufficentInput: (),
    Invariant: (),
    InsufficentLiquidityMinted: (),
    InsufficentLiquidityBurned: (),
}

impl Root for U128 {
    // babylonian method (https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Babylonian_method)
    fn sqrt(self) -> Self {
        let mut z = ~U128::from(0, 0);
        if (self > ~U128::from(0, 3)) {
            z = self;
            let mut x = self / ~U128::from(0, 2) + ~U128::from(0, 1);
            while (x < z) {
                z = x;
                x = (self / x + x) / ~U128::from(0, 2);
            }
        } else if (self != ~U128::from(0, 0)) {
            z = ~U128::from(0, 1);
        }
        z
    }
}

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
    token0_vault_fees_collected: u64 = 0,
    token1_vault_fees_collected: u64 = 0,
    lp_token_supply: u64 = 0,
    vault: b256 = ZERO_B256,
    vault_fee: VaultFee = VaultFee {
        stored_fee: 0,
        change_rate: 0,
        update_time: 0,
    },
}

////////////////////////////////////////
// Helper functions
////////////////////////////////////////

#[storage(read)]fn get_input_amount(asset_id: b256, token_0_reserve: u64, token_1_reserve: u64) -> u64 {
    let (token0, token1) = get_tokens();
    let mut amount = 0;
    if (asset_id == token0) {
        amount = this_balance(~ContractId::from(token0)) - token_0_reserve - storage.token0_vault_fees_collected;
    } else if (asset_id == token1) {
        amount = this_balance(~ContractId::from(token1)) - token_1_reserve - storage.token1_vault_fees_collected;
    } else {
        revert(0);
    }
    amount
}

#[storage(read)]fn get_pool_balance() -> (u64, u64) {
    let (token0, token1) = get_tokens();
    let balance_0 = this_balance(~ContractId::from(token0)) - storage.token0_vault_fees_collected;
    let balance_1 = this_balance(~ContractId::from(token1)) - storage.token1_vault_fees_collected;
    (balance_0, balance_1)
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

#[storage(read, write)]fn process_protocol_fee(amount: u64, is_token0: bool) -> (u64, u64) {
    let fee_info = storage.vault_fee;
    let current_fee = fee_info.stored_fee - (fee_info.change_rate * (timestamp() - fee_info.update_time));
    // let fee = amount * current_fee / 1_000_000;
    let fee = (~U128::from(0, amount) * ~U128::from(0, current_fee) / ~U128::from(0, 1_000_000))
        .as_u64()
        .unwrap();

    if (fee > 0) {
        if (is_token0) {
            storage.token0_vault_fees_collected = storage.token0_vault_fees_collected + fee;
        } else {
            storage.token1_vault_fees_collected = storage.token1_vault_fees_collected + fee;
        }
    }
    (amount - fee, fee)
}

#[storage(write)]fn cache_vault_fees(vault: b256) {
    let vault = abi(Vault, vault);
    let vault_fees = vault.get_fees();
    storage.vault_fee = VaultFee {
        stored_fee: vault_fees.current_fee,
        change_rate: vault_fees.change_rate,
        update_time: timestamp(),
    };
}

// ////////////////////////////////////////
// // ABI definitions
// ////////////////////////////////////////
impl Exchange for Contract {
    #[storage(read, write)]fn initialize(new_vault: b256) {
        require(storage.vault == ZERO_B256, Error::AlreadyInitialized);
        storage.vault = new_vault;
        cache_vault_fees(new_vault);
    }

    #[storage(read)]fn get_pool_info() -> PoolInfo {
        PoolInfo {
            token_0_reserve: storage.token0_reserve,
            token_1_reserve: storage.token1_reserve,
            lp_token_supply: storage.lp_token_supply,
        }
    }

    #[storage(read)]fn get_vault_info() -> VaultInfo {
        let fees = storage.vault_fee;
        VaultInfo {
            vault: storage.vault,
            token0_protocol_fees_collected: storage.token0_vault_fees_collected,
            token1_protocol_fees_collected: storage.token1_vault_fees_collected,
            current_fee: fees.stored_fee - (fees.change_rate * (timestamp() - fees.update_time)),
            change_rate: fees.change_rate,
            update_time: fees.update_time,
        }
    }

    #[storage(read, write)]fn cache_vault_fees() {
        cache_vault_fees(storage.vault);
    }

    #[storage(read)]fn get_add_liquidity_token_amount(token_0_amount: u64) -> u64 {
        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let token_1_amount = mutiply_div(token_0_amount, token_1_reserve, token_0_reserve);
        token_1_amount
    }

    #[storage(read, write)]fn add_liquidity(recipient: Identity) -> u64 {
        let (token0, token1) = get_tokens();

        let total_liquidity = storage.lp_token_supply;

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        let (current_token_0_amount, current_token_1_amount) = get_pool_balance();

        assert(current_token_0_amount > 0);
        assert(current_token_1_amount > 0);

        let mut minted: u64 = 0;
        if total_liquidity > 0 {
            let token0_liquidity = current_token_0_amount * total_liquidity / token_0_reserve;
            let token1_liquidity = current_token_1_amount * total_liquidity / token_1_reserve;

            minted = if (token0_liquidity < token1_liquidity) { token0_liquidity } else { token1_liquidity };
            
            store_reserves(token_0_reserve + current_token_0_amount, token_1_reserve + current_token_1_amount);

            mint(minted);
            storage.lp_token_supply = total_liquidity + minted;
        } else {
            let initial_liquidity = (~U128::from(0, current_token_0_amount) * ~U128::from(0, current_token_1_amount))
                .sqrt()
                .as_u64()
                .unwrap() - MINIMUM_LIQUIDITY;

            // Add fund to the reserves
            store_reserves(current_token_0_amount, current_token_1_amount);

            // Mint LP token
            mint(initial_liquidity);
            storage.lp_token_supply = initial_liquidity + MINIMUM_LIQUIDITY;

            minted = initial_liquidity;
        };
        require(minted > 0, Error::InsufficentLiquidityMinted);

        transfer(minted, contract_id(), recipient);

        minted
    }

    #[storage(read, write)]fn remove_liquidity(recipient: Identity) -> RemoveLiquidityInfo {
        let (token0, token1) = get_tokens();

        let lp_tokens = this_balance(contract_id());
        require(lp_tokens > 0, Error::InsufficentInput);

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;
        let total_liquidity = storage.lp_token_supply;
        let (current_token_0_amount, current_token_1_amount) = get_pool_balance();

        let amount0 = lp_tokens * current_token_0_amount / total_liquidity; // using balances ensures pro-rata distribution
        let amount1 = lp_tokens * current_token_1_amount / total_liquidity; // using balances ensures pro-rata distribution
        require(amount0 > 0 && amount1 > 0, Error::InsufficentLiquidityBurned);
        
        burn(lp_tokens);
        storage.lp_token_supply = total_liquidity - lp_tokens;

        transfer(amount0, ~ContractId::from(token0), recipient);
        transfer(amount1, ~ContractId::from(token1), recipient);

        store_reserves(current_token_0_amount - amount0, current_token_1_amount - amount1);

        RemoveLiquidityInfo {
            token_0_amount: amount0,
            token_1_amount: amount1,
        }
    }

    #[storage(read, write)]fn swap(amount_0_out: u64, amount_1_out: u64, recipient: Identity) {
        require(amount_0_out > 0 || amount_1_out > 0, Error::InsufficentOutput);
        let (token0, token1) = get_tokens();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        require(amount_0_out < token_0_reserve && amount_1_out < token_1_reserve, Error::InsufficentLiquidity);

        if (amount_0_out > 0) {
            transfer(amount_0_out, ~ContractId::from(token0), recipient);
        }
        if (amount_1_out > 0) {
            transfer(amount_1_out, ~ContractId::from(token1), recipient);
        }
        // Should be the following line, but `let mut` doesn't work with destructuring
        // let (balance_0, balance_1) = get_pool_balance();
        let mut balance_0 = this_balance(~ContractId::from(token0)) - storage.token0_vault_fees_collected;
        let mut balance_1 = this_balance(~ContractId::from(token1)) - storage.token1_vault_fees_collected;

        let (amount0_in, amount0_protocol_fee) = if balance_0 > token_0_reserve - amount_0_out {
                process_protocol_fee(balance_0 - (token_0_reserve - amount_0_out), true)
            } else {
                (0, 0)
            };
        let (amount1_in, amount1_protocol_fee) = if balance_1 > token_1_reserve - amount_1_out {
                process_protocol_fee(balance_1 - (token_1_reserve - amount_1_out), false)
            } else {
                (0, 0)
            };

        require(amount0_in > 0 || amount1_in > 0, Error::InsufficentInput);

        balance_0 = balance_0 - amount0_protocol_fee;
        balance_1 = balance_1 - amount1_protocol_fee;

        let balance0_adjusted = ~U128::from(0, balance_0) * ~U128::from(0, 1000) - (~U128::from(0, amount0_in) * ~U128::from(0, 3));
        let balance1_adjusted = ~U128::from(0, balance_1) * ~U128::from(0, 1000) - (~U128::from(0, amount1_in) * ~U128::from(0, 3));

        let left = balance0_adjusted * balance1_adjusted;
        let right = ~U128::from(0, token_0_reserve) * ~U128::from(0, token_1_reserve) * ~U128::from(0, 1000 * 1000);
        require(left > right || left == right, Error::Invariant); // U128 doesn't have >= yet

        store_reserves(balance_0, balance_1);
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
