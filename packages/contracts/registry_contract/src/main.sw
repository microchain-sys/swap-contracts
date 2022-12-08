contract;

use std::{
    constants::ZERO_B256,
    contract_id::ContractId,
    external::bytecode_root,
    option::Option,
    storage::StorageMap,
};
use core::num::*;
use exchange_abi::Exchange;

enum Error {
    UnorderedTokens: (),
    AlreadyInitialized: (),
    AlreadyRegistered: (),
    InvalidContractCode: (),
    PoolInitialized: (),
}

abi PoolRegistry {
    #[storage(write, read)]
    fn initialize(template_exchange_id: b256);
    // Add exchange contract to the token
    #[storage(write, read)]
    fn add_exchange_contract(exchange_id: b256);
    // Get exchange contract for desired token
    #[storage(read)]
    fn get_exchange_contract(token_a: b256, token_b: b256) -> Option<b256>;
    #[storage(read)]
    fn is_pool(addr: b256) -> bool;
    #[storage(read)]
    fn exchange_contract_root() -> b256;
}

storage {
    expected_contract_root: b256 = ZERO_B256,
    pools: StorageMap<(b256, b256), b256> = StorageMap {},
    is_pool: StorageMap<b256, bool> = StorageMap {},
}

impl PoolRegistry for Contract {
    #[storage(write, read)]
    fn initialize(template_exchange_id: b256) {
        require(storage.expected_contract_root == ZERO_B256, Error::AlreadyInitialized);
        let root = bytecode_root(ContractId::from(template_exchange_id));
        storage.expected_contract_root = root;
    }

    #[storage(write, read)]
    fn add_exchange_contract(exchange_id: b256) {
        let exchange = abi(Exchange, exchange_id);

        let root = bytecode_root(ContractId::from(exchange_id));
        require(root == storage.expected_contract_root, Error::InvalidContractCode);

        let (token0, token1) = exchange.get_tokens();
        require(token0 < token1, Error::UnorderedTokens);

        let existing_exchange = storage.pools.get((token0, token1));
        require(existing_exchange == b256::min(), Error::AlreadyRegistered);

        let pool_info = exchange.get_pool_info();
        require(pool_info.lp_token_supply == 0, Error::PoolInitialized);

        storage.pools.insert((token0, token1), exchange_id);
        storage.is_pool.insert(exchange_id, true);
    }

    #[storage(read)]
    fn get_exchange_contract(token_a: b256, token_b: b256) -> Option<b256> {
        let (token0, token1) = if token_a < token_b {
            (token_a, token_b)
        } else {
            (token_b, token_a)
        };
        let exchange = storage.pools.get((token0, token1));

        if (exchange == b256::min()) {
            Option::None
        } else {
            Option::Some(exchange)
        }
    }

    #[storage(read)]
    fn is_pool(addr: b256) -> bool {
        storage.is_pool.get(addr)
    }

    #[storage(read)]
    fn exchange_contract_root() -> b256 {
        storage.expected_contract_root
    }
}
