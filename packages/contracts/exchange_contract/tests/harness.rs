use std::{
    vec,
    str::FromStr,
};
use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    signers::WalletUnlocked,
    tx::{AssetId, Bytes32, StorageSlot},
};
use tokio::time::{sleep, Duration};

///////////////////////////////
// Load the Exchange Contract abi
///////////////////////////////
abigen!(TestExchange, "./out/debug/exchange_contract-abi.json");

///////////////////////////////
// Load the Token Contract abi
///////////////////////////////
abigen!(
    TestToken,
    "../token_contract/out/debug/token_contract-abi.json"
);

///////////////////////////////
// Load the Vault Contract abi
///////////////////////////////
abigen!(
    TestVault,
    "../vault_contract/out/debug/vault_contract-abi.json"
);

struct Fixture {
    wallet: WalletUnlocked,
    token_contract_id: Bech32ContractId,
    exchange_contract_id: Bech32ContractId,
    vault_contract_id: Bech32ContractId,
    token_asset_id: AssetId,
    exchange_asset_id: AssetId,
    token_instance: TestToken,
    exchange_instance: TestExchange,
    vault_instance: TestVault,
}

fn to_9_decimal(num: u64) -> u64 {
    num * 1_000_000_000
}

async fn setup() -> Fixture {
    let num_wallets = 1;
    let num_coins = 1;
    let amount = to_9_decimal(10000);
    let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));
  
    let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet = wallets.pop().unwrap();
    // let wallet = launch_provider_and_get_wallet().await;

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
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
    )
    .await
    .unwrap();

    let vault_contract_id = Contract::deploy(
        "../vault_contract/out/debug/vault_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let exchange_instance = TestExchange::new(exchange_contract_id.to_string(), wallet.clone());
    let token_instance = TestToken::new(token_contract_id.to_string(), wallet.clone());
    let vault_instance = TestVault::new(vault_contract_id.to_string(), wallet.clone());

    let wallet_token_amount = to_9_decimal(20000);

    // Initialize token contract
    token_instance
        .methods()
        .initialize(wallet_token_amount, wallet.address().into())
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

    exchange_instance
        .methods()
        .initialize(Bits256(vault_contract_id.hash().into()))
        .set_contracts(&[vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    Fixture {
        wallet: wallet,
        token_contract_id: token_contract_id.clone(),
        exchange_contract_id: exchange_contract_id.clone(),
        vault_contract_id: vault_contract_id,
        token_asset_id: AssetId::new(*token_contract_id.hash()),
        exchange_asset_id: AssetId::new(*exchange_contract_id.hash()),
        token_instance: token_instance,
        exchange_instance: exchange_instance,
        vault_instance: vault_instance,
    }
}

async fn add_liquidity(fixture: &Fixture, token_0_amount: u64, token_1_amount: u64) {
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_contract_id,
            token_0_amount,
            BASE_ASSET_ID,
            TxParameters::default()
        )
        .await;

    // Deposit some Token Asset
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_contract_id,
            token_1_amount,
            fixture.token_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    let result = fixture.exchange_instance
        .methods()
        .add_liquidity(Identity::Address(fixture.wallet.address().into()))
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
}

#[tokio::test]
async fn mint() {
    let fixture = setup().await;

    let token_0_amount = to_9_decimal(1);
    let token_1_amount = to_9_decimal(4);

    let expected_liquidity = to_9_decimal(2);

    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_contract_id,
            token_0_amount,
            BASE_ASSET_ID,
            TxParameters::default()
        )
        .await;

    // Deposit some Token Asset
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_contract_id,
            token_1_amount,
            fixture.token_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    // Add liquidity for the second time. Keeping the proportion 1:2
    // It should return the same amount of LP as the amount of ETH deposited
    let result = fixture.exchange_instance
        .methods()
        .add_liquidity(Identity::Address(fixture.wallet.address().into()))
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

    assert_eq!(result.value, expected_liquidity - 1000);

    // expect(await pair.balanceOf(wallet.address)).to.eq(expectedLiquidity.sub(MINIMUM_LIQUIDITY))
    // expect(await token0.balanceOf(pair.address)).to.eq(token0Amount)
    // expect(await token1.balanceOf(pair.address)).to.eq(token1Amount)

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_0_reserve, token_0_amount);
    assert_eq!(pool_info.value.token_1_reserve, token_1_amount);
    assert_eq!(pool_info.value.lp_token_supply, expected_liquidity);

    // TODO: test non-initialization mints
}

#[tokio::test]
async fn swap0() {
    swap_test(1, 5, 10, 1662497915).await;
}

#[tokio::test]
async fn swap1() {
    swap_test(1, 10, 5, 453305446).await;
}

