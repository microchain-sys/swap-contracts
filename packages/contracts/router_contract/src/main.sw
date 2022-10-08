contract;

use std::{
    chain::auth::msg_sender,
    context::call_frames::msg_asset_id,
};
use exchange_abi::Exchange;

use std::{
    address::Address,
    chain::auth::{AuthError},
    b512::B512,
    context::this_balance,
    contract_id::ContractId,
    identity::Identity,
    option::Option,
    result::Result,
    revert::{revert, require},
    token::{
        transfer,
        force_transfer_to_contract,
    },
    logging::log,
    inputs::{
        Input,
        input_count,
        input_owner,
        input_type,
    },
};

use microchain_helpers::{
    quote,
    get_input_price,
    get_output_price,
};

enum Error {
    InsufficentOutput: (),
    ExcessiveInput: (),
    InsufficentAAmount: (),
    InsufficentBAmount: (),
    // InvalidToken: (),
}

struct SwapData {
    exact_amount: u64,
    slippage_amount: u64,
    // output: b256,
    pool: b256,
}

struct AddLiquidityOutput {
    amount_a: u64,
    amount_b: u64,
    liquidity: u64,
}

struct SwapOutput {
    input_amount: u64,
    output_amount: u64,
}

abi Router {
    fn add_liquidity(
        pool: b256,
        amount_a_desired: u64,
        amount_b_desired: u64,
        amount_a_min: u64,
        amount_b_min: u64,
        recipient: Identity,
    ) -> AddLiquidityOutput;

    fn swap_exact_input(swap_data: SwapData) -> SwapOutput;
    fn swap_exact_output(swap_data: SwapData) -> SwapOutput;
}

impl Router for Contract {
    fn add_liquidity(
        pool: b256,
        amount_a_desired: u64,
        amount_b_desired: u64,
        amount_a_min: u64,
        amount_b_min: u64,
        recipient: Identity,
    ) -> AddLiquidityOutput {
        let exchange = abi(Exchange, pool);
        let (token0, token1) = exchange.get_tokens();
        let pool_info = exchange.get_pool_info();
        let sender_identity = msg_sender().unwrap(); // Only used for returning "change"
        // TODO: We assume tokenA == token0 & tokenB == token1, but we should check
        let (reserve_a, reserve_b) = (pool_info.token_0_reserve, pool_info.token_1_reserve);

        let mut amount_a = 0;
        let mut amount_b = 0;
        if (pool_info.token_0_reserve == 0 && pool_info.token_1_reserve == 0) {
            amount_a = amount_a_desired;
            amount_b = amount_b_desired;
        } else {
            let amount_b_optional = quote(amount_a_desired, reserve_a, reserve_b);
            if (amount_b_optional <= amount_b_desired) {
                require(amount_b_optional >= amount_b_min, Error::InsufficentBAmount());
                amount_a = amount_a_desired;
                amount_b = amount_b_optional;
            } else {
                let amount_a_optional = quote(amount_b_desired, reserve_b, reserve_a);
                assert(amount_a_optional <= amount_a_desired);
                require(amount_a_optional >= amount_a_min, Error::InsufficentAAmount());
                amount_a = amount_a_optional;
                amount_b = amount_b_desired;
            }
        }

        force_transfer_to_contract(amount_a, ~ContractId::from(token0), ~ContractId::from(pool));
        force_transfer_to_contract(amount_b, ~ContractId::from(token1), ~ContractId::from(pool));

        let liquidity = exchange.add_liquidity(recipient);

        let current_token_0_amount = this_balance(~ContractId::from(token0));
        let current_token_1_amount = this_balance(~ContractId::from(token1));

        if (current_token_0_amount > 0) {
            transfer(amount_a, ~ContractId::from(token0), sender_identity);
        }
        if (current_token_1_amount > 0) {
            transfer(amount_b, ~ContractId::from(token1), sender_identity);
        }

        AddLiquidityOutput {
            amount_a: 0,//amount_a,
            amount_b: 0,//amount_b,
            liquidity: 0,//liquidity,
        }
    }


    fn swap_exact_input(swap_data: SwapData) -> SwapOutput {
        let exchange = abi(Exchange, swap_data.pool);

        // let (token0, token1) = exchange.get_tokens();
        // let pool_info = exchange.pool_info();
        // let (input_reserve, output_reserve) = match msg_asset_id().into() {
        //     token0 => (pool_info.token_0_reserve, pool_info.token_1_reserve),
        //     token1 => (pool_info.token_1_reserve, pool_info.token_0_reserve),
        // };

        // uint amountOut = get_input_price(swap_data.exact_amount, reserve_input, reserve_output);

        let sender_identity = msg_sender().unwrap();

        let output_amount = exchange.swap{
            asset_id: msg_asset_id().into(),
            coins: swap_data.exact_amount,
        }(0, 0, sender_identity);

        // require(output_amount >= swap_data.slippage_amount, Error::InsufficentOutput);

        SwapOutput {
            input_amount: swap_data.exact_amount,
            output_amount: 0,//output_amount,
        }
    }

    fn swap_exact_output(swap_data: SwapData) -> SwapOutput {
        let exchange = abi(Exchange, swap_data.pool);
        let sender_identity = msg_sender().unwrap();
        
        let (token0, token1) = exchange.get_tokens();
        let pool_info = exchange.get_pool_info();
        let mut reserve_input = 0;
        let mut reserve_output = 0;

        if (msg_asset_id().into() == token0) {
            reserve_input = pool_info.token_0_reserve;
            reserve_output = pool_info.token_1_reserve;
        } else if (msg_asset_id().into() == token1) {
            reserve_input = pool_info.token_1_reserve;
            reserve_output = pool_info.token_0_reserve;
        } else {
            revert(0);
        }

        let input_amount = get_output_price(swap_data.exact_amount, reserve_input, reserve_output);

        require(input_amount <= swap_data.slippage_amount, Error::ExcessiveInput);

        let output_amount = exchange.swap{
            asset_id: msg_asset_id().into(),
            coins: input_amount,
        }(0, 0, sender_identity);

        SwapOutput {
            input_amount: input_amount,
            output_amount: swap_data.exact_amount,
            // input_amount: pool_info.token_0_reserve,
            // output_amount: pool_info.token_1_reserve,
        }
        
        // let (token0, token1) = exchange.get_tokens();
        // let pool_info = exchange.pool_info();
        // let (input_reserve, output_reserve) = match msg_asset_id().into() {
        //     token0 => (pool_info.token_0_reserve, pool_info.token_1_reserve),
        //     token1 => (pool_info.token_1_reserve, pool_info.token_0_reserve),
        // };

        // uint amountOut = get_input_price(swap_data.exact_amount, reserve_input, reserve_output);

        // let output_amount = exchange.get_add_liquidity_token_amount(1000);//pool_info.token_0_reserve;//exchange.swap{
        //     asset_id: msg_asset_id().into(),
        //     coins: swap_data.exact_amount,
        // }(msg_asset_id().into(), sender_identity);

        // require(output_amount >= swap_data.slippage_amount, Error::InsufficentOutput);

        // SwapOutput {
        //     input_amount: swap_data.exact_amount,
        //     output_amount: output_amount,
        // }
        // SwapOutput {
        //     input_amount: 100,
        //     output_amount: output_amount,
        // }
        // output_amount
    }
}
