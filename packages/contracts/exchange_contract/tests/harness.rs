extern crate test_helpers;

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
use test_helpers::get_timestamp_and_call;

///////////////////////////////
// Load the Exchange Contract abi
///////////////////////////////
abigen!(Exchange, "./out/debug/exchange_contract-abi.json");

///////////////////////////////
// Load the Token Contract abi
///////////////////////////////
abigen!(
    TestToken,
    "../token_contract/out/debug/token_contract-abi.json"
);
abigen!(Vault, "../vault_contract/out/debug/vault_contract-abi.json");

struct Fixture {
    wallet: WalletUnlocked,
    token_contract_id: Bech32ContractId,
    exchange_contract_id: Bech32ContractId,
    vault_contract_id: Bech32ContractId,
    token_asset_id: AssetId,
    exchange_asset_id: AssetId,
    token_instance: TestToken,
    exchange_instance: Exchange,
    vault_instance: Vault,
}

fn to_9_decimal(num: u64) -> u64 {
    num * 1_000_000_000
}

async fn setup() -> Fixture {
    let num_wallets = 1;
    let num_coins = 1;
    let amount = to_9_decimal(10000);
    let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));
  
    let wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet = wallets.get(0).unwrap().clone();
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

    let exchange_instance = Exchange::new(exchange_contract_id.clone(), wallet.clone());
    let token_instance = TestToken::new(token_contract_id.clone(), wallet.clone());
    let vault_instance = Vault::new(vault_contract_id.clone(), wallet.clone());

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

