contract;

use std::{
    storage::StorageMap,
    option::Option,
};
use exchange_abi::Exchange;

enum Error {
    UnorderedTokens: (),
}

abi PoolRegistry {
    // Add exchange contract to the token
    #[storage(write)]fn add_exchange_contract(exchange_id: b256);
    // Get exchange contract for desired token
    #[storage(read)]fn get_exchange_contract(token_a: b256, token_b: b256) -> Option<b256>;
}

storage {
    pools: StorageMap<(b256, b256), b256> = StorageMap {},
}

impl PoolRegistry for Contract {
    #[storage(write)]fn add_exchange_contract(exchange_id: b256) {
        let exchange = abi(Exchange, exchange_id);
        let (token0, token1) = exchange.get_tokens();
        require(token0 < token1, Error::UnorderedTokens);

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
