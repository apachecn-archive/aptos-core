// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{transaction::SignedTransaction, LocalAccount},
};
use cached_packages::aptos_stdlib;
use forge::{reconfig, LocalSwarm, NodeExt, Swarm};
use rand::random;

pub async fn create_and_fund_account(swarm: &'_ mut dyn Swarm, amount: u64) -> LocalAccount {
    let mut info = swarm.aptos_public_info();
    info.create_and_fund_user_account(amount).await.unwrap()
}

pub async fn transfer_coins_non_blocking(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn = sender.sign_with_transaction_builder(transaction_factory.payload(
        aptos_stdlib::aptos_coin_transfer(receiver.address(), amount),
    ));

    client.submit(&txn).await.unwrap();
    txn
}

pub async fn transfer_coins(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    amount: u64,
) -> SignedTransaction {
    let txn =
        transfer_coins_non_blocking(client, transaction_factory, sender, receiver, amount).await;

    client.wait_for_signed_transaction(&txn).await.unwrap();

    txn
}

pub async fn transfer_and_reconfig(
    client: &RestClient,
    transaction_factory: &TransactionFactory,
    root_account: &mut LocalAccount,
    sender: &mut LocalAccount,
    receiver: &LocalAccount,
    num_transfers: usize,
) {
    for _ in 0..num_transfers {
        // Reconfigurations have a 20% chance of being executed
        if random::<u16>() % 5 == 0 {
            reconfig(client, transaction_factory, root_account).await;
        }

        transfer_coins(client, transaction_factory, sender, receiver, 1).await;
    }
}

pub async fn assert_balance(client: &RestClient, account: &LocalAccount, balance: u64) {
    let on_chain_balance = client
        .get_account_balance(account.address())
        .await
        .unwrap()
        .into_inner();

    assert_eq!(on_chain_balance.get(), balance);
}

/// This module provides useful functions for operating, handling and managing
/// AptosSwarm instances. It is particularly useful for working with tests that
/// require a SmokeTestEnvironment, as it provides a generic interface across
/// AptosSwarms, regardless of if the swarm is a validator swarm, validator full
/// node swarm, or a public full node swarm.
#[cfg(test)]
pub mod swarm_utils {
    use aptos_config::config::{NodeConfig, WaypointConfig};
    use aptos_types::waypoint::Waypoint;

    pub fn insert_waypoint(node_config: &mut NodeConfig, waypoint: Waypoint) {
        node_config.base.waypoint = WaypointConfig::FromConfig(waypoint);
    }
}

/// This helper function creates 3 new accounts, mints funds, transfers funds
/// between the accounts and verifies that these operations succeed.
pub async fn check_create_mint_transfer(swarm: &mut LocalSwarm) {
    let client = swarm.validators().next().unwrap().rest_client();

    // Create account 0, mint 10 coins and check balance
    let mut account_0 = create_and_fund_account(swarm, 10).await;
    assert_balance(&client, &account_0, 10).await;

    // Create account 1, mint 1 coin, transfer 3 coins from account 0 to 1, check balances
    let account_1 = create_and_fund_account(swarm, 1).await;
    transfer_coins(
        &client,
        &swarm.chain_info().transaction_factory(),
        &mut account_0,
        &account_1,
        3,
    )
    .await;

    assert_balance(&client, &account_0, 7).await;
    assert_balance(&client, &account_1, 4).await;

    // Create account 2, mint 15 coins and check balance
    let account_2 = create_and_fund_account(swarm, 15).await;
    assert_balance(&client, &account_2, 15).await;
}
