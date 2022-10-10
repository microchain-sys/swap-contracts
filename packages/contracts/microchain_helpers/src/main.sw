library microchain_helpers;

use std::{
    address::*,
    block::*,
    chain::auth::*,
    context::{*, call_frames::*},
    contract_id::ContractId,
    result::*,
    revert::{revert, require},
    identity::Identity,
    math::Root,
    u128::U128,
};


enum Error {
    InsufficentReserves: (),
    InsufficentAmount: (),
}

// Liquidity miner fee apply to all swaps
const LIQUIDITY_MINER_FEE = 333;

// Calculate 0.3% fee
pub fn calculate_amount_with_fee(amount: u64) -> u64 {
    let fee: u64 = (amount / LIQUIDITY_MINER_FEE);
    amount - fee
}

pub fn mutiply_div(a: u64, b: u64, c: u64) -> u64 {
    let calculation = (~U128::from(0, a) * ~U128::from(0, b));
    let result_wrapped = (calculation / ~U128::from(0, c)).as_u64();

    // TODO remove workaround once https://github.com/FuelLabs/sway/pull/1671 lands.
    match result_wrapped {
        Result::Ok(inner_value) => inner_value, _ => revert(0), 
    }
}

/// Pricing function for converting between tokens.
pub fn get_input_price(input_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    require(input_amount > 0, Error::InsufficentAmount);
    require(input_reserve > 0 && output_reserve > 0, Error::InsufficentReserves);
    let input_amount_with_fee = ~U128::from(0, input_amount) * ~U128::from(0, 997);
    let numerator = input_amount_with_fee * ~U128::from(0, output_reserve);
    let denominator = (~U128::from(0, input_reserve) * ~U128::from(0, 1000)) + input_amount_with_fee;
    let result_wrapped = (numerator / denominator).as_u64();
    result_wrapped.unwrap()
}

/// Pricing function for converting between tokens.
pub fn get_output_price(output_amount: u64, input_reserve: u64, output_reserve: u64) -> u64 {
    require(output_amount > 0, Error::InsufficentAmount);
    require(input_reserve > 0 && output_reserve > 0, Error::InsufficentReserves);

    let numerator = ~U128::from(0, input_reserve) * ~U128::from(0, output_amount) * ~U128::from(0, 1000);
    let denominator = ~U128::from(0, output_reserve - output_amount) * ~U128::from(0, 997);
    let amount_in = (numerator / denominator) + ~U128::from(0, 1);
    amount_in.as_u64().unwrap()
}

pub fn quote(amount_a: u64, reserve_a: u64, reserve_b: u64) -> u64 {
    require(amount_a > 0, Error::InsufficentAmount());
    require(reserve_a > 0 && reserve_b > 0, Error::InsufficentReserves);
    return amount_a * reserve_b / reserve_a;
}

#[storage(read)]pub fn get_b256(key: b256) -> b256 {
    asm(r1: key, r2) {
        move r2 sp;
        cfei i32;
        srwq r2 r1;
        r2: b256
    }
}

// Store b256 values on memory
#[storage(write)]pub fn store_b256(key: b256, value: b256) {
    asm(r1: key, r2: value) {
        swwq r1 r2;
    };
}

/// Return the sender as an Address or panic
pub fn get_msg_sender_address_or_panic() -> Address {
    let sender: Result<Identity, AuthError> = msg_sender();
    if let Identity::Address(address) = sender.unwrap() {
       address
    } else {
       revert(0);
    }
}
