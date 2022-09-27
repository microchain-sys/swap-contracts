contract;

use std::{
    chain::auth::msg_sender,
    context::call_frames::msg_asset_id,
    // inputs::{Input, inputs_owner}
};
use exchange_abi::Exchange;

use std::{
    address::Address,
    chain::auth::{AuthError},
    assert::assert,
    b512::B512,
    contract_id::ContractId,
    identity::Identity,
    option::Option,
    result::Result,
    inputs::{Input, input_count, input_owner, input_type},
};

struct SwapData {
    exact_amount: u64,
    slippage_amount: u64,
    // output: b256,
    pool: b256,
}

struct SwapOutput {
    input_amount: u64,
    output_amount: u64,
}

abi Router {
    fn swap_exact_input(swap_data: SwapData) -> SwapOutput;
    fn swap_exact_output(swap_data: SwapData) -> SwapOutput;
}

impl Router for Contract {
    fn swap_exact_input(swap_data: SwapData) -> SwapOutput {
        let exchange = abi(Exchange, swap_data.pool);
        let sender_identity = msg_sender().unwrap();

        let output_amount = exchange.swap_with_minimum{
            asset_id: msg_asset_id().into(),
            coins: swap_data.exact_amount,
        }(msg_asset_id().into(), swap_data.slippage_amount, sender_identity);

        SwapOutput {
            input_amount: swap_data.exact_amount,
            output_amount: output_amount,
        }
    }

    fn swap_exact_output(swap_data: SwapData) -> SwapOutput {
        let exchange = abi(Exchange, swap_data.pool);
        let sender_identity = msg_sender().unwrap();

        let input_amount = exchange.swap_with_maximum{
            asset_id: msg_asset_id().into(),
            coins: swap_data.slippage_amount,
        }(msg_asset_id().into(), swap_data.exact_amount, sender_identity);

        SwapOutput {
            input_amount: input_amount,
            output_amount: swap_data.exact_amount,
        }
    }
}
