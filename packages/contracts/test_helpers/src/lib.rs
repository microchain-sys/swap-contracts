use core::fmt::Debug;

use fuels::{
  contract::{
    call_response::FuelCallResponse,
    contract::ContractCallHandler,
  },
  fuel_node::Config,
  prelude::*,
//   tx::UniqueIdentifier,
};

static mut TIMESTAMP: u64 = 1;

pub async fn get_wallets() -> Vec<WalletUnlocked> {
    let num_wallets = 3;
    let num_coins = 1;
    let initial_amount = 10_000_000_000_000;
    let wallets_config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(initial_amount));

    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };

    let wallets = launch_custom_provider_and_get_wallets(wallets_config, Some(config), None).await;

    wallets
}

pub async fn get_timestamp_and_call<T>(handler: ContractCallHandler<T>) -> (FuelCallResponse<T>, u64)
where
    T: Tokenizable + Debug,
{
    // let script = handler.get_executable_call().await.unwrap();
    // let tx_id = script.tx.id().to_string();
    // let provider = handler.provider.clone();
    let call_response = handler.call().await.unwrap();

    // Looks like timestamps are broken?
    // https://github.com/FuelLabs/sway-applications/blob/master/name-registry/project/registry-contract/tests/utils/abi.rs#L11
    // For now, we'll fake it
    // TODO: use actual block times

    let time: u64;

    unsafe {
        TIMESTAMP += 1;
        time = TIMESTAMP;
    }

    (call_response, time)
}
