contract;

use std::{
    auth::{msg_sender, AuthError},
    block::timestamp,
    call_frames::contract_id,
    constants::ZERO_B256,
    contract_id::ContractId,
    identity::Identity,
};

use exchange_abi::{Exchange};
use vault_abi::{Vault, VaultFee};

enum Error {
    MustBeCalledByOwner: (),
}


////////////////////////////////////////
// Helper functions
////////////////////////////////////////

struct StoredFees {
    start_time: u32,
    start_fee: u16,
    change_rate: u16,
}


storage {
    owner: Identity = Identity::Address(Address::from(ZERO_B256)),
    fees: StoredFees = StoredFees {
        start_time: 0,
        start_fee: 0,
        change_rate: 0,
    },
}

impl Vault for Contract {
    #[storage(read)]fn get_fees() -> VaultFee {
        let fees = storage.fees;
        let decrease_since_change = fees.change_rate * (timestamp() - fees.start_time);
        let current_fee = if decrease_since_change > fees.start_fee {
            0
        } else {
            fees.start_fee - decrease_since_change
        };

        VaultFee {
            start_time: fees.start_time,
            start_fee: fees.start_fee,
            current_fee: current_fee,
            change_rate: fees.change_rate,
        }
    }

    #[storage(read, write)]fn set_fees(start_fee: u16, change_rate: u16) {
        let sender: Result<Identity, AuthError> = msg_sender();

        let owner = storage.owner;
        require(
            sender.unwrap() == owner || owner == Identity::Address(Address::from(ZERO_B256)),
            Error::MustBeCalledByOwner
        );
        if (owner == Identity::Address(Address::from(ZERO_B256))) {
            storage.owner = sender.unwrap();
        }

        storage.fees = StoredFees {
            start_time: timestamp(),
            start_fee: start_fee,
            change_rate: change_rate,
        }
    }

    // Note: can call withdraw_protocol_fees on any contract, but there's no vulnerability to the vault
    fn claim_fees(pool: b256) {
        let exchange = abi(Exchange, pool);
        exchange.withdraw_protocol_fees(Identity::ContractId(contract_id()));
    }
}