#[tokio::test]
async fn swap2() {
    swap_test(2, 5, 10, 2851015155).await;
}

#[tokio::test]
async fn swap3() {
    swap_test(2, 10, 5, 831248957).await;
}

#[tokio::test]
async fn swap4() {
    swap_test(1, 10, 10, 906610893).await;
}

#[tokio::test]
async fn swap5() {
    swap_test(1, 100, 100, 987158034).await;
}

#[tokio::test]
async fn swap6() {
    swap_test(1, 1000, 1000, 996006981).await;
}

// Optimistic tests

async fn swap_test(
    swap_amount: u64,
    token_0_amount: u64,
    token_1_amount: u64,
    expected_output_amount: u64,
) {
    let fixture = setup().await;
    add_liquidity(&fixture, to_9_decimal(token_0_amount), to_9_decimal(token_1_amount))
        .await;

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    let is_err = fixture.exchange_instance
        .methods()
        .swap(0, expected_output_amount + 1, Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(
            Some(to_9_decimal(swap_amount)),
            None,
            None,
        ))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(1)
        .call()
        .await
        .is_err();
    assert!(is_err);

    let receipt = fixture.exchange_instance
        .methods()
        .swap(0, expected_output_amount, Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(
            Some(to_9_decimal(swap_amount)),
            None,
            None,
        ))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_output_amount);
}

#[tokio::test]
async fn swap_token_0() {
    let fixture = setup().await;
    
    let token_0_amount = to_9_decimal(5);
    let token_1_amount = to_9_decimal(10);

    add_liquidity(&fixture, token_0_amount, token_1_amount)
        .await;

    let swap_amount = to_9_decimal(1);
    let expected_output = 1662497915;

    let token_0_starting_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let token_1_starting_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    let receipt = fixture.exchange_instance
        .methods()
        .swap(0, expected_output, Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(Some(swap_amount), None, None))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, expectedOutputAmount)
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token0Amount.add(swapAmount), token1Amount.sub(expectedOutputAmount))
    //   .to.emit(pair, 'Swap')
    //   .withArgs(wallet.address, swapAmount, 0, 0, expectedOutputAmount, wallet.address)

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_0_reserve, token_0_amount + swap_amount);
    assert_eq!(pool_info.value.token_1_reserve, token_1_amount - expected_output);

    let exchange_token_0_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.exchange_contract_id, BASE_ASSET_ID)
        .await
        .unwrap();
    let exchange_token_1_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.exchange_contract_id, fixture.token_asset_id)
        .await
        .unwrap();
    assert_eq!(exchange_token_0_balance, token_0_amount + swap_amount);
    assert_eq!(exchange_token_1_balance, token_1_amount - expected_output);

    let token_0_end_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let token_1_end_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    assert_eq!(token_0_end_balance, token_0_starting_balance - swap_amount);
    assert_eq!(token_1_end_balance, token_1_starting_balance + expected_output);
}


#[tokio::test]
async fn swap_token_1() {
    let fixture = setup().await;
    
    let token_0_amount = to_9_decimal(5);
    let token_1_amount = to_9_decimal(10);

    add_liquidity(&fixture, token_0_amount, token_1_amount)
        .await;

    let swap_amount = to_9_decimal(1);
    let expected_output = 453305446;

    let token_0_starting_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let token_1_starting_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    let receipt = fixture.exchange_instance
        .methods()
        .swap(expected_output, 0, Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(Some(swap_amount), Some(fixture.token_asset_id.clone()), None))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, expectedOutputAmount)
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token0Amount.add(swapAmount), token1Amount.sub(expectedOutputAmount))
    //   .to.emit(pair, 'Swap')
    //   .withArgs(wallet.address, swapAmount, 0, 0, expectedOutputAmount, wallet.address)

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_1_reserve, token_1_amount + swap_amount);
    assert_eq!(pool_info.value.token_0_reserve, token_0_amount - expected_output);

    let exchange_token_0_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.exchange_contract_id, BASE_ASSET_ID)
        .await
        .unwrap();
    let exchange_token_1_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.exchange_contract_id, fixture.token_asset_id)
        .await
        .unwrap();
    assert_eq!(exchange_token_1_balance, token_1_amount + swap_amount);
    assert_eq!(exchange_token_0_balance, token_0_amount - expected_output);

    let token_0_end_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let token_1_end_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    assert_eq!(token_1_end_balance, token_1_starting_balance - swap_amount);
    assert_eq!(token_0_end_balance, token_0_starting_balance + expected_output);
}


