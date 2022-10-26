use std::{vec, str::FromStr};
use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    signers::WalletUnlocked,
    tx::{AssetId, Bytes32, StorageSlot},
};

///////////////////////////////
// Load the Router Script ABI abi
///////////////////////////////
abigen!(Router, "./out/debug/router_contract-abi.json");

///////////////////////////////
// Load the Exchange Contract abi
///////////////////////////////
abigen!(TestExchange, "../exchange_contract/out/debug/exchange_contract-abi.json");

///////////////////////////////
// Load the Token Contract abi
///////////////////////////////T
abigen!(TestToken, "../token_contract/out/debug/token_contract-abi.json");
abigen!(Vault, "../vault_contract/out/debug/vault_contract-abi.json");


fn to_9_decimal(num: u64) -> u64 {
    num * 1_000_000_000
}

const MINIMUM_LIQUIDITY: u64 = 1000;

struct Fixture {
    wallet: WalletUnlocked,

    token_a_contract_id: Bech32ContractId,
    token_a_asset_id: AssetId,
    token_a_instance: TestToken,
    exchange_a_contract_id: Bech32ContractId,
    exchange_a_asset_id: AssetId,
    exchange_a_instance: TestExchange,

    token_b_contract_id: Bech32ContractId,
    token_b_asset_id: AssetId,
    token_b_instance: TestToken,
    exchange_b_contract_id: Bech32ContractId,
    exchange_b_asset_id: AssetId,
    exchange_b_instance: TestExchange,

    router_contract_id: Bech32ContractId,
    router_instance: Router,

    vault_contract_id: Bech32ContractId,
    vault_instance: Vault,
}

