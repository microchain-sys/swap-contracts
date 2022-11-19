use core::fmt::Debug;

use fuels::{
  client::types::TransactionStatus,
  contract::contract::{CallResponse, ContractCallHandler},
  prelude::*,
};

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
    let time = time.timestamp() as u64;

    (call_response, time)
}