async fn add_liquidity(fixture: &Fixture, token_0_amount: u64, token_1_amount: u64) -> u64 {
    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    println!("{:?}", pool_info.value);

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

    let handler = fixture.exchange_instance
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
        ));

    let (_response, timestamp) = get_timestamp_and_call(handler).await;

    timestamp
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

    let swap_logs = fixture.exchange_instance.logs_with_type::<LiquidityAdded>(&result.receipts).unwrap();
    assert_eq!(swap_logs.len(), 2);

    let swap_event = swap_logs.get(0).unwrap();
    assert_eq!(swap_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.recipient, Bits256([0; 32]));
    assert_eq!(swap_event.amount_0, 0);
    assert_eq!(swap_event.amount_1, 0);
    assert_eq!(swap_event.lp_tokens, 1000);

    let swap_event = swap_logs.get(1).unwrap();
    assert_eq!(swap_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.recipient, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.amount_0, token_0_amount);
    assert_eq!(swap_event.amount_1, token_1_amount);
    assert_eq!(swap_event.lp_tokens, expected_liquidity - 1000);

    let reserve_logs = fixture.exchange_instance.logs_with_type::<UpdateReserves>(&result.receipts).unwrap();
    assert_eq!(reserve_logs.len(), 1);
    let reserve_event = reserve_logs.get(0).unwrap();
    assert_eq!(reserve_event.amount_0, 1000000000);
    assert_eq!(reserve_event.amount_1, 4000000000);

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

    let _receipt = fixture.exchange_instance
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

    let swap_logs = fixture.exchange_instance.logs_with_type::<Swap>(&receipt.receipts).unwrap();
    assert_eq!(swap_logs.len(), 1);
    let swap_event = swap_logs.get(0).unwrap();
    assert_eq!(swap_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.recipient, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.amount_0_in, swap_amount);
    assert_eq!(swap_event.amount_1_in, 0);
    assert_eq!(swap_event.amount_0_out, 0);
    assert_eq!(swap_event.amount_1_out, expected_output);

    let reserve_logs = fixture.exchange_instance.logs_with_type::<UpdateReserves>(&receipt.receipts).unwrap();
    assert_eq!(reserve_logs.len(), 1);
    let reserve_event = reserve_logs.get(0).unwrap();
    assert_eq!(reserve_event.amount_0, token_0_amount + swap_amount);
    assert_eq!(reserve_event.amount_1, token_1_amount - expected_output);

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

    let swap_logs = fixture.exchange_instance.logs_with_type::<Swap>(&receipt.receipts).unwrap();
    assert_eq!(swap_logs.len(), 1);
    let swap_event = swap_logs.get(0).unwrap();
    assert_eq!(swap_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.recipient, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.amount_1_in, swap_amount);
    assert_eq!(swap_event.amount_0_in, 0);
    assert_eq!(swap_event.amount_1_out, 0);
    assert_eq!(swap_event.amount_0_out, expected_output);

    let reserve_logs = fixture.exchange_instance.logs_with_type::<UpdateReserves>(&receipt.receipts).unwrap();
    assert_eq!(reserve_logs.len(), 1);
    let reserve_event = reserve_logs.get(0).unwrap();
    assert_eq!(reserve_event.amount_1, token_1_amount + swap_amount);
    assert_eq!(reserve_event.amount_0, token_0_amount - expected_output);

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

    let swap_logs = fixture.exchange_instance.logs_with_type::<LiquidityRemoved>(&receipt.receipts).unwrap();
    assert_eq!(swap_logs.len(), 1);
    let swap_event = swap_logs.get(0).unwrap();
    assert_eq!(swap_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.recipient, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(swap_event.amount_0, token_0_amount - 1000);
    assert_eq!(swap_event.amount_1, token_1_amount - 1000);
    assert_eq!(swap_event.lp_tokens, expected_liquidity);

    let reserve_logs = fixture.exchange_instance.logs_with_type::<UpdateReserves>(&receipt.receipts).unwrap();
    assert_eq!(reserve_logs.len(), 1);
    let reserve_event = reserve_logs.get(0).unwrap();
    assert_eq!(reserve_event.amount_1, 1000);
    assert_eq!(reserve_event.amount_0, 1000);

    let lp_balance = fixture.wallet.get_asset_balance(&fixture.exchange_asset_id.clone()).await.unwrap();
    assert_eq!(lp_balance, 0);

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.lp_token_supply, 1000); // MINIMUM_LIQUIDITY

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

    let (_result, set_fee_timestamp) = get_timestamp_and_call(
        fixture.vault_instance
            .methods()
            .set_fees(10000, 1000)
    )
        .await;
    
    fixture.exchange_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    let (fee_info, fee_info_timestamp) = get_timestamp_and_call(
        fixture.exchange_instance.methods().get_vault_info()
    ).await;

    assert_eq!(fee_info.value.current_fee as u64, 10000 - (1000 * (fee_info_timestamp - set_fee_timestamp)));
    assert_eq!(fee_info.value.change_rate, 1000);

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    // Swap again, token0 -> token1

    let input = to_9_decimal(1);
    let expected_output = 1648613753;

    let (result, swap_timestamp) = get_timestamp_and_call(
        fixture.exchange_instance
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
    )
        .await;

    let expected_fee_rate = 10000 - (1000 * (swap_timestamp - set_fee_timestamp));
    let expected_fee_0 = input * expected_fee_rate / 1_000_000;

    let logs = fixture.exchange_instance.logs_with_type::<ProtocolFeeCollected>(&result.receipts).unwrap();
    assert_eq!(logs.len(), 1);
    let fee_event = logs.get(0).unwrap();
    assert_eq!(fee_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(fee_event.amount_0, expected_fee_0);
    assert_eq!(fee_event.amount_1, 0);

    let (fee_info, fee_info_timestamp) = get_timestamp_and_call(
        fixture.exchange_instance.methods().get_vault_info()
    ).await;

    let expected_fee_rate = 10000 - (1000 * (fee_info_timestamp - set_fee_timestamp));
    assert_eq!(fee_info.value.current_fee as u64, expected_fee_rate);
    assert_eq!(fee_info.value.change_rate, 1000);
    assert_eq!(fee_info.value.token_0_protocol_fees_collected, expected_fee_0);
    assert_eq!(fee_info.value.token_1_protocol_fees_collected, 0);

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_output);

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();
    assert_eq!(pool_info.value.token_0_reserve, token_0_amount + input - expected_fee_0);
    assert_eq!(pool_info.value.token_1_reserve, token_1_amount - expected_output);

    // Wait, let the fee decrease to 0.9%
    // Sleep isn't ideal for these tests, ideally the local VM would allow changing timestamps
    sleep(Duration::from_secs(1)).await;

    let (fee_info, fee_info_timestamp) = get_timestamp_and_call(
        fixture.exchange_instance.methods().get_vault_info()
    ).await;
    let expected_fee_rate = 10000 - (1000 * (fee_info_timestamp - set_fee_timestamp));
    assert_eq!(fee_info.value.current_fee as u64, expected_fee_rate);

    // Swap again, token1 -> token0

    let starting_token_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();

    let expected_output = 633116959;

    let (result, swap_timestamp) = get_timestamp_and_call(
        fixture.exchange_instance
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
    )
        .await;

    let expected_fee_rate = 10000 - (1000 * (swap_timestamp - set_fee_timestamp));
    let expected_fee_1 = input * expected_fee_rate / 1_000_000;

    let logs = fixture.exchange_instance.logs_with_type::<ProtocolFeeCollected>(&result.receipts).unwrap();
    assert_eq!(logs.len(), 1);
    let fee_event = logs.get(0).unwrap();
    assert_eq!(fee_event.sender, Bits256(fixture.wallet.address().hash().into()));
    assert_eq!(fee_event.amount_0, 0);
    assert_eq!(fee_event.amount_1, expected_fee_1);

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.token_0_protocol_fees_collected, expected_fee_0);
    assert_eq!(fee_info.value.token_1_protocol_fees_collected, expected_fee_1);

    let end_token_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_output);

    // Claim funds

    let result = fixture.vault_instance
        .methods()
        .claim_fees(Bits256(fixture.exchange_contract_id.hash().into()))
        .set_contracts(&[fixture.exchange_contract_id.clone()])
        .call()
        .await
        .unwrap();

    let logs = fixture.exchange_instance.logs_with_type::<ProtocolFeeWithdrawn>(&result.receipts).unwrap();
    assert_eq!(logs.len(), 1);
    let swap_event = logs.get(0).unwrap();
    assert_eq!(swap_event.amount_0, expected_fee_0);
    assert_eq!(swap_event.amount_1, expected_fee_1);

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
    assert_eq!(vault_token_0_balance, expected_fee_0);
    assert_eq!(vault_token_1_balance, expected_fee_1);
}