async fn setup() -> Fixture {
    let num_wallets = 1;
    let num_coins = 1;
    let amount = to_9_decimal(10000);
    let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(amount));

    let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet = wallets.pop().unwrap();

    //////////////////////////////////////////
    // Setup contracts
    //////////////////////////////////////////

    let token_a_contract_id = Contract::deploy_with_parameters(
        "../token_contract/out/debug/token_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
        Salt::from([0u8; 32]),
    )
    .await
    .unwrap();

    let token_b_contract_id = Contract::deploy_with_parameters(
        "../token_contract/out/debug/token_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    let token0_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap();
    let token1_slot = Bytes32::from_str("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();

    let storage_vec = vec![
        StorageSlot::new(token1_slot, token_a_contract_id.hash()),
    ];

    // Deploy contract and get ID
    let exchange_a_contract_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([0u8; 32]),
    )
    .await
    .unwrap();

    let storage_vec = vec![
        StorageSlot::new(token0_slot, token_a_contract_id.hash()),
        StorageSlot::new(token1_slot, token_b_contract_id.hash()),
    ];

    // Deploy contract and get ID
    let exchange_b_contract_id = Contract::deploy_with_parameters(
        "../exchange_contract/out/debug/exchange_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_manual_storage(Some(storage_vec)),
        Salt::from([1u8; 32]),
    )
    .await
    .unwrap();

    let router_contract_id = Contract::deploy(
        "./out/debug/router_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
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

    let exchange_a_instance = TestExchange::new(exchange_a_contract_id.to_string(), wallet.clone());
    let token_a_instance = TestToken::new(token_a_contract_id.to_string(), wallet.clone());
    let exchange_b_instance = TestExchange::new(exchange_b_contract_id.to_string(), wallet.clone());
    let token_b_instance = TestToken::new(token_b_contract_id.to_string(), wallet.clone());
    let router_instance = Router::new(router_contract_id.to_string(), wallet.clone());
    let vault_instance = Vault::new(vault_contract_id.to_string(), wallet.clone());

    let wallet_token_amount = to_9_decimal(20000);

    // Initialize token contract
    token_a_instance
        .methods()
        .initialize(wallet_token_amount, wallet.address().into())
        .call()
        .await
        .unwrap();

    // Mint some alt tokens
    token_a_instance
        .methods()
        .mint()
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    // Initialize token contract
    token_b_instance
        .methods()
        .initialize(wallet_token_amount, wallet.address().into())
        .call()
        .await
        .unwrap();

    // Mint some alt tokens
    token_b_instance
        .methods()
        .mint()
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    exchange_a_instance
        .methods()
        .initialize(Bits256(vault_contract_id.hash().into()))
        .set_contracts(&[vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    exchange_b_instance
        .methods()
        .initialize(Bits256(vault_contract_id.hash().into()))
        .set_contracts(&[vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    Fixture {
        wallet: wallet,

        token_a_contract_id: token_a_contract_id.clone(),
        token_a_asset_id: AssetId::new(*token_a_contract_id.hash()),
        token_a_instance: token_a_instance,
        exchange_a_contract_id: exchange_a_contract_id.clone(),
        exchange_a_instance: exchange_a_instance,
        exchange_a_asset_id: AssetId::new(*exchange_a_contract_id.hash()),

        token_b_contract_id: token_b_contract_id.clone(),
        token_b_asset_id: AssetId::new(*token_b_contract_id.hash()),
        token_b_instance: token_b_instance,
        exchange_b_contract_id: exchange_b_contract_id.clone(),
        exchange_b_instance: exchange_b_instance,
        exchange_b_asset_id: AssetId::new(*exchange_b_contract_id.hash()),

        vault_contract_id: vault_contract_id,
        vault_instance: vault_instance,

        router_contract_id: router_contract_id,
        router_instance: router_instance,
    }
}


async fn add_pool_a_liquidity(fixture: &Fixture, token_0_amount: u64, token_1_amount: u64) {
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_a_contract_id,
            token_0_amount,
            BASE_ASSET_ID,
            TxParameters::default()
        )
        .await;

    // Deposit some Token Asset
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_a_contract_id,
            token_1_amount,
            fixture.token_a_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    let result = fixture.exchange_a_instance
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

// TODO: DRY
async fn add_pool_b_liquidity(fixture: &Fixture, token_0_amount: u64, token_1_amount: u64) {
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_b_contract_id,
            token_0_amount,
            fixture.token_a_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    // Deposit some Token Asset
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.exchange_b_contract_id,
            token_1_amount,
            fixture.token_b_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    let result = fixture.exchange_b_instance
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
async fn add_liquidity() {
    let fixture = setup().await;

    let token_0_amount = to_9_decimal(1);
    let token_1_amount = to_9_decimal(4);
    let expected_liquidity = to_9_decimal(2);

    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.router_contract_id,
            token_0_amount,
            BASE_ASSET_ID,
            TxParameters::default()
        )
        .await;
    let _receipts = fixture.wallet
        .force_transfer_to_contract(
            &fixture.router_contract_id,
            token_1_amount,
            fixture.token_a_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    let receipt = fixture.router_instance
        .methods()
        .add_liquidity(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            token_0_amount,
            token_1_amount,
            0,
            0,
            Identity::Address(fixture.wallet.address().into()),
        )
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
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(3)
        .call()
        .await
        .unwrap();
    //   .to.emit(token0, 'Transfer')
    //   .withArgs(wallet.address, pair.address, token_0_amount)
    //   .to.emit(token1, 'Transfer')
    //   .withArgs(wallet.address, pair.address, token_1_amount)
    //   .to.emit(pair, 'Transfer')
    //   .withArgs(AddressZero, AddressZero, MINIMUM_LIQUIDITY)
    //   .to.emit(pair, 'Transfer')
    //   .withArgs(AddressZero, wallet.address, expected_liquidity.sub(MINIMUM_LIQUIDITY))
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token_0_amount, token_1_amount)
    //   .to.emit(pair, 'Mint')
    //   .withArgs(router.address, token_0_amount, token_1_amount)

    let lp_tokens = fixture.wallet.get_asset_balance(&fixture.exchange_a_asset_id).await.unwrap();
    assert_eq!(lp_tokens, expected_liquidity - MINIMUM_LIQUIDITY);
}

#[tokio::test]
async fn swap_exact_input_0() {
    let fixture = setup().await;

    let token0_amount = to_9_decimal(5);
    let token1_amount = to_9_decimal(10);
    let swap_amount = to_9_decimal(1);
    let expected_amount = 1662497915;

    add_pool_a_liquidity(&fixture, token0_amount, token1_amount)
        .await;

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    let is_err = fixture.router_instance
        .methods()
        .swap_exact_input(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            expected_amount + 1,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(swap_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(1)
        .call()
        .await
        .is_err();
    assert!(is_err);

    let result = fixture.router_instance
        .methods()
        .swap_exact_input(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            expected_amount,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(swap_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    //   .to.emit(token0, 'Transfer')
    //   .withArgs(wallet.address, pair.address, swapAmount)
    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, expectedOutputAmount)
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token0Amount.add(swapAmount), token1Amount.sub(expectedOutputAmount))
    //   .to.emit(pair, 'Swap')
    //   .withArgs(router.address, swapAmount, 0, 0, expectedOutputAmount, wallet.address)

    assert_eq!(result.value.input_amount, swap_amount);
    assert_eq!(result.value.output_amount, expected_amount);

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_amount);
}


#[tokio::test]
async fn swap_exact_output_0() {
    let fixture = setup().await;

    let token0_amount = to_9_decimal(5);
    let token1_amount = to_9_decimal(10);
    let expected_input = to_9_decimal(1);
    let provided_input = to_9_decimal(2);
    let output_amount = 1662497915;

    add_pool_a_liquidity(&fixture, token0_amount, token1_amount)
        .await;

    let starting_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    // Check that max_input reverts
    let is_err = fixture.router_instance
        .methods()
        .swap_exact_output(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            output_amount,
            expected_input - 1,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(provided_input),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(3)
        .call()
        .await
        .is_err();
    assert!(is_err);

    let result = fixture.router_instance
        .methods()
        .swap_exact_output(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            output_amount,
            expected_input,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(provided_input),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(3)
        .call()
        .await
        .unwrap();

    //   .to.emit(token0, 'Transfer')
    //   .withArgs(wallet.address, pair.address, swapAmount)
    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, expectedOutputAmount)
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token0Amount.add(swapAmount), token1Amount.sub(expectedOutputAmount))
    //   .to.emit(pair, 'Swap')
    //   .withArgs(router.address, swapAmount, 0, 0, expectedOutputAmount, wallet.address)

    assert_eq!(result.value.input_amount, expected_input);
    assert_eq!(result.value.output_amount, output_amount);

    let end_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, output_amount);
    assert_eq!(starting_eth_balance - end_eth_balance, expected_input);
}

#[tokio::test]
async fn swap_exact_input_multi() {
    let fixture = setup().await;

    let token0_amount_a = to_9_decimal(5);
    let token1_amount_a = to_9_decimal(10);

    let token0_amount_b = to_9_decimal(5);
    let token1_amount_b = to_9_decimal(10);

    let swap_amount = to_9_decimal(1);
    let expected_amount = 2489685056;

    add_pool_a_liquidity(&fixture, token0_amount_a, token1_amount_a)
        .await;

    add_pool_b_liquidity(&fixture, token0_amount_b, token1_amount_b)
        .await;

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    let is_err = fixture.router_instance
        .methods()
        .swap_exact_input_multihop(
            vec![
                Bits256(fixture.exchange_a_contract_id.hash().into()),
                Bits256(fixture.exchange_b_contract_id.hash().into()),
            ],
            expected_amount + 1,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(swap_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[
            fixture.exchange_a_contract_id.clone(),
            fixture.exchange_b_contract_id.clone(),
        ])
        .append_variable_outputs(1)
        .call()
        .await
        .is_err();
    assert!(is_err);

    let result = fixture.router_instance
        .methods()
        .swap_exact_input_multihop(
            vec![
                Bits256(fixture.exchange_a_contract_id.hash().into()),
                Bits256(fixture.exchange_b_contract_id.hash().into()),
            ],
            expected_amount,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(swap_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[
            fixture.exchange_a_contract_id.clone(),
            fixture.exchange_b_contract_id.clone(),
        ])
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    //   .to.emit(token0, 'Transfer')
    //   .withArgs(wallet.address, pair.address, swapAmount)
    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, expectedOutputAmount)
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token0Amount.add(swapAmount), token1Amount.sub(expectedOutputAmount))
    //   .to.emit(pair, 'Swap')
    //   .withArgs(router.address, swapAmount, 0, 0, expectedOutputAmount, wallet.address)

    assert_eq!(result.value.input_amount, swap_amount);
    assert_eq!(result.value.output_amount, expected_amount);

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_amount);
}


#[tokio::test]
async fn swap_exact_output_multi() {
    let fixture = setup().await;

    let token0_amount_a = to_9_decimal(5);
    let token1_amount_a = to_9_decimal(10);

    let token0_amount_b = to_9_decimal(5);
    let token1_amount_b = to_9_decimal(10);

    let input_amount = to_9_decimal(2);
    let expected_input = to_9_decimal(1);
    let output_amount = 2489685056;

    add_pool_a_liquidity(&fixture, token0_amount_a, token1_amount_a)
        .await;

    add_pool_b_liquidity(&fixture, token0_amount_b, token1_amount_b)
        .await;

    let starting_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();


    let is_err = fixture.router_instance
        .methods()
        .swap_exact_output_multihop(
            vec![
                Bits256(fixture.exchange_a_contract_id.hash().into()),
                Bits256(fixture.exchange_b_contract_id.hash().into()),
            ],
            output_amount,
            expected_input - 1,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(input_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[
            fixture.exchange_a_contract_id.clone(),
            fixture.exchange_b_contract_id.clone(),
        ])
        .append_variable_outputs(2)
        .call()
        .await
        .is_err();
    assert!(is_err);

    let result = fixture.router_instance
        .methods()
        .swap_exact_output_multihop(
            vec![
                Bits256(fixture.exchange_a_contract_id.hash().into()),
                Bits256(fixture.exchange_b_contract_id.hash().into()),
            ],
            output_amount,
            expected_input,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(input_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[
            fixture.exchange_a_contract_id.clone(),
            fixture.exchange_b_contract_id.clone(),
        ])
        .append_variable_outputs(2)
        .call()
        .await
        .unwrap();

    //   .to.emit(token0, 'Transfer')
    //   .withArgs(wallet.address, pair.address, swapAmount)
    //   .to.emit(token1, 'Transfer')
    //   .withArgs(pair.address, wallet.address, expectedOutputAmount)
    //   .to.emit(pair, 'Sync')
    //   .withArgs(token0Amount.add(swapAmount), token1Amount.sub(expectedOutputAmount))
    //   .to.emit(pair, 'Swap')
    //   .withArgs(router.address, swapAmount, 0, 0, expectedOutputAmount, wallet.address)

    assert_eq!(result.value.input_amount, expected_input);
    assert_eq!(result.value.output_amount, output_amount);

    let end_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, output_amount);
    assert_eq!(starting_eth_balance - end_eth_balance, expected_input);
}


#[tokio::test]
async fn with_protocol_fees_swap_exact_input_0() {
    let fixture = setup().await;

    fixture.vault_instance
        .methods()
        .set_fees(10_000, 0)
        .call()
        .await
        .unwrap();

    fixture.exchange_a_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    let token0_amount = to_9_decimal(5);
    let token1_amount = to_9_decimal(10);
    let swap_amount = to_9_decimal(1);
    let expected_amount = 1648613753;

    fixture.vault_instance
        .methods()
        .set_fees(10_000, 0)
        .call()
        .await
        .unwrap();

    fixture.exchange_a_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();


    add_pool_a_liquidity(&fixture, token0_amount, token1_amount)
        .await;

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    let result = fixture.router_instance
        .methods()
        .swap_exact_input(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            expected_amount,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(swap_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    assert_eq!(result.value.input_amount, swap_amount);
    assert_eq!(result.value.output_amount, expected_amount);

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_amount);
}


#[tokio::test]
async fn with_protocol_fees_swap_exact_output_0() {
    let fixture = setup().await;

    fixture.vault_instance
        .methods()
        .set_fees(10_000, 0)
        .call()
        .await
        .unwrap();

    fixture.exchange_a_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    let token0_amount = to_9_decimal(5);
    let token1_amount = to_9_decimal(10);
    let expected_input = 1010101010;
    let provided_input = to_9_decimal(2);
    let output_amount = 1662497915;

    add_pool_a_liquidity(&fixture, token0_amount, token1_amount)
        .await;

    let starting_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    let result = fixture.router_instance
        .methods()
        .swap_exact_output(
            Bits256(fixture.exchange_a_contract_id.hash().into()),
            output_amount,
            expected_input,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(provided_input),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[fixture.exchange_a_contract_id.clone()])
        .append_variable_outputs(3)
        .call()
        .await
        .unwrap();

    assert_eq!(result.value.input_amount, expected_input);
    assert_eq!(result.value.output_amount, output_amount);

    let end_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_a_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, output_amount);
    assert_eq!(starting_eth_balance - end_eth_balance, expected_input);
}

#[tokio::test]
async fn with_protocol_fees_swap_exact_input_multi() {
    let fixture = setup().await;

    let token0_amount_a = to_9_decimal(5);
    let token1_amount_a = to_9_decimal(10);

    let token0_amount_b = to_9_decimal(5);
    let token1_amount_b = to_9_decimal(10);

    let swap_amount = to_9_decimal(1);
    let expected_amount = 2455371143;

    fixture.vault_instance
        .methods()
        .set_fees(10_000, 0)
        .call()
        .await
        .unwrap();

    fixture.exchange_a_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    fixture.exchange_b_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    add_pool_a_liquidity(&fixture, token0_amount_a, token1_amount_a)
        .await;

    add_pool_b_liquidity(&fixture, token0_amount_b, token1_amount_b)
        .await;

    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    let result = fixture.router_instance
        .methods()
        .swap_exact_input_multihop(
            vec![
                Bits256(fixture.exchange_a_contract_id.hash().into()),
                Bits256(fixture.exchange_b_contract_id.hash().into()),
            ],
            expected_amount,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(swap_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[
            fixture.exchange_a_contract_id.clone(),
            fixture.exchange_b_contract_id.clone(),
        ])
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    assert_eq!(result.value.input_amount, swap_amount);
    assert_eq!(result.value.output_amount, expected_amount);

    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, expected_amount);
}

#[tokio::test]
async fn with_protocol_fees_swap_exact_output_multi() {
    let fixture = setup().await;

    let token0_amount_a = to_9_decimal(5);
    let token1_amount_a = to_9_decimal(10);

    let token0_amount_b = to_9_decimal(5);
    let token1_amount_b = to_9_decimal(10);

    let input_amount = to_9_decimal(2);
    let expected_input = 1022363234;
    let output_amount = 2489685056;

    fixture.vault_instance
        .methods()
        .set_fees(10_000, 0)
        .call()
        .await
        .unwrap();

    fixture.exchange_a_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    fixture.exchange_b_instance
        .methods()
        .cache_vault_fees()
        .set_contracts(&[fixture.vault_contract_id.clone()])
        .call()
        .await
        .unwrap();

    add_pool_a_liquidity(&fixture, token0_amount_a, token1_amount_a)
        .await;

    add_pool_b_liquidity(&fixture, token0_amount_b, token1_amount_b)
        .await;

    let starting_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let starting_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    let result = fixture.router_instance
        .methods()
        .swap_exact_output_multihop(
            vec![
                Bits256(fixture.exchange_a_contract_id.hash().into()),
                Bits256(fixture.exchange_b_contract_id.hash().into()),
            ],
            output_amount,
            expected_input,
            Identity::Address(fixture.wallet.address().into()),
        )
        .tx_params(TxParameters {
            gas_price: 0,
            gas_limit: 100_000_000,
            maturity: 0,
        })
        .call_params(CallParameters::new(
            Some(input_amount),
            None,
            Some(100_000_000),
        ))
        .set_contracts(&[
            fixture.exchange_a_contract_id.clone(),
            fixture.exchange_b_contract_id.clone(),
        ])
        .append_variable_outputs(2)
        .call()
        .await
        .unwrap();

    assert_eq!(result.value.input_amount, expected_input);
    assert_eq!(result.value.output_amount, output_amount);

    let end_eth_balance = fixture.wallet.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
    let end_token_balance = fixture.wallet.get_asset_balance(&fixture.token_b_asset_id).await.unwrap();

    assert_eq!(end_token_balance - starting_token_balance, output_amount);
    assert_eq!(starting_eth_balance - end_eth_balance, expected_input);
}
