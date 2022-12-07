use core::fmt::Debug;

use fuels::{
  client::types::TransactionStatus,
  contract::contract::{CallResponse, ContractCallHandler},
  prelude::*,
  tx::UniqueIdentifier,
};

pub async fn get_wallets() -> Vec<WalletUnlocked> {
    let num_wallets = 3;
    let num_coins = 1;
    let initial_amount = 10_000_000_000_000;
    let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(initial_amount));

    let wallets = launch_custom_provider_and_get_wallets(config, None, None).await;
    wallets
}

pub async fn get_timestamp_and_call<T>(handler: ContractCallHandler<T>) -> (CallResponse<T>, u64)
where
    T: Tokenizable + Debug,
{
    let script = handler.get_call_execution_script().await.unwrap();
    let tx_id = script.tx.id().to_string();
    let provider = handler.provider.clone();
    let call_response = handler.call().await.unwrap();
    let tx_status = provider.get_transaction_by_id(&tx_id).await.unwrap().status;

    let time = match tx_status {
        TransactionStatus::Success { time, .. } => time,
        _ => panic!("tx failed"),
    };
    let time = time.0;

    (call_response, time)
}
