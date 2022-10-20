contract;

use std::{
    chain::auth::msg_sender,
    context::{
        msg_amount,
        call_frames::msg_asset_id,
    },
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
    vec::*,
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
    InvalidToken: (),
    InvalidInput: (),
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

    fn swap_exact_input(
        pool: b256,
        min_amount_out: u64,
        recipient: Identity,
    ) -> SwapOutput;

    fn swap_exact_output(
        pool: b256,
        amount_out: u64,
        max_amount_in: u64,
        recipient: Identity,
    ) -> SwapOutput;

    fn swap_exact_input_multihop(
        pools: Vec<b256>,
        min_amount_out: u64,
        recipient: Identity,
    ) -> SwapOutput;

    fn swap_exact_output_multihop(
        pools: Vec<b256>,
        amount_out: u64,
        max_amount_in: u64,
        recipient: Identity,
    ) -> SwapOutput;

    // Used to allow easy token deposits, should be removed before prod
    fn null();
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
            amount_a: amount_a,
            amount_b: amount_b,
            liquidity: liquidity,
        }
    }

    fn swap_exact_input(
        pool: b256,
        min_amount_out: u64,
        recipient: Identity,
    ) -> SwapOutput {
        let exchange = abi(Exchange, pool);
        let input_asset: b256 = msg_asset_id().into();

        let (token0, token1) = exchange.get_tokens();
        let pool_info = exchange.get_pool_info();

        require(token0 == input_asset || token1 == input_asset, Error::InvalidToken);

        let (out0, out1) = if token0 == input_asset {
            (0, get_input_price(msg_amount(), pool_info.token_0_reserve, pool_info.token_1_reserve))
        } else {
            (get_input_price(msg_amount(), pool_info.token_1_reserve, pool_info.token_0_reserve), 0)
        };

        exchange.swap{
            asset_id: input_asset,
            coins: msg_amount(),
        }(out0, out1, recipient);

        SwapOutput {
            input_amount: msg_amount(),
            output_amount: if token0 == input_asset { out1 } else { out0 },
        }
    }

    fn swap_exact_output(
        pool: b256,
        amount_out: u64,
        max_amount_in: u64,
        recipient: Identity,
    ) -> SwapOutput {
        let exchange = abi(Exchange, pool);
        let input_asset: b256 = msg_asset_id().into();

        let (token0, token1) = exchange.get_tokens();
        let pool_info = exchange.get_pool_info();

        require(token0 == input_asset || token1 == input_asset, Error::InvalidToken);

        let (input_amount, out0, out1) = if token0 == input_asset {
            (get_output_price(amount_out, pool_info.token_0_reserve, pool_info.token_1_reserve), 0, amount_out)
        } else {
            (get_output_price(amount_out, pool_info.token_1_reserve, pool_info.token_0_reserve), amount_out, 0)
        };

        exchange.swap{
            asset_id: input_asset,
            coins: input_amount,
        }(out0, out1, recipient);

        if (msg_amount() > input_amount) {
            let sender_identity = msg_sender().unwrap();
            transfer(msg_amount() - input_amount, msg_asset_id(), sender_identity);
        }

        SwapOutput {
            input_amount: input_amount,
            output_amount: if token0 == input_asset { out1 } else { out0 },
        }
    }


    fn swap_exact_input_multihop(
        pools: Vec<b256>,
        min_amount_out: u64,
        recipient: Identity,
    ) -> SwapOutput {
        let mut input_asset: b256 = msg_asset_id().into();
        // let mut input_amount = msg_amount();
        let mut output_amount: u64 = msg_amount();

        require(pools.len() > 0, Error::InvalidInput);

        // force_transfer_to_contract(input_amount, msg_asset_id(), ~ContractId::from(pools.get(0).unwrap()));

        let mut i = 0;
        while i < pools.len() {
            let pool_id = pools.get(i).unwrap();
            let exchange = abi(Exchange,pool_id);
            let (token0, token1) = exchange.get_tokens();
            let pool_info = exchange.get_pool_info();

            require(token0 == input_asset || token1 == input_asset, Error::InvalidToken);

            let (out0, out1) = if token0 == input_asset {
                (0, get_input_price(output_amount, pool_info.token_0_reserve, pool_info.token_1_reserve))
            } else {
                (get_input_price(output_amount, pool_info.token_1_reserve, pool_info.token_0_reserve), 0)
            };

            let swap_recipient = if i == pools.len() - 1 {
                recipient
                } else {
                    Identity::ContractId(~ContractId::from(pools.get(i + 1).unwrap()))
                };

            if i == 0 {
                exchange.swap{
                    asset_id: input_asset,
                    coins: msg_amount(),
                }(out0, out1, swap_recipient);
            } else {
                // No need to include assets, the last swap already sent them
                exchange.swap(out0, out1, swap_recipient);
            }

            let (_output_amount, _input_asset) = if token0 == input_asset { (out1, token1) } else { (out0, token0) };
            output_amount = _output_amount;
            input_asset = _input_asset;

            i += 1;
        }

        SwapOutput {
            input_amount: msg_amount(),
            // to delete:
            // Weird variable naming, but typically the input for swap n is the output for swap n - 1
            // Therefore, the final "input" is our output from the whole trade
            output_amount: output_amount,
        }
    }

    fn swap_exact_output_multihop(
        pools: Vec<b256>,
        amount_out: u64,
        max_amount_in: u64,
        recipient: Identity,
    ) -> SwapOutput {
        require(pools.len() > 0, Error::InvalidInput);

        let mut input_assets: Vec<b256> = ~Vec::with_capacity(pools.len());
        input_assets.push(msg_asset_id().into());

        let mut input_amounts: Vec<u64> = ~Vec::with_capacity(pools.len());
        let mut output_amounts: Vec<u64> = ~Vec::with_capacity(pools.len());

        let mut i = 0;
        while i < pools.len() {
            let pool_id = pools.get(i).unwrap();
            let exchange = abi(Exchange, pool_id);
            let (token0, token1) = exchange.get_tokens();
            let input_asset = input_assets.get(i).unwrap();
            require(token0 == input_asset || token1 == input_asset, Error::InvalidToken);
            
            input_assets.push(if input_asset == token0 { token1 } else { token0 });

            // Hacky way to create an empty vector, hopefully there's a method added
            input_amounts.push(0);
            output_amounts.push(0);

            i += 1;
        }
        output_amounts.set(pools.len() - 1, amount_out);

        i = pools.len();
        while i > 0 {
            let j = i - 1;
            // TODO: should we cache this in memory since it's read again in the next loop?
            let pool_id = pools.get(j).unwrap();
            let input_asset = input_assets.get(j).unwrap();
            let exchange = abi(Exchange, pool_id);
            let (token0, token1) = exchange.get_tokens();
            let pool_info = exchange.get_pool_info();
            let pool_output_amount = output_amounts.get(j).unwrap();

            let input_amount = if token0 == input_asset {
                get_output_price(pool_output_amount, pool_info.token_0_reserve, pool_info.token_1_reserve)
            } else {
                get_output_price(pool_output_amount, pool_info.token_1_reserve, pool_info.token_0_reserve)
            };

            input_amounts.set(j, input_amount);
            if (j > 0) {
                output_amounts.set(j - 1, input_amount);
            }

            i -= 1;
        }

        i = 0;
        while i < pools.len() {
            let pool_id = pools.get(i).unwrap();
            let exchange = abi(Exchange,pool_id);
            let (token0, token1) = exchange.get_tokens();
            let pool_info = exchange.get_pool_info();

            let input_asset = input_assets.get(i).unwrap();
            let input_amount = input_amounts.get(i).unwrap();
            let output_amount = output_amounts.get(i).unwrap();

            let (out0, out1) = if token0 == input_asset { (0, output_amount) } else { (output_amount, 0) };

            let swap_recipient = if i == pools.len() - 1 {
                    recipient
                } else {
                    Identity::ContractId(~ContractId::from(pools.get(i + 1).unwrap()))
                };

            if i == 0 {
                exchange.swap{
                    asset_id: input_asset,
                    coins: input_amount,
                }(out0, out1, swap_recipient);

                if (msg_amount() > input_amount) {
                    let sender_identity = msg_sender().unwrap();
                    transfer(msg_amount() - input_amount, msg_asset_id(), sender_identity);
                }
            } else {
                // No need to include assets, the last swap already sent them
                exchange.swap(out0, out1, swap_recipient);
            }

            i += 1;
        }

        SwapOutput {
            input_amount: input_amounts.get(0).unwrap(),
            output_amount: amount_out,
        }
    }

    fn null() {}
}