#[tokio::test]
async fn burn() {
    let fixture = setup().await;

    let token_0_amount = to_9_decimal(3);
    let token_1_amount = to_9_decimal(3);

    add_liquidity(&fixture, token_0_amount, token_1_amount)
        .await;

    let expected_liquidity = to_9_decimal(3) - 1000;

    let token_0_starting_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let token_1_starting_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    let receipt = fixture.exchange_instance
        .methods()
        .remove_liquidity(Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(
            Some(expected_liquidity),
            Some(fixture.exchange_asset_id.clone()),
            None
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

    //   .to.emit(pair, 'Transfer')
    //   .withArgs(pair.address, AddressZero, expectedLiquidity.sub(MINIMUM_LIQUIDITY))
    //   .to.emit(token0, 'Transfer')
    //   .withArgs(pair.address, wallet.address, token0Amount.sub(1000))
    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, token1Amount.sub(1000))
    //   .to.emit(pair, 'Sync')
    //   .withArgs(1000, 1000)
    //   .to.emit(pair, 'Burn')
    //   .withArgs(wallet.address, token0Amount.sub(1000), token1Amount.sub(1000), wallet.address)

    let lp_balance = fixture.wallet.get_asset_balance(&fixture.exchange_asset_id.clone()).await.unwrap();
    assert_eq!(lp_balance, 0);

    // expect(await pair.totalSupply()).to.eq(MINIMUM_LIQUIDITY)

    let exchange_token_0_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.exchange_contract_id, BASE_ASSET_ID)
        .await
        .unwrap();
    let exchange_token_1_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.exchange_contract_id, fixture.token_asset_id)
        .await
        .unwrap();
    assert_eq!(exchange_token_0_balance, 1000);
    assert_eq!(exchange_token_1_balance, 1000);

    let token_0_end_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let token_1_end_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    assert_eq!(token_0_end_balance, token_0_starting_balance + token_0_amount - 1000);
    assert_eq!(token_1_end_balance, token_1_starting_balance + token_1_amount - 1000);
}

#[tokio::test]
async fn accrue_protocol_fees() {
    let fixture = setup().await;

    let token_0_amount = to_9_decimal(5);
    let token_1_amount = to_9_decimal(10);

    add_liquidity(&fixture, token_0_amount, token_1_amount)
        .await;

    fixture.vault_instance
        .methods()
        .set_fees(10000, 1000)
        .call()
        .await
        .unwrap();
    
    fixture.exchange_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.current_fee, 10000);
    assert_eq!(fee_info.value.change_rate, 1000);

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    // Swap again, token0 -> token1

    let input = to_9_decimal(1);
    let expected_output = 1648613753;

    let _receipt = fixture.exchange_instance
        .methods()
        .swap(0, expected_output, Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(
            Some(input),
            None,
            None,
        ))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.current_fee, 10000);
    assert_eq!(fee_info.value.change_rate, 1000);
    assert_eq!(fee_info.value.token_0_protocol_fees_collected, 10000000);
    assert_eq!(fee_info.value.token_1_protocol_fees_collected, 0);

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_output);

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_0_reserve, token_0_amount + (input * 99 / 100));
    assert_eq!(pool_info.value.token_1_reserve, token_1_amount - expected_output);

    // Wait, let the fee decrease to 0.9%
    // Sleep isn't ideal for these tests, ideally the local VM would allow changing timestamps
    sleep(Duration::from_secs(1)).await;

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.current_fee, 9000);

    // Swap again, token1 -> token0

    let starting_token_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();

    let expected_output = 633116959;

    let _receipt = fixture.exchange_instance
        .methods()
        .swap(expected_output, 0, Identity::Address(fixture.wallet.address().into()))
        .call_params(CallParameters::new(
            Some(input),
            Some(fixture.token_asset_id.clone()),
            None,
        ))
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.token_0_protocol_fees_collected, 10000000);
    assert_eq!(fee_info.value.token_1_protocol_fees_collected, 9000000);

    let end_token_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_output);

    // Claim funds

    fixture.vault_instance
        .methods()
        .claim_fees(Bits256(fixture.exchange_contract_id.hash().into()))
        .set_contracts(&[fixture.exchange_contract_id.clone()])
        .call()
        .await
        .unwrap();

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.token_0_protocol_fees_collected, 0);
    assert_eq!(fee_info.value.token_1_protocol_fees_collected, 0);

    let vault_token_0_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.vault_contract_id, BASE_ASSET_ID)
        .await
        .unwrap();
    let vault_token_1_balance = fixture
        .wallet
        .get_provider()
        .unwrap()
        .get_contract_asset_balance(&fixture.vault_contract_id, fixture.token_asset_id)
        .await
        .unwrap();
    assert_eq!(vault_token_0_balance, 10000000);
    assert_eq!(vault_token_1_balance, 9000000);
}
