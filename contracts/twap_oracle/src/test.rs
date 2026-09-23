#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

use crate::{Error, TwapOracle, TwapOracleClient};

#[test]
fn test_initialize() {
    let e = Env::default();
    let contract_id = e.register(TwapOracle, ());
    let client = TwapOracleClient::new(&e, &contract_id);

    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);

    client.initialize(&token_a, &token_b, &60);

    // Try to initialize again (direct call so the Err is observable).
    let result = e.as_contract(&contract_id, || {
        TwapOracle::initialize(e.clone(), token_a.clone(), token_b.clone(), 60)
    });
    assert_eq!(result, Err(Error::AlreadyInitialized));

    let (a, b) = client.get_tokens();
    assert_eq!(a, token_a);
    assert_eq!(b, token_b);
}

#[test]
fn test_update_and_get_twap() {
    let e = Env::default();
    e.ledger().with_mut(|li| li.timestamp = 1000);

    let contract_id = e.register(TwapOracle, ());
    let client = TwapOracleClient::new(&e, &contract_id);

    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);

    client.initialize(&token_a, &token_b, &10);

    // First update
    client.update_price(&100);
    assert_eq!(client.get_twap(), 0); // No time elapsed yet

    // Advance time
    e.ledger().with_mut(|li| li.timestamp = 1010);

    // Second update
    client.update_price(&110);
    // TWAP = (100 * 10) / 10 = 100
    assert_eq!(client.get_twap(), 100);

    // Advance time again
    e.ledger().with_mut(|li| li.timestamp = 1025);

    // Third update
    client.update_price(&120);
    // Cumulative = 100*10 + 110*15 = 1000 + 1650 = 2650
    // Total time = 10 + 15 = 25
    // TWAP = 2650 / 25 = 106
    assert_eq!(client.get_twap(), 106);
}

#[test]
fn test_update_too_soon() {
    let e = Env::default();
    e.ledger().with_mut(|li| li.timestamp = 1000);

    let contract_id = e.register(TwapOracle, ());
    let client = TwapOracleClient::new(&e, &contract_id);

    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);

    client.initialize(&token_a, &token_b, &60);
    client.update_price(&100);

    assert_eq!(
        e.as_contract(&contract_id, || TwapOracle::update_price(e.clone(), 110)),
        Err(Error::InsufficientTimeElapsed)
    );

    // Advance time by 50 seconds
    e.ledger().with_mut(|li| li.timestamp = 1050);
    assert_eq!(
        e.as_contract(&contract_id, || TwapOracle::update_price(e.clone(), 110)),
        Err(Error::InsufficientTimeElapsed)
    );

    // Advance to 1060
    e.ledger().with_mut(|li| li.timestamp = 1060);
    assert_eq!(
        e.as_contract(&contract_id, || TwapOracle::update_price(e.clone(), 110)),
        Ok(())
    );
}

#[test]
fn test_invalid_price() {
    let e = Env::default();
    let contract_id = e.register(TwapOracle, ());
    let client = TwapOracleClient::new(&e, &contract_id);

    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);

    client.initialize(&token_a, &token_b, &60);

    let zero = || TwapOracle::update_price(e.clone(), 0);
    let negative = || TwapOracle::update_price(e.clone(), -1);
    assert_eq!(e.as_contract(&contract_id, zero), Err(Error::InvalidPrice));
    assert_eq!(e.as_contract(&contract_id, negative), Err(Error::InvalidPrice));
}

#[test]
fn test_not_initialized() {
    let e = Env::default();
    let contract_id = e.register(TwapOracle, ());
    let client = TwapOracleClient::new(&e, &contract_id);

    let update = || TwapOracle::update_price(e.clone(), 100);
    assert_eq!(e.as_contract(&contract_id, update), Err(Error::NotInitialized));
    assert_eq!(client.get_twap(), 0);
}

#[test]
fn test_state_getter_reflects_updates() {
    let e = Env::default();
    e.ledger().with_mut(|li| li.timestamp = 5000);

    let contract_id = e.register(TwapOracle, ());
    let client = TwapOracleClient::new(&e, &contract_id);

    let token_a = Address::generate(&e);
    let token_b = Address::generate(&e);

    client.initialize(&token_a, &token_b, &10);
    client.update_price(&100);

    // Advance time and update again.
    e.ledger().with_mut(|li| li.timestamp = 5010);
    client.update_price(&110);

    let state = client.get_state();
    assert_eq!(state.last_price, 110);
    assert_eq!(state.last_update_timestamp, 5010);
    assert_eq!(state.cumulative_price, 100 * 10);
    assert_eq!(state.total_time, 10);
}