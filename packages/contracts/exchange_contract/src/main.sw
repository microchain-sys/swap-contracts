contract;

use std::{
    auth::msg_sender,
    assert::assert,
    block::timestamp,
    call_frames::contract_id,
    constants::ZERO_B256,
    context::*,
    contract_id::ContractId,
    identity::Identity,
    logging::log,
    revert::revert,
    storage::get,
    token::{burn, mint, transfer},
    u128::U128,
    u256::U256,
};

use exchange_abi::{
    Exchange,
    FeeInfo,
    LiquidityAdded,
    LiquidityRemoved,
    Observation,
    PoolInfo,
    ProtocolFeeCollected,
    ProtocolFeeWithdrawn,
    RemoveLiquidityInfo,
    Swap,
    TWAPInfo,
    UpdateReserves,
    VaultInfo,
};
use microchain_helpers::{identity_to_b256};
use vault_abi::Vault;

enum Error {
    AlreadyInitialized: (),
    InsufficentOutput: (),
    InsufficentLiquidity: (),
    InsufficentInput: (),
    Invariant: (),
    InsufficentLiquidityMinted: (),
    InsufficentLiquidityBurned: (),
    MustBeCalledByVault: (),
    TWAPOutOfRange: (),
}

////////////////////////////////////////
// Constants
////////////////////////////////////////
const TOKEN_0_SLOT = 0x0000000000000000000000000000000000000000000000000000000000000000;
const TOKEN_1_SLOT = 0x0000000000000000000000000000000000000000000000000000000000000001;

/// Minimum ETH liquidity to open a pool.
const MINIMUM_LIQUIDITY = 1000;

const TWAP_PERCISION = U256::from((0, 0, 0, 1000000000));

////////////////////////////////////////
// Storage declarations
////////////////////////////////////////
// Packed into a single 8-byte slot
struct VaultFee {
    // These values should be divided by 1,000,000 to get the rate. So 10,000 = 1%
    stored_fee: u16, // 1 byte
    change_rate: u16, // 1 byte
    // TODO: directional changes
    // fee_increasing: bool, // 1 byte
    update_time: u32, // 4 bytes
}

storage {
    token0_reserve: u64 = 0,
    token1_reserve: u64 = 0,
    token0_vault_fees_collected: u64 = 0,
    token1_vault_fees_collected: u64 = 0,
    lp_token_supply: u64 = 0,
    vault: b256 = ZERO_B256,
    vault_fee: VaultFee = VaultFee {
        stored_fee: 0u16,
        change_rate: 0u16,
        update_time: 0u32,
    },
    // the most-recently updated index of the TWAP buffer
    twap_buffer_current_element: u64 = 0,
    // the current maximum number of observations that are being stored
    twap_buffer_size: u64 = 0,
    // the next maximum number of observations to store, triggered in observations.write
    twap_next_buffer_size: u64 = 0,
    twap_buffer: StorageMap<u64, Observation> = StorageMap {},
}

////////////////////////////////////////
// Helper functions
////////////////////////////////////////
#[storage(read)]
fn get_input_amount(asset_id: b256, token_0_reserve: u64, token_1_reserve: u64) -> u64 {
    let (token0, token1) = get_tokens();
    let mut amount = 0;
    if (asset_id == token0) {
        amount = this_balance(ContractId::from(token0)) - token_0_reserve - storage.token0_vault_fees_collected;
    } else if (asset_id == token1) {
        amount = this_balance(ContractId::from(token1)) - token_1_reserve - storage.token1_vault_fees_collected;
    } else {
        revert(0);
    }
    amount
}

#[storage(read)]
fn get_pool_balance() -> (u64, u64) {
    let (token0, token1) = get_tokens();
    let balance_0 = this_balance(ContractId::from(token0)) - storage.token0_vault_fees_collected;
    let balance_1 = this_balance(ContractId::from(token1)) - storage.token1_vault_fees_collected;
    (balance_0, balance_1)
}

