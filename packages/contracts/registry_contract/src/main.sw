contract;

use std::{
    storage::StorageMap,
    option::Option,
};
use core::num::*;
use exchange_abi::Exchange;

enum Error {
    UnorderedTokens: (),
    AlreadyInitialized: (),
    InvalidContractCode: (),
    PoolInitialized: (),
}

abi PoolRegistry {
    #[storage(write, read)]fn initialize(template_exchange_id: b256);
    // Add exchange contract to the token
    #[storage(write, read)]fn add_exchange_contract(exchange_id: b256);
    // Get exchange contract for desired token
    #[storage(read)]fn get_exchange_contract(token_a: b256, token_b: b256) -> Option<b256>;
}

fn get_contract_root(addr: b256) -> b256 {
    let mut result_buffer: b256 = ~b256::min();
    asm(hash: result_buffer, addr: addr) {
        croo hash addr;
        hash: b256 // Return
    }
}

storage {
    expected_contract_root: b256 = ~b256::min(),
    pools: StorageMap<(b256, b256), b256> = StorageMap {},
}

impl PoolRegistry for Contract {
    #[storage(write, read)]fn initialize(template_exchange_id: b256) {
        require(storage.expected_contract_root == ~b256::min(), Error::AlreadyInitialized);
        let root = get_contract_root(template_exchange_id);
        storage.expected_contract_root = root;
    }

    #[storage(write, read)]fn add_exchange_contract(exchange_id: b256) {
        let exchange = abi(Exchange, exchange_id);

        let root = get_contract_root(exchange_id);
        require(root == storage.expected_contract_root, Error::InvalidContractCode);

        let (token0, token1) = exchange.get_tokens();
        require(token0 < token1, Error::UnorderedTokens);

        let pool_info = exchange.get_pool_info();
        require(pool_info.lp_token_supply == 0, Error::PoolInitialized);

        storage.pools.insert((token0, token1), exchange_id);
    }

    #[storage(read)]fn get_exchange_contract(token_a: b256, token_b: b256) -> Option<b256> {
        let (token0, token1) = if token_a < token_b { (token_a, token_b) } else { (token_b, token_a) };
        let exchange = storage.pools.get((token0, token1));

        if (exchange == 0x0000000000000000000000000000000000000000000000000000000000000000) {
            Option::None
        } else {
            Option::Some(exchange)
        }
    }
}