#[tokio::test]
async fn non_vault_claim_will_fail() {
    let fixture = setup().await;

    let is_err = fixture.exchange_instance
        .methods()
        .withdraw_protocol_fees(Identity::Address(fixture.wallet.address().into()))
        .call()
        .await
        .is_err();
    assert!(is_err);
}

#[tokio::test]
async fn protocol_fees_minimum_zero() {
    let fixture = setup().await;

    fixture.vault_instance
        .methods()
        .set_fees(1_000, 10_000)
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
    assert_eq!(fee_info.value.current_fee, 1_000);
    assert_eq!(fee_info.value.change_rate, 10_000);

    // Wait, the fee should decrease to 0 over this time
    // Sleep isn't ideal for these tests, ideally the local VM would allow changing timestamps
    sleep(Duration::from_secs(1)).await;

    let fee_info = fixture.exchange_instance.methods().get_vault_info().call().await.unwrap();
    assert_eq!(fee_info.value.current_fee, 0);
}


#[tokio::test]
async fn observe_price_changes() {
    let fixture = setup().await;

    // Observation with no liquidity should fail
    let is_err = fixture
        .exchange_instance
        .methods()
        .observe(0)
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call()
        .await
        .is_err();
    assert!(is_err);

    // Add liquidity

    let token_0_amount = to_9_decimal(5);
    let token_1_amount = to_9_decimal(10);

    let add_liq_timestamp = add_liquidity(&fixture, token_0_amount, token_1_amount)
        .await;

    sleep(Duration::from_secs(1)).await;

    // Observe price since liquidity added

    let (observation_1, observation_timestamp) = get_timestamp_and_call(
        fixture
            .exchange_instance
            .methods()
            .observe(0)
            .tx_params(TxParameters {
                gas_price: 0,
                gas_limit: 100_000_000,
                maturity: 0,
            })
        )
        .await;

    let precision = 1_000_000_000;
    let token_1_price = token_0_amount * precision / token_1_amount;
    let token_0_price = token_1_amount * precision / token_0_amount;

    let time_delta = observation_timestamp - add_liq_timestamp;

    // TODO: Is `u256.d` the best way to do this? Is there a .to_u64()?
    assert_eq!(observation_1.value.0.d, token_0_price * time_delta);
    assert_eq!(observation_1.value.1.d, token_1_price * time_delta);

    sleep(Duration::from_secs(1)).await;

    // Increase buffer size
    fixture.exchange_instance
        .methods()
        .expand_twap_buffer(2)
        .call()
        .await
        .unwrap();

    // Make a swap to change the price

    let input = to_9_decimal(1);
    let expected_output = 1648613753;

    let (_result, swap_timestamp) = get_timestamp_and_call(
        fixture.exchange_instance
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
        )
        .await;

    let pool_info = fixture.exchange_instance.methods().get_pool_info().call().await.unwrap();

    let token_0_price_post_swap = pool_info.value.token_1_reserve * precision / pool_info.value.token_0_reserve;
    let token_1_price_post_swap = pool_info.value.token_0_reserve * precision / pool_info.value.token_1_reserve;

    let (observation_2, observation_timestamp) = get_timestamp_and_call(
        fixture
            .exchange_instance
            .methods()
            .observe(0)
            .tx_params(TxParameters {
                gas_price: 0,
                gas_limit: 100_000_000,
                maturity: 0,
            })
        )
        .await;

    let time_delta_before_swap = swap_timestamp - add_liq_timestamp;
    let time_delta_after_swap = observation_timestamp - swap_timestamp;

    // TODO: Is `u256.d` the best way to do this? Is there a .to_u64()?
    assert_eq!(observation_2.value.0.d, (token_0_price * time_delta_before_swap) + (token_0_price_post_swap * time_delta_after_swap));
    assert_eq!(observation_2.value.1.d, (token_1_price * time_delta_before_swap) + (token_1_price_post_swap * time_delta_after_swap));

    // TODO: Test price where seconds_ago != 0
}