#[storage(read, write)]
fn store_reserves(
    reserve0: u64,
    reserve1: u64,
    prev_reserve0: u64,
    prev_reserve1: u64,
) {
    let buffer_size = storage.twap_buffer_size;
    let next_buffer_size = storage.twap_next_buffer_size;
    let current_element = storage.twap_buffer_current_element;

    let mut next_observation_slot = (current_element + 1) % next_buffer_size;
    let mut last_observation = storage.twap_buffer.get(current_element);
    if (last_observation.timestamp <= 1) {
        next_observation_slot = current_element;
        last_observation = Observation {
            timestamp: timestamp(),
            price_0_cumulative_last: U256::min(),
            price_1_cumulative_last: U256::min(),
        };
    }

    let time_elapsed = timestamp() - last_observation.timestamp;
    if (time_elapsed != 0
        && prev_reserve0 != 0
        && prev_reserve1 != 0)
    {
        let time_elapsed_u256 = U256::from((0, 0, 0, time_elapsed));
        let price_0_cumulative = U256::from((0, 0, 0, prev_reserve1)) * TWAP_PERCISION * time_elapsed_u256 / U256::from((0, 0, 0, prev_reserve0));
        let price_1_cumulative = U256::from((0, 0, 0, prev_reserve0)) * TWAP_PERCISION * time_elapsed_u256 / U256::from((0, 0, 0, prev_reserve1));

        storage.twap_buffer.insert(next_observation_slot, Observation {
            timestamp: timestamp(),
            price_0_cumulative_last: last_observation.price_0_cumulative_last + price_0_cumulative,
            price_1_cumulative_last: last_observation.price_1_cumulative_last + price_1_cumulative,
        });

        storage.twap_buffer_current_element = next_observation_slot;
        if (next_observation_slot >= buffer_size) {
            storage.twap_buffer_size = buffer_size + 1;
        }
    }

    storage.token0_reserve = reserve0;
    storage.token1_reserve = reserve1;

    log(UpdateReserves {
        amount_0: reserve0,
        amount_1: reserve1,
    })
}

#[storage(read)]
fn get_tokens() -> (b256, b256) {
    (get::<b256>(TOKEN_0_SLOT), get::<b256>(TOKEN_1_SLOT))
}

#[storage(read)]
fn get_current_fee() -> u64 {
    let fee_info = storage.vault_fee;
    let decrease_since_storage = fee_info.change_rate * (timestamp() - fee_info.update_time);
    if decrease_since_storage > fee_info.stored_fee {
        0
    } else {
        fee_info.stored_fee - decrease_since_storage
    }
}

#[storage(read, write)]
fn process_protocol_fee(amount: u64, is_token0: bool) -> (u64, u64) {
    let current_fee_rate = get_current_fee();
    let mut fee = 0;

    if (current_fee_rate > 0) {
        fee = (U128::from((0, amount)) * U128::from((0, current_fee_rate)) / U128::from((0, 1_000_000))).as_u64().unwrap();
        let sender: b256 = identity_to_b256(msg_sender().unwrap());

        let mut amount_0 = 0;
        let mut amount_1 = 0;
        if (is_token0) {
            storage.token0_vault_fees_collected = storage.token0_vault_fees_collected + fee;
            amount_0 = fee;
        } else {
            storage.token1_vault_fees_collected = storage.token1_vault_fees_collected + fee;
            amount_1 = fee;
        }

        log(ProtocolFeeCollected {
            sender: sender,
            amount_0: amount_0,
            amount_1: amount_1,
        });
    }
    (amount - fee, fee)
}

