use aurora_engine_types::types::{Address, Wei};
use near_sdk::{AccountId, Gas, NearToken};

use crate::{
    internal::{CHAIN_ID, MAX_YOCTO_NEAR, account_id_to_address},
    tests::utils::{self, crypto, into_ethabi_token, nep141, test_context::TestContext},
    types::Action,
};

// An external caller gets its deposit back if the cross-contract call fails.
#[tokio::test]
async fn test_caller_refunds_contract_call() -> anyhow::Result<()> {
    let TestContext {
        worker,
        wallet_contract,
        wallet_sk,
        address_registrar,
        ..
    } = TestContext::new().await?;

    let caller = worker.root_account()?;
    let deposit_amount = NearToken::from_near(3);
    let create_tx = |receiver_id: &AccountId, nonce: u64| {
        let method = "register";
        let args = br#"{"account_id": "birchmd.near"}"#;
        let action = Action::FunctionCall {
            receiver_id: receiver_id.to_string(),
            method_name: method.into(),
            args: args.to_vec(),
            gas: Gas::from_tgas(10).as_gas(),
            yocto_near: 0,
        };
        utils::create_signed_transaction(
            nonce,
            receiver_id,
            Wei::new_u128(deposit_amount.as_yoctonear() / (MAX_YOCTO_NEAR as u128)),
            action,
            &wallet_sk,
        )
    };

    // External caller gets a refund when the cross-contract call fails
    let pre_tx_account_balance = caller.view_account().await?.balance;
    let receiver_id: AccountId = "fake.near".parse()?;
    let result = wallet_contract
        .rlp_execute_from(
            &caller,
            receiver_id.as_str(),
            &create_tx(&receiver_id, 0),
            deposit_amount,
        )
        .await?;
    assert!(!result.success);
    let post_tx_account_balance = caller.view_account().await?.balance;
    assert!(
        pre_tx_account_balance.as_yoctonear() - post_tx_account_balance.as_yoctonear()
            < deposit_amount.as_yoctonear()
    );

    // External caller does not get a refund when their tokens are spent
    let pre_tx_account_balance = post_tx_account_balance;
    let receiver_id = address_registrar.id();
    let result = wallet_contract
        .rlp_execute_from(
            &caller,
            receiver_id.as_str(),
            &create_tx(receiver_id, 1),
            deposit_amount,
        )
        .await?;
    assert!(result.success);
    let post_tx_account_balance = caller.view_account().await?.balance;
    assert!(
        pre_tx_account_balance.as_yoctonear() - post_tx_account_balance.as_yoctonear()
            >= deposit_amount.as_yoctonear()
    );

    Ok(())
}

// An external caller gets its deposit back if the address check fails.
#[tokio::test]
async fn test_caller_refunds_failed_address_check() -> anyhow::Result<()> {
    let TestContext {
        worker,
        wallet_contract,
        wallet_sk,
        wallet_address,
        address_registrar,
        ..
    } = TestContext::new().await?;

    // Deploy a NEP-141 contract and register its address.
    // Registering should prevent a lazy relayer from setting the target incorrectly.
    let token_contract = nep141::Nep141::deploy(&worker).await?;
    let register_output: Option<String> = address_registrar
        .call("register")
        .args_json(serde_json::json!({
            "account_id": token_contract.contract.id().as_str()
        }))
        .max_gas()
        .deposit(NearToken::from_millinear(1))
        .transact()
        .await?
        .json()?;
    let token_address: [u8; 20] = hex::decode(
        register_output
            .as_ref()
            .unwrap()
            .strip_prefix("0x")
            .unwrap(),
    )?
    .try_into()
    .unwrap();
    assert_eq!(
        token_address,
        account_id_to_address(&token_contract.contract.id().as_str().parse().unwrap(),).0
    );

    // The user submits a transaction to interact with the NEP-141 contract.
    let transaction = aurora_engine_transactions::eip_2930::Transaction2930 {
        nonce: 0.into(),
        gas_price: 0.into(),
        gas_limit: 0.into(),
        to: Some(Address::from_array(token_address)),
        value: Wei::zero(),
        data: [
            crate::eth_emulation::ERC20_BALANCE_OF_SELECTOR.to_vec(),
            ethabi::encode(&[into_ethabi_token(&wallet_address)]),
        ]
        .concat(),
        chain_id: CHAIN_ID,
        access_list: Vec::new(),
    };
    let signed_transaction = crypto::sign_transaction(transaction, &wallet_sk);

    let deposit_amount = NearToken::from_near(3);
    let caller = worker.root_account()?;
    let pre_tx_account_balance = caller.view_account().await?.balance;
    // Improperly set target (using address instead of named account)
    let target = {
        let wallet_account_id = wallet_contract.inner.id().as_str();
        let suffix_index = wallet_account_id.find('.').unwrap();
        let suffix = &wallet_account_id[suffix_index..];
        format!("{}{}", register_output.unwrap(), suffix)
    };
    let result = wallet_contract
        .rlp_execute_from(&caller, &target, &signed_transaction, deposit_amount)
        .await?;
    assert!(!result.success);
    let post_tx_account_balance = caller.view_account().await?.balance;
    assert!(
        pre_tx_account_balance.as_yoctonear() - post_tx_account_balance.as_yoctonear()
            < deposit_amount.as_yoctonear()
    );

    Ok(())
}

// An external caller gets its deposit back if the NEP-141 balance check fails.
#[tokio::test]
async fn test_caller_refunds_failed_nep_141_balance_check() -> anyhow::Result<()> {
    let TestContext {
        worker,
        wallet_contract,
        wallet_sk,
        ..
    } = TestContext::new().await?;

    // This contract does not exit, so the call to `storage_balance_of` will fail.
    let token_contract: AccountId = "not_a_token.near".parse().unwrap();
    let other_address = account_id_to_address(&token_contract);

    // Create an emulated ERC-20 transfer interacting with the fake token contract.
    let transaction = aurora_engine_transactions::eip_2930::Transaction2930 {
        nonce: 0.into(),
        gas_price: 0.into(),
        gas_limit: 0.into(),
        to: Some(Address::new(account_id_to_address(&token_contract))),
        value: Wei::zero(),
        data: [
            crate::eth_emulation::ERC20_TRANSFER_SELECTOR,
            ethabi::encode(&[
                into_ethabi_token(&other_address),
                ethabi::Token::Uint(NearToken::from_near(5).as_yoctonear().into()),
            ])
            .as_slice(),
            b"hello_world", // Include a memo
        ]
        .concat(),
        chain_id: CHAIN_ID,
        access_list: Vec::new(),
    };
    let signed_transaction = crypto::sign_transaction(transaction, &wallet_sk);

    let deposit_amount = NearToken::from_near(3);
    let caller = worker.root_account()?;
    let pre_tx_account_balance = caller.view_account().await?.balance;
    let result = wallet_contract
        .rlp_execute_from(
            &caller,
            token_contract.as_str(),
            &signed_transaction,
            deposit_amount,
        )
        .await?;
    assert!(!result.success);
    let post_tx_account_balance = caller.view_account().await?.balance;
    assert!(
        pre_tx_account_balance.as_yoctonear() - post_tx_account_balance.as_yoctonear()
            < deposit_amount.as_yoctonear()
    );

    Ok(())
}
