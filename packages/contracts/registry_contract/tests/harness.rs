use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    tx::{Bytes32, StorageSlot},
};

use std::str::FromStr;

///////////////////////////////
// Load the SwaySwap Contract abi
///////////////////////////////
abigen!(RegistryBuilder, "out/debug/registry_contract-abi.json");

abigen!(Exchange, "../exchange_contract/out/debug/exchange_contract-abi.json");

abigen!(
    TestToken,
    "../token_contract/out/debug/token_contract-abi.json"
);

const ZERO_B256: Bits256 = Bits256([0; 32]);

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

    let registry_instance = RegistryBuilder::new(
        registry_contract_id,
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

    let root = registry_instance.methods().exchange_contract_root().simulate().await.unwrap();
    assert_eq!(root.value, ZERO_B256, "Registry should be uninitialized");

    registry_instance
        .methods()
        .initialize(Bits256(exchange_contract_id.hash().into()))
        .call()
        .await
        .unwrap();

    let root = registry_instance.methods().exchange_contract_root().simulate().await.unwrap();
    assert_ne!(root.value, ZERO_B256, "Registry should be initialized");

    let result = registry_instance
        .methods()
        .is_pool(Bits256(exchange_contract_id.hash().into()))
        .call()
        .await
        .unwrap();
    assert!(!result.value, "is_pool shouldn't return true");

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

    let result = registry_instance
        .methods()
        .is_pool(Bits256(exchange_contract_id.hash().into()))
        .call()
        .await
        .unwrap();
    assert!(result.value, "is_pool should return true");
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

    let registry_instance = RegistryBuilder::new(registry_contract_id, wallet.clone());

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

    registry_instance
        .methods()
        .initialize(Bits256(exchange_contract_id.hash().into()))
        .call()
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

#[tokio::test]
async fn test_invalid_contract() {
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

    let registry_instance = RegistryBuilder::new(registry_contract_id, wallet.clone());

    let token0_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let token1_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();    

    // Create fake token ids
    let token_id_1 = Bytes32::from_str("0x000005877b940cc69d7a9a71000a0cfdd79e93f783f198de893165278712a480").unwrap();
    let token_id_2 = Bytes32::from_str("0x716c345b96f3c17234c73881c40df43d3d492b902a01a062c12e92eeae0284e9").unwrap();

    let storage_vec = vec![
        StorageSlot::new(token0_slot, token_id_1),
        StorageSlot::new(token1_slot, token_id_2),
    ];

    let valid_exchange_contract_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec.clone())),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    // Note: we're using a different binary here, so the code hash won't match the first exchange
    let invalid_exchange_contract_id = Contract::deploy_with_parameters(
        "./tests/modified_exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    registry_instance
        .methods()
        .initialize(Bits256(valid_exchange_contract_id.hash().into()))
        .call()
        .await
        .unwrap();

    // Test storage
    let is_err = registry_instance
        .methods()
        .add_exchange_contract(Bits256(invalid_exchange_contract_id.hash().into()))
        .set_contracts(&[invalid_exchange_contract_id.clone()])
        .call()
        .await
        .is_err();
    assert!(is_err);
}

#[tokio::test]
async fn initialized_pools_should_fail() {
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

    let registry_instance = RegistryBuilder::new(registry_contract_id, wallet.clone());

    let token_contract_id = Contract::deploy(
        "../token_contract/out/debug/token_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let token_instance = TestToken::new(token_contract_id.clone(), wallet.clone());

    // Initialize token contract
    token_instance
        .methods()
        .initialize(100000, wallet.address().into())
        .call()
        .await
        .unwrap();

    // Mint some alt tokens
    token_instance
        .methods()
        .mint()
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    let key = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();
    let storage_slot = StorageSlot::new(key, token_contract_id.hash());

    // Deploy contract and get ID
    let exchange_contract_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(vec![storage_slot.clone()])),
        Salt::from([3u8; 32]),
    )
    .await
    .unwrap();

    let exchange_instance = Exchange::new(exchange_contract_id.clone(), wallet.clone());

    // Add Liquidity

    let _receipts = wallet
        .force_transfer_to_contract(
            &exchange_contract_id,
            2000,
            BASE_ASSET_ID,
            TxParameters::default()
        )
        .await;

    // Deposit some Token Asset
    let _receipts = wallet
        .force_transfer_to_contract(
            &exchange_contract_id,
            2000,
            AssetId::new(*token_contract_id.hash()),
            TxParameters::default()
        )
        .await;

    exchange_instance
        .methods()
        .add_liquidity(Identity::Address(wallet.address().into()))
        .append_variable_outputs(3)
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            None,
            None,
            Some(100_000_000),
        ))
        .call()
        .await
        .unwrap();


    //////

    registry_instance
        .methods()
        .initialize(Bits256(exchange_contract_id.hash().into()))
        .call()
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

#[tokio::test]
async fn duplicate_pools_should_fail() {
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

    let registry_instance = RegistryBuilder::new(registry_contract_id, wallet.clone());

    let token0_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let token1_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    // Create fake token ids
    let token_id_1 = Bytes32::from_str("0x000005877b940cc69d7a9a71000a0cfdd79e93f783f198de893165278712a480").unwrap();
    let token_id_2 = Bytes32::from_str("0x716c345b96f3c17234c73881c40df43d3d492b902a01a062c12e92eeae0284e9").unwrap();

    let storage_vec = vec![
        StorageSlot::new(token0_slot, token_id_1),
        StorageSlot::new(token1_slot, token_id_2),
    ];

    // Deploy contract and get ID
    let exchange_contract_1_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec.clone())),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    // Deploy contract and get ID
    let exchange_contract_2_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([2u8; 32]),
    )
    .await
    .unwrap();

    registry_instance
        .methods()
        .initialize(Bits256(exchange_contract_1_id.hash().into()))
        .call()
        .await
        .unwrap();

    registry_instance
        .methods()
        .add_exchange_contract(Bits256(exchange_contract_1_id.hash().into()))
        .set_contracts(&[exchange_contract_1_id.clone()])
        .call()
        .await
        .unwrap();

    let is_err = registry_instance
        .methods()
        .add_exchange_contract(Bits256(exchange_contract_2_id.hash().into()))
        .set_contracts(&[exchange_contract_2_id.clone()])
        .call()
        .await
        .is_err();
    assert!(is_err);
}