#[storage(write)]
fn cache_vault_fees(vault: b256) {
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
    #[storage(read, write)]
    fn initialize(new_vault: b256) {
        require(storage.vault == ZERO_B256, Error::AlreadyInitialized);
        storage.vault = new_vault;
        cache_vault_fees(new_vault);
    }

    #[storage(read)]
    fn get_pool_info() -> PoolInfo {
        PoolInfo {
            token_0_reserve: storage.token0_reserve,
            token_1_reserve: storage.token1_reserve,
            lp_token_supply: storage.lp_token_supply,
        }
    }

    #[storage(read)]
    fn get_vault_info() -> VaultInfo {
        let fees = storage.vault_fee;
        VaultInfo {
            vault: storage.vault,
            token0_protocol_fees_collected: storage.token0_vault_fees_collected,
            token1_protocol_fees_collected: storage.token1_vault_fees_collected,
            current_fee: get_current_fee(),
            change_rate: fees.change_rate,
            update_time: fees.update_time,
        }
    }

    #[storage(read)]
    fn get_fee_info() -> FeeInfo {
        let fees = storage.vault_fee;
        FeeInfo {
            current_fee: get_current_fee(),
            change_rate: fees.change_rate,
            update_time: fees.update_time,
        }
    }

    #[storage(read)]
    fn get_twap_info() -> TWAPInfo {
        TWAPInfo {
            current_element: storage.twap_buffer_current_element,
            buffer_size: storage.twap_buffer_size,
            next_buffer_size: storage.twap_next_buffer_size,
        }
    }

    #[storage(read, write)]
    fn cache_vault_fees() {
        cache_vault_fees(storage.vault);
    }

    #[storage(read, write)]
    fn add_liquidity(recipient: Identity) -> u64 {
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

            minted = if (token0_liquidity < token1_liquidity) {
                token0_liquidity
            } else {
                token1_liquidity
            };

            store_reserves(token_0_reserve + current_token_0_amount, token_1_reserve + current_token_1_amount, token_0_reserve, token_1_reserve);

            mint(minted);
            storage.lp_token_supply = total_liquidity + minted;
        } else {
            let initial_liquidity = (U128::from((0, current_token_0_amount)) * U128::from((0, current_token_1_amount))).sqrt().as_u64().unwrap() - MINIMUM_LIQUIDITY;

            // Ensure there's at least 1 TWAP slot
            storage.twap_buffer_size = 1;
            storage.twap_next_buffer_size = 1;
            storage.twap_buffer.insert(0, Observation {
                timestamp: timestamp(),
                price_0_cumulative_last: U256::min(),
                price_1_cumulative_last: U256::min(),
            });

            // Add fund to the reserves
            store_reserves(current_token_0_amount, current_token_1_amount, 0, 0);

            // Mint LP token
            mint(initial_liquidity);
            storage.lp_token_supply = initial_liquidity + MINIMUM_LIQUIDITY;

            minted = initial_liquidity;

            // Log the liquidity that's burned
            log(LiquidityAdded {
                sender: identity_to_b256(msg_sender().unwrap()),
                amount_0: 0,
                amount_1: 0,
                lp_tokens: MINIMUM_LIQUIDITY,
                recipient: b256::min(),
            });
        };
        require(minted > 0, Error::InsufficentLiquidityMinted);

        transfer(minted, contract_id(), recipient);

        log(LiquidityAdded {
            sender: identity_to_b256(msg_sender().unwrap()),
            amount_0: current_token_0_amount - token_0_reserve,
            amount_1: current_token_1_amount - token_1_reserve,
            lp_tokens: minted,
            recipient: identity_to_b256(recipient),
        });

        minted
    }

    #[storage(read, write)]
    fn remove_liquidity(recipient: Identity) -> RemoveLiquidityInfo {
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

        transfer(amount0, ContractId::from(token0), recipient);
        transfer(amount1, ContractId::from(token1), recipient);

        store_reserves(current_token_0_amount - amount0, current_token_1_amount - amount1, token_0_reserve, token_1_reserve);

        log(LiquidityRemoved {
            sender: identity_to_b256(msg_sender().unwrap()),
            amount_0: amount0,
            amount_1: amount1,
            lp_tokens: lp_tokens,
            recipient: identity_to_b256(recipient),
        });

        RemoveLiquidityInfo {
            token_0_amount: amount0,
            token_1_amount: amount1,
        }
    }

    #[storage(read, write)]
    fn swap(amount_0_out: u64, amount_1_out: u64, recipient: Identity) {
        require(amount_0_out > 0 || amount_1_out > 0, Error::InsufficentOutput);
        let (token0, token1) = get_tokens();

        let token_0_reserve = storage.token0_reserve;
        let token_1_reserve = storage.token1_reserve;

        require(amount_0_out < token_0_reserve && amount_1_out < token_1_reserve, Error::InsufficentLiquidity);

        if (amount_0_out > 0) {
            transfer(amount_0_out, ContractId::from(token0), recipient);
        }
        if (amount_1_out > 0) {
            transfer(amount_1_out, ContractId::from(token1), recipient);
        }
        // Should be the following line, but `let mut` doesn't work with destructuring
        // let (balance_0, balance_1) = get_pool_balance();
        let mut balance_0 = this_balance(ContractId::from(token0)) - storage.token0_vault_fees_collected;
        let mut balance_1 = this_balance(ContractId::from(token1)) - storage.token1_vault_fees_collected;

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

        let balance0_adjusted = U128::from((0, balance_0)) * U128::from((0, 1000)) - (U128::from((0, amount0_in)) * U128::from((0, 3)));
        let balance1_adjusted = U128::from((0, balance_1)) * U128::from((0, 1000)) - (U128::from((0, amount1_in)) * U128::from((0, 3)));

        let left = balance0_adjusted * balance1_adjusted;
        let right = U128::from((0, token_0_reserve)) * U128::from((0, token_1_reserve)) * U128::from((0, 1000 * 1000));
        require(left > right || left == right, Error::Invariant); // U128 doesn't have >= yet
        store_reserves(balance_0, balance_1, token_0_reserve, token_1_reserve);

        log(Swap {
            sender: identity_to_b256(msg_sender().unwrap()),
            amount_0_in: amount0_in,
            amount_1_in: amount1_in,
            amount_0_out: amount_0_out,
            amount_1_out: amount_1_out,
            recipient: identity_to_b256(recipient),
        });
    }

    #[storage(read, write)]
    fn expand_twap_buffer(new_total_slots: u64) {
        let mut i = storage.twap_next_buffer_size;

        while i < new_total_slots {
            // The goal here is to initialize these slots (write a value), without
            // providing actual data. Observations will be aware that timestamp=1 means
            // initialized, but not yet used
            storage.twap_buffer.insert(i, Observation {
                timestamp: 1,
                price_0_cumulative_last: U256::max(),
                price_1_cumulative_last: U256::max(),
            });
            i += 1;
        }

        storage.twap_next_buffer_size = new_total_slots;
    }

    #[storage(read, write)]
    fn withdraw_protocol_fees(recipient: Identity) -> (u64, u64) {
        let sender: Identity = msg_sender().unwrap();
        require(identity_to_b256(sender) == storage.vault, Error::MustBeCalledByVault);

        let (token0, token1) = get_tokens();
        let (token0_vault_fees_collected, token1_vault_fees_collected) = (
            storage.token0_vault_fees_collected,
            storage.token1_vault_fees_collected,
        );

        if (token0_vault_fees_collected > 0) {
            transfer(token0_vault_fees_collected, ContractId::from(token0), recipient);
            storage.token0_vault_fees_collected = 0;
        }
        if (token1_vault_fees_collected > 0) {
            transfer(token1_vault_fees_collected, ContractId::from(token1), recipient);
            storage.token1_vault_fees_collected = 0;
        }

        log(ProtocolFeeWithdrawn {
            amount_0: token0_vault_fees_collected,
            amount_1: token1_vault_fees_collected,
        });
        (token0_vault_fees_collected, token1_vault_fees_collected)
    }

    #[storage(read)]
    fn get_tokens() -> (b256, b256) {
        get_tokens()
    }

    #[storage(read)]
    fn get_observation(slot: u64) -> Observation {
        require(slot < storage.twap_buffer_size, Error::TWAPOutOfRange);
        storage.twap_buffer.get(slot)
    }
}
