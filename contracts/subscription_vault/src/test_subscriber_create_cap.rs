//! Adversarial coverage for the subscriber creation-rate cap (#987).

use crate::{Error, SubscriptionVault, SubscriptionVaultClient};
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, Symbol};

const INTERVAL: u64 = 60;
const AMOUNT: i128 = 1_000_000;

fn setup() -> (Env, Address, SubscriptionVaultClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let contract_id = env.register(SubscriptionVault, ());
    let client = SubscriptionVaultClient::new(&env, &contract_id);
    client.init(&token, &7, &admin, &AMOUNT, &0);

    (env, admin, client)
}

fn create_subscription(
    client: &SubscriptionVaultClient,
    subscriber: &Address,
    merchant: &Address,
) -> u32 {
    client.create_subscription(
        subscriber,
        merchant,
        &AMOUNT,
        &INTERVAL,
        &false,
        &None,
        &None,
        &None::<u32>,
        &None::<Symbol>,
    )
}

#[test]
fn admin_cap_limits_creation_and_rejected_attempt_leaves_state_unchanged() {
    let (env, admin, client) = setup();
    let subscriber = Address::generate(&env);

    client.set_subscriber_create_cap(&admin, &1);
    assert_eq!(client.get_subscriber_create_cap(), 1);
    create_subscription(&client, &subscriber, &Address::generate(&env));
    assert_eq!(client.get_subscriber_active_count(&subscriber), 1);

    let rejected = client.try_create_subscription(
        &subscriber,
        &Address::generate(&env),
        &AMOUNT,
        &INTERVAL,
        &false,
        &None,
        &None,
        &None::<u32>,
        &None::<Symbol>,
    );
    assert_eq!(rejected, Err(Ok(Error::SubscriberRateLimited)));
    assert_eq!(client.get_subscriber_active_count(&subscriber), 1);

    // If the rejected call had incremented its rate-limit window, this retry
    // would still fail after increasing the cap to two.
    client.set_subscriber_create_cap(&admin, &2);
    create_subscription(&client, &subscriber, &Address::generate(&env));
    assert_eq!(client.get_subscriber_active_count(&subscriber), 2);
}

#[test]
fn zero_cap_blocks_creation_without_mutating_subscription_state() {
    let (env, admin, client) = setup();
    let subscriber = Address::generate(&env);

    client.set_subscriber_create_cap(&admin, &0);
    assert_eq!(client.get_subscriber_create_cap(), 0);
    let rejected = client.try_create_subscription(
        &subscriber,
        &Address::generate(&env),
        &AMOUNT,
        &INTERVAL,
        &false,
        &None,
        &None,
        &None::<u32>,
        &None::<Symbol>,
    );
    assert_eq!(rejected, Err(Ok(Error::SubscriberRateLimited)));
    assert_eq!(client.get_subscriber_active_count(&subscriber), 0);
}

#[test]
fn only_admin_can_change_cap_and_rejection_preserves_prior_value() {
    let (env, admin, client) = setup();
    let stranger = Address::generate(&env);

    client.set_subscriber_create_cap(&admin, &u32::MAX);
    let rejected = client.try_set_subscriber_create_cap(&stranger, &0);
    assert_eq!(rejected, Err(Ok(Error::Unauthorized)));
    assert_eq!(client.get_subscriber_create_cap(), u32::MAX);
}
