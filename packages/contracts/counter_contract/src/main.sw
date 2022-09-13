contract;

use std::{
    logging::log,
};


abi SwapSwap {
    #[storage(read, write)]fn increment();

    #[storage(read, write)]fn increase(amount: u64);

    #[storage(read)]fn counter() -> u64;
}

storage {
    count: u64 = 0,
}

struct Increased {
    amount: u64,
    new_count: u64,
}

impl SwapSwap for Contract {
    #[storage(read, write)]fn increment() {
        let current_count = storage.count;

        storage.count = current_count + 1;

        log(Increased {
            amount: 1,
            new_count: current_count + 1,
        });
    }

    #[storage(read, write)]fn increase(amount: u64) {
        let current_count = storage.count;

        storage.count = current_count + amount;

        log(Increased {
            amount: amount,
            new_count: current_count + amount,
        });
    }

    #[storage(read)]fn counter() -> u64 {
        storage.count
    }
}
