use std::{vec, str::FromStr};
use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    signers::WalletUnlocked,
    tx::{AssetId, ContractId, Bytes32, StorageSlot},
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
abigen!(
    TestToken,
    "../token_contract/out/debug/token_contract-abi.json"
);


fn to_9_decimal(num: u64) -> u64 {
    num * 1_000_000_000
}

const MINIMUM_LIQUIDITY: u64 = 1000;

struct Fixture {
    wallet: WalletUnlocked,
    token_contract_id: Bech32ContractId,
    exchange_contract_id: Bech32ContractId,
    router_contract_id: Bech32ContractId,
    token_asset_id: AssetId,
    exchange_asset_id: AssetId,
    token_instance: TestToken,
    exchange_instance: TestExchange,
    router_instance: Router,
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


    let router_contract_id = Contract::deploy(
        "./out/debug/router_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let exchange_instance = TestExchange::new(exchange_contract_id.to_string(), wallet.clone());
    let token_instance = TestToken::new(token_contract_id.to_string(), wallet.clone());
    let router_instance = Router::new(router_contract_id.to_string(), wallet.clone());

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

    Fixture {
        wallet: wallet,
        token_contract_id: token_contract_id.clone(),
        exchange_contract_id: exchange_contract_id.clone(),
        router_contract_id: router_contract_id,
        token_asset_id: AssetId::new(*token_contract_id.hash()),
        exchange_asset_id: AssetId::new(*exchange_contract_id.hash()),
        token_instance: token_instance,
        exchange_instance: exchange_instance,
        router_instance: router_instance,
    }
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
            fixture.token_asset_id.clone(),
            TxParameters::default()
        )
        .await;

    let receipt = fixture.router_instance
        .methods()
        .add_liquidity(
            Bits256(fixture.exchange_contract_id.hash().into()),
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
        .set_contracts(&[fixture.exchange_contract_id.clone()])
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

    let lp_tokens = fixture.wallet.get_asset_balance(&fixture.exchange_asset_id).await.unwrap();
    assert_eq!(lp_tokens, expected_liquidity - MINIMUM_LIQUIDITY);
}
