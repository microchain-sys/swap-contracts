use fuels::{
    core::tx::{AssetId, ContractId},
    prelude::*,
    fuels_abigen::abigen,
};
use test_helpers::get_wallets;

///////////////////////////////
// Load the Token Contract abi
///////////////////////////////
abigen!(TestToken, "./out/debug/token_contract-abi.json");

#[tokio::test]
async fn token_contract() {
    let wallets = get_wallets().await;
    let wallet_owner = wallets.get(0).unwrap();
    let wallet_mint1 = wallets.get(1).unwrap();
    let wallet_mint2 = wallets.get(2).unwrap();

    ////////////////////////////////////////////////////////
    // Setup contracts
    ////////////////////////////////////////////////////////

    let token_contract_id = Contract::deploy(
        "../token_contract/out/debug/token_contract.bin",
        &wallet_owner,
        TxParameters::default(),
        StorageConfiguration::new(None, None),
    )
    .await
    .unwrap();
    let token_instance = TestToken::new(token_contract_id.clone(), wallet_owner.clone());

    ////////////////////////////////////////////////////////
    // Test Token Contract
    ////////////////////////////////////////////////////////

    // Get the contract ID and a handle to it
    let token_mint_amount = 10000;
    // Amount of tokens given to the wallet
    let wallet_token_amount = 1000;

    // Initialize contract
    token_instance
        .methods()
        .initialize(token_mint_amount, wallet_owner.address().into())
        .call()
        .await
        .unwrap();

    // Contract can be initialized only once
    let is_error = token_instance
        .methods()
        .initialize(token_mint_amount, wallet_owner.address().into())
        .call()
        .await
        .is_err();
    assert!(is_error);

    // Verify the mint amount
    let mint_amount_contract = token_instance
        .methods()
        .get_mint_amount()
        .call()
        .await
        .unwrap();
    assert_eq!(mint_amount_contract.value, token_mint_amount);

    // Verify update mint amount
    token_instance
        .methods()
        .set_mint_amount(1)
        .call()
        .await
        .unwrap();
    let mint_amount_contract = token_instance
        .methods()
        .get_mint_amount()
        .call()
        .await
        .unwrap();
    assert_eq!(mint_amount_contract.value, 1);

    // Update mint amount to the original value
    token_instance
        .methods()
        .set_mint_amount(token_mint_amount)
        .call()
        .await
        .unwrap();

    // Mint some alt tokens
    token_instance
        .methods()
        .mint_coins(token_mint_amount)
        .call()
        .await
        .unwrap();

    // Check the balance of the contract of its own asset
    let result = token_instance.methods().get_balance().call().await.unwrap();
    assert_eq!(result.value, token_mint_amount);

    // Transfer tokens to the wallet
    let address = wallet_owner.address();
    token_instance
        .methods()
        .transfer_coins(wallet_token_amount, address.clone().into())
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
    // Check the balance of the contract of its own asset
    let result = token_instance.methods().get_balance().call().await.unwrap();
    let contract_balance = token_mint_amount - wallet_token_amount;
    assert_eq!(result.value, contract_balance);

    // Burn all minted coins
    token_instance
        .methods()
        .burn_coins(contract_balance)
        .call()
        .await
        .unwrap();

    // Check the balance of the contract of its own asset
    let result = token_instance.methods().get_balance().call().await.unwrap();
    assert_eq!(result.value, 0);

    ////////////////////////////////////////////////////////
    // Test mint and transfer to address
    ////////////////////////////////////////////////////////

    let token_mint1_instance = TestToken::new(token_contract_id.clone(), wallet_mint1.clone());
    // Mint and transfer some alt tokens to the wallet
    token_mint1_instance
        .methods()
        .mint()
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();
    // Mint can be called only once
    let is_error = token_mint1_instance
        .methods()
        .mint()
        .append_variable_outputs(1)
        .call()
        .await
        .is_err();
    assert!(is_error);

    // Inspect the wallet for alt tokens
    // There must be an easier way than this double-cast, no?
    let token_contract_id_casted: ContractId = token_contract_id.clone().into();
    let casted_again: [u8; 32] = token_contract_id_casted.clone().into();
    let alt_token_id = AssetId::from(casted_again);
    let alt_token_balance = wallet_mint1
        .get_asset_balance(&alt_token_id)
        .await
        .unwrap();
    // The wallet shall received the tokens minted
    assert_eq!(alt_token_balance, token_mint_amount);

    //  Other wallet should be able to mint tokens
    let token_mint2_instance = TestToken::new(token_contract_id.clone(), wallet_mint2.clone());
    token_mint2_instance
        .methods()
        .mint()
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap();

    // Inspect the wallet for alt tokens
    let alt_token_balance2 = wallet_mint2
        .get_asset_balance(&alt_token_id)
        .await
        .unwrap();
    // The wallet shall received the tokens minted
    assert_eq!(alt_token_balance2, token_mint_amount);

    // As we mint and transfer the contract balance should be 0
    let result = token_instance.methods().get_balance().call().await.unwrap();
    assert_eq!(result.value, 0);

    ////////////////////////////////////////////////////////
    // Check only owner can call contract
    ////////////////////////////////////////////////////////

    let is_error = token_mint1_instance
        .methods()
        .burn_coins(1)
        .call()
        .await
        .is_err();
    assert!(is_error);
    let is_error = token_mint1_instance
        .methods()
        .mint_coins(1)
        .call()
        .await
        .is_err();
    assert!(is_error);
    let is_error = token_mint1_instance
        .methods()
        .set_mint_amount(1)
        .call()
        .await
        .is_err();
    assert!(is_error);
    let is_error = token_mint1_instance
        .methods()
        .transfer_coins(1, wallet_mint2.address().into())
        .call()
        .await
        .is_err();
    assert!(is_error);
}
