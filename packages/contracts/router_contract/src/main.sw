contract;

use std::{
    chain::auth::msg_sender,
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
    exact_output: bool,
    input: b256,
    // output: b256,
    pool: b256,
}

struct SwapOutput {
    input_amount: u64,
    output_amount: u64,
}

abi Router {
    fn simple_swap(swap_data: SwapData) -> SwapOutput;
}

impl Router for Contract {
    fn simple_swap(swap_data: SwapData) -> SwapOutput {
        // force_transfer_to_contract(exact_amount, swap_data.input, swap_data.pool);

        let exchange = abi(Exchange, swap_data.pool);
        let sender_identity = msg_sender().unwrap();

        if (swap_data.exact_output) {
            let input_amount = exchange.swap_with_maximum{
                asset_id: swap_data.input,
                coins: swap_data.slippage_amount,
            }(swap_data.input, swap_data.exact_amount, sender_identity);

            SwapOutput {
                input_amount: input_amount,
                output_amount: swap_data.exact_amount,
            }
        } else {
            let output_amount = exchange.swap_with_minimum{
                asset_id: swap_data.input,
                coins: swap_data.exact_amount,
            }(swap_data.input, swap_data.slippage_amount, sender_identity);

            SwapOutput {
                input_amount: swap_data.exact_amount,
                output_amount: output_amount,
            }
        }
    }
}
