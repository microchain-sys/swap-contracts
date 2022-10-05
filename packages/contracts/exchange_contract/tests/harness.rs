use std::{vec, str::FromStr};
use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    signers::WalletUnlocked,
    tx::{AssetId, ContractId, Bytes32, StorageSlot},
};

///////////////////////////////
// Load the Exchange Contract abi
///////////////////////////////
abigen!(TestExchange, "out/debug/exchange_contract-abi.json");

///////////////////////////////
// Load the Token Contract abi
///////////////////////////////T
abigen!(
    TestToken,
    "../token_contract/out/debug/token_contract-abi.json"
);

async fn deposit_and_add_liquidity(
    wallet: &WalletUnlocked,
    exchange_instance: &TestExchange,
    exchange_contract_id: &Bech32ContractId,
    native_amount: u64,
    token_asset_id: AssetId,
    token_amount_deposit: u64,
) -> u64 {
    // Deposit some Native Asset
    let _receipts = wallet
        .force_transfer_to_contract(
            &exchange_contract_id,
            native_amount,
            BASE_ASSET_ID,
            TxParameters::default()
        )
        .await;

    // Deposit some Token Asset
    let _receipts = wallet
        .force_transfer_to_contract(
            &exchange_contract_id,
            token_amount_deposit,
            token_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    // Add liquidity for the second time. Keeping the proportion 1:2
    // It should return the same amount of LP as the amount of ETH deposited
    let result = exchange_instance
        .methods()
        .add_liquidity(1, Identity::Address(wallet.address().into()))
        .append_variable_outputs(2)
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

    result.value
}

#[tokio::test]
async fn exchange_contract() {
    // default initial amount 1000000000
    let wallet = launch_provider_and_get_wallet().await;
    // Wallet address
    let address = wallet.address();

    //////////////////////////////////////////
    // Setup contracts
    //////////////////////////////////////////

    let token_contract_id = Contract::deploy(
        "../token_contract/out/debug/token_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let key = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();
    let value = token_contract_id.hash();
    let storage_slot = StorageSlot::new(key, value);
    let storage_vec = vec![storage_slot.clone()];

    // Deploy contract and get ID
    let exchange_contract_id = Contract::deploy(
        "out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
    )
    .await
    .unwrap();

    let exchange_instance = TestExchange::new(exchange_contract_id.to_string(), wallet.clone());
    let token_instance = TestToken::new(token_contract_id.to_string(), wallet.clone());

    // There must be an easier way than this double-cast, no?
    let token_contract_id_casted: ContractId = token_contract_id.clone().into();
    let token_casted_again: [u8; 32] = token_contract_id_casted.clone().into();
    let token_asset_id = AssetId::from(token_casted_again);
    // LP Token asset id
    // There must be an easier way than this double-cast, no?
    let exchange_contract_id_casted: ContractId = exchange_contract_id.clone().into();
    let exchange_casted_again: [u8; 32] = exchange_contract_id_casted.clone().into();
    let lp_asset_id = AssetId::from(exchange_casted_again);

    ////////////////////////////////////////////////////////
    // Mint some tokens to the wallet
    ////////////////////////////////////////////////////////

    // Get the contract ID and a handle to it
    let wallet_token_amount = 20000;

    // Initialize token contract
    token_instance
        .methods()
        .initialize(wallet_token_amount, address.into())
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

    // Total amount of native amounts
    // send to the wallet
    let native_amount = 100;

    ////////////////////////////////////////////////////////
    // Deposit tokens and create pool
    ////////////////////////////////////////////////////////

    let native_amount_deposit = native_amount;
    let token_amount_deposit = 200;
    // Check user position
    let lp_amount_received = deposit_and_add_liquidity(
        &wallet,
        &exchange_instance,
        &exchange_contract_id,
        native_amount_deposit,
        token_asset_id,
        token_amount_deposit,
    )
    .await;
    assert_eq!(lp_amount_received, native_amount);

    ////////////////////////////////////////////////////////
    // Remove liquidity and receive assets back
    ////////////////////////////////////////////////////////

    // Remove LP tokens from liquidity it should keep proportion 1:2
    // It should return the exact amount added on the add liquidity
    let result = exchange_instance
        .methods()
        .remove_liquidity(1, 1, Identity::Address(address.into()))
        .call_params(CallParameters::new(
            Some(lp_amount_received),
            Some(lp_asset_id.clone()),
            Some(100_000_000),
        ))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(2)
        .call()
        .await
        .unwrap();
    assert_eq!(result.value.token_0_amount, native_amount_deposit);
    assert_eq!(result.value.token_1_amount, token_amount_deposit);

    ////////////////////////////////////////////////////////
    // Setup the pool
    ////////////////////////////////////////////////////////

    // Check user position
    let _t = deposit_and_add_liquidity(
        &wallet,
        &exchange_instance,
        &exchange_contract_id,
        native_amount_deposit,
        token_asset_id,
        token_amount_deposit,
    )
    .await;

    ////////////////////////////////////////////////////////
    // Amounts
    ////////////////////////////////////////////////////////

    // Swap amount
    let amount: u64 = 10;
    // Amount used on a second add_liquidity
    let eth_to_add_liquidity_amount: u64 = 100;
    // Final balance of LP tokens
    let expected_final_lp_amount: u64 = 199;
    // Final eth amount removed from the Pool
    let remove_liquidity_eth_amount: u64 = 201;
    // Final token amount removed from the Pool
    let remove_liquidity_token_amount: u64 = 388;

    ////////////////////////////////////////////////////////
    // SWAP WITH MINIMUM (ETH -> TOKEN)
    ////////////////////////////////////////////////////////

    // Get expected swap amount ETH -> TOKEN
    let amount_expected = exchange_instance
        .methods()
        .get_swap_with_minimum(amount)
        .call()
        .await
        .unwrap();
    assert!(amount_expected.value.has_liquidity);
    // Swap using expected amount ETH -> TOKEN
    let response = exchange_instance
        .methods()
        .swap(Bits256(BASE_ASSET_ID.into()), Identity::Address(address.into()))
        // .swap_with_minimum(BASE_ASSET_ID.into(), amount_expected.value.amount, Identity::Address(address.into()))
        .call_params(CallParameters::new(Some(amount), None, None))
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
    assert_eq!(response.value, amount_expected.value.amount);

    let pool_info = exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_0_reserve, 110);
    assert_eq!(pool_info.value.token_1_reserve, 182);

    ////////////////////////////////////////////////////////
    // SWAP WITH MINIMUM (TOKEN -> ETH)
    ////////////////////////////////////////////////////////

    // Get expected swap amount TOKEN -> ETH
    let amount_expected = exchange_instance
        .methods()
        .get_swap_with_minimum(amount)
        .call_params(CallParameters::new(Some(0), Some(token_asset_id.clone()), None))
        .call()
        .await
        .unwrap();
    assert!(amount_expected.value.has_liquidity);
    // Swap using expected amount TOKEN -> ETH
    let response = exchange_instance
        .methods()
        .swap(Bits256(token_asset_id.into()), Identity::Address(address.into()))
        // .swap_with_minimum(token_asset_id.into(), amount_expected.value.amount, Identity::Address(address.into()))
        .call_params(CallParameters::new(
            Some(amount),
            Some(token_asset_id.clone()),
            None,
        ))
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
    assert_eq!(response.value, amount_expected.value.amount);

    ////////////////////////////////////////////////////////
    // SWAP WITH MAXIMUM EXPECT ERRORS (ETH -> TOKEN)
    ////////////////////////////////////////////////////////

    // Should throw error
    // If the output is bigger them the reserve
    let is_err = exchange_instance
        .methods()
        .get_swap_with_maximum(1000)
        .call()
        .await
        .is_err();
    assert!(is_err);

    ////////////////////////////////////////////////////////
    // SWAP WITH MAXIMUM EXPECT ERRORS (TOKEN -> ETH)
    ////////////////////////////////////////////////////////

    // Should return u64::MAX
    // If the output is equal to the reserve
    let is_err = exchange_instance
        .methods()
        .get_swap_with_maximum(token_amount_deposit + 1)
        .call()
        .await
        .is_err();
    assert!(is_err);

    ////////////////////////////////////////////////////////
    // SWAP WITH MAXIMUM (ETH -> TOKEN)
    ////////////////////////////////////////////////////////

    // Get expected swap amount ETH -> TOKEN
    let amount_expected = exchange_instance
        .methods()
        .get_swap_with_maximum(amount)
        .call()
        .await
        .unwrap();
    assert!(amount_expected.value.has_liquidity);
    // Swap using expected amount ETH -> TOKEN
    let response = exchange_instance
        .methods()
        // .swap(BASE_ASSET_ID.into(), Identity::Address(address.into()))
        .swap_with_maximum(Bits256(BASE_ASSET_ID.into()), amount, Identity::Address(address.into()))
        .call_params(CallParameters::new(
            Some(amount_expected.value.amount),
            None,
            None,
        ))
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
    assert_eq!(response.value, amount_expected.value.amount);

    ////////////////////////////////////////////////////////
    // SWAP WITH MAXIMUM (TOKEN -> ETH)
    ////////////////////////////////////////////////////////

    // Get expected swap amount TOKEN -> ETH
    let amount_expected = exchange_instance
        .methods()
        .get_swap_with_maximum(amount)
        .call_params(CallParameters::new(None, Some(token_asset_id.clone()), None))
        .call()
        .await
        .unwrap();
    assert!(amount_expected.value.has_liquidity);
    // Swap using expected amount TOKEN -> ETH
    let response = exchange_instance
        .methods()
        // .swap(token_asset_id.into(), Identity::Address(address.into()))
        .swap_with_maximum(Bits256(token_asset_id.into()), amount, Identity::Address(address.into()))
        .call_params(CallParameters::new(
            Some(amount_expected.value.amount),
            Some(token_asset_id.clone()),
            None,
        ))
        .append_variable_outputs(2)
        .call()
        .await
        .unwrap();
    assert_eq!(response.value, amount_expected.value.amount);

    ////////////////////////////////////////////////////////
    // Add more liquidity to the contract
    ////////////////////////////////////////////////////////

    let token_amount_required = exchange_instance
        .methods()
        .get_add_liquidity_token_amount(eth_to_add_liquidity_amount)
        .simulate()
        .await
        .unwrap();
    let lp_amount_received = deposit_and_add_liquidity(
        &wallet,
        &exchange_instance,
        &exchange_contract_id,
        native_amount_deposit,
        token_asset_id,
        token_amount_required.value,
    )
    .await
        + lp_amount_received;
    // The amount of tokens returned should be smaller
    // as swaps already happen
    assert_eq!(lp_amount_received, expected_final_lp_amount);

    ////////////////////////////////////////////////////////
    // Remove liquidity and receive assets back
    ////////////////////////////////////////////////////////

    let response = exchange_instance
        .methods()
        .remove_liquidity(1, 1, Identity::Address(address.into()))
        .call_params(CallParameters::new(
            Some(lp_amount_received),
            Some(lp_asset_id.clone()),
            Some(100_000_000),
        ))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(2)
        .call()
        .await
        .unwrap();
    assert_eq!(response.value.token_0_amount, remove_liquidity_eth_amount);
    assert_eq!(response.value.token_1_amount, remove_liquidity_token_amount);

    ////////////////////////////////////////////////////////
    // Check contract pool is zero
    ////////////////////////////////////////////////////////

    let pool_info = exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_0_reserve, 0);
    assert_eq!(pool_info.value.token_1_reserve, 0);
    assert_eq!(pool_info.value.lp_token_supply, 0);
}
