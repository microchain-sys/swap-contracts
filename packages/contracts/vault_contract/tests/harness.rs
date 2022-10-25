use std::{
    time::{SystemTime, UNIX_EPOCH},
};
use fuels::{
    prelude::*,
    fuels_abigen::abigen,
    signers::WalletUnlocked,
};
use tokio::time::{sleep, Duration};


///////////////////////////////
// Load the Vault Contract abi
///////////////////////////////
abigen!(TestVault, "./out/debug/vault_contract-abi.json");

struct Fixture {
    wallet: WalletUnlocked,
    vault_contract_id: Bech32ContractId,
    vault_instance: TestVault,
}

async fn setup() -> Fixture {
    let num_wallets = 1;
    let num_coins = 1;
    let config = WalletsConfig::new(Some(num_wallets), Some(num_coins), Some(1000000000));
  
    let mut wallets = launch_custom_provider_and_get_wallets(config, None).await;
    let wallet = wallets.pop().unwrap();

    //////////////////////////////////////////
    // Setup contracts
    //////////////////////////////////////////

    let vault_contract_id = Contract::deploy(
        "./out/debug/vault_contract.bin",
        &wallet,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();

    let vault_instance = TestVault::new(vault_contract_id.to_string(), wallet.clone());

    Fixture {
        wallet: wallet,
        vault_contract_id: vault_contract_id.clone(),
        vault_instance: vault_instance,
    }
}

#[tokio::test]
async fn set_fee() {
    let fixture = setup().await;

    // Add liquidity for the second time. Keeping the proportion 1:2
    // It should return the same amount of LP as the amount of ETH deposited
    let _result = fixture.vault_instance
        .methods()
        .set_fees(100, 1)
        .call()
        .await
        .unwrap();

    let returned_fees = fixture.vault_instance.methods().get_fees().call().await.unwrap();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    assert!(now.as_secs() - returned_fees.value.start_time as u64 <= 1);
    assert_eq!(returned_fees.value.start_fee, 100);
    assert_eq!(returned_fees.value.current_fee, 100);
    assert_eq!(returned_fees.value.change_rate, 1);

    sleep(Duration::from_secs(2)).await;

    let returned_fees = fixture.vault_instance.methods().get_fees().call().await.unwrap();

    assert!(now.as_secs() - returned_fees.value.start_time as u64 <= 1);
    assert_eq!(returned_fees.value.start_fee, 100);
    assert_eq!(returned_fees.value.current_fee, 98);
    assert_eq!(returned_fees.value.change_rate, 1);
}

#[tokio::test]
async fn protocol_fees_minimum_zero() {
    let fixture = setup().await;

    // Add liquidity for the second time. Keeping the proportion 1:2
    // It should return the same amount of LP as the amount of ETH deposited
    let _result = fixture.vault_instance
        .methods()
        .set_fees(1_000, 10_000)
        .call()
        .await
        .unwrap();


    sleep(Duration::from_secs(1)).await;

    let returned_fees = fixture.vault_instance.methods().get_fees().call().await.unwrap();

    assert_eq!(returned_fees.value.start_fee, 1_000);
    assert_eq!(returned_fees.value.current_fee, 0);
    assert_eq!(returned_fees.value.change_rate, 10_000);
}
