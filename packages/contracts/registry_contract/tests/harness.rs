use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    tx::{Bytes32, StorageSlot},
};

use std::str::FromStr;

///////////////////////////////
// Load the SwaySwap Contract abi
///////////////////////////////
abigen!(TestRegistryBuilder, "out/debug/registry_contract-abi.json");

abigen!(TestExchange, "../exchange_contract/out/debug/exchange_contract-abi.json");


#[tokio::test]
async fn register_exchange() {
    // Provider and Wallet
    let wallet = launch_provider_and_get_wallet().await;

    // Get the contract ID and a handle to it
    let registry_contract_id = Contract::deploy(
        "out/debug/registry_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let registry_instance = TestRegistryBuilder::new(
        registry_contract_id.to_string(),
        wallet.clone(),
    );

    let token0_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let token1_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();    

    // Create fake token ids
    let token_id_1 = Bytes32::from_str("0x000005877b940cc69d7a9a71000a0cfdd79e93f783f198de893165278712a480").unwrap();
    let token_id_2 = Bytes32::from_str("0x716c345b96f3c17234c73881c40df43d3d492b902a01a062c12e92eeae0284e9").unwrap();
    let token_id_nonexistent = Bytes32::from_str("0xdf43d3d492b90716c345b96f3c17234c73881e92eeae0284e9c402a01a062c12").unwrap();

    let storage_vec = vec![
        StorageSlot::new(token0_slot, token_id_1),
        StorageSlot::new(token1_slot, token_id_2),
    ];

    // Deploy contract and get ID
    let exchange_contract_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    // Test storage
    registry_instance
        .methods()
        .add_exchange_contract(Bits256(exchange_contract_id.hash().into()))
        .set_contracts(&[exchange_contract_id.clone()])
        .call()
        .await
        .unwrap();

    // Test retrieval (normal)
    let result = registry_instance
        .methods()
        .get_exchange_contract(Bits256(token_id_1.into()), Bits256(token_id_2.into()))
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, Some(Bits256(exchange_contract_id.hash().into())));

    // Test retrieval (reversed order)
    let result = registry_instance
        .methods()
        .get_exchange_contract(Bits256(token_id_2.into()), Bits256(token_id_1.into()))
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, Some(Bits256(exchange_contract_id.hash().into())));

    // Test retrieval (non-existent)
    let result = registry_instance
        .methods()
        .get_exchange_contract(Bits256(token_id_1.into()), Bits256(token_id_nonexistent.into()))
        .call()
        .await
        .unwrap();
    assert_eq!(result.value, None);
}


#[tokio::test]
async fn unordered_tokens_should_fail() {
    // Provider and Wallet
    let wallet = launch_provider_and_get_wallet().await;

    // Get the contract ID and a handle to it
    let registry_contract_id = Contract::deploy(
        "out/debug/registry_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let registry_instance = TestRegistryBuilder::new(
        registry_contract_id.to_string(),
        wallet.clone(),
    );

    let token0_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let token1_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();    

    // Create fake token ids
    let token_id_1 = Bytes32::from_str("0x000005877b940cc69d7a9a71000a0cfdd79e93f783f198de893165278712a480").unwrap();
    let token_id_2 = Bytes32::from_str("0x716c345b96f3c17234c73881c40df43d3d492b902a01a062c12e92eeae0284e9").unwrap();

    let storage_vec = vec![
        StorageSlot::new(token0_slot, token_id_2),
        StorageSlot::new(token1_slot, token_id_1),
    ];

    // Deploy contract and get ID
    let exchange_contract_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    // Test storage
    let is_err = registry_instance
        .methods()
        .add_exchange_contract(Bits256(exchange_contract_id.hash().into()))
        .set_contracts(&[exchange_contract_id.clone()])
        .call()
        .await
        .is_err();
    assert!(is_err);
}
