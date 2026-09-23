#![no_std]
use soroban_sdk::{contract, contracterror, contractimpl, contracttype, Address, Env};

#[cfg(test)]
mod test;

/// Errors returned by the `TwapOracle` contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    InvalidPrice = 3,
    InsufficientTimeElapsed = 4,
}

/// Storage keys used by the TWAP oracle contract.
#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    TokenA,
    TokenB,
    MinUpdateIntervalSeconds,
    /// Single instance key holding the whole TWAP accumulator state.
    ///
    /// Replaces the four separate `CumulativePrice` / `TotalTime` /
    /// `LastUpdateTimestamp` / `LastPrice` keys: `update_price` now performs
    /// one read + one write of instance storage instead of four.
    State,
}

/// The TWAP accumulator, packed into a single instance-storage entry.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwapStorage {
    pub cumulative_price: i128,
    pub total_time: u64,
    pub last_update_timestamp: u64,
    pub last_price: i128,
}

impl Default for TwapStorage {
    fn default() -> Self {
        Self {
            cumulative_price: 0,
            total_time: 0,
            last_update_timestamp: 0,
            last_price: 0,
        }
    }
}

pub trait PriceOracle {
    fn latest_price(e: Env) -> i128;
}

#[contract]
/// TWAP Price Oracle for token pairs.
pub struct TwapOracle;

#[contractimpl]
impl TwapOracle {
    /// Initializes the TWAP oracle with token pair addresses and minimum update interval.
    ///
    /// # Parameters
    /// - `e`: Soroban environment.
    /// - `token_a`: Contract address of token A.
    /// - `token_b`: Contract address of token B.
    /// - `min_update_interval_seconds`: Minimum time in seconds between updates.
    ///
    /// # Returns
    /// - `Ok(())` when initialization succeeds.
    /// - `Err(Error::AlreadyInitialized)` if already initialized.
    pub fn initialize(
        e: Env,
        token_a: Address,
        token_b: Address,
        min_update_interval_seconds: u64,
    ) -> Result<(), Error> {
        if e.storage().instance().has(&DataKey::TokenA) {
            return Err(Error::AlreadyInitialized);
        }
        e.storage().instance().set(&DataKey::TokenA, &token_a);
        e.storage().instance().set(&DataKey::TokenB, &token_b);
        e.storage().instance().set(&DataKey::MinUpdateIntervalSeconds, &min_update_interval_seconds);
        // One write for the whole accumulator state.
        e.storage().instance().set(&DataKey::State, &TwapStorage::default());
        Ok(())
    }

    /// Updates the TWAP accumulator with a new price.
    /// Only allows updates after the minimum interval has elapsed.
    ///
    /// # Parameters
    /// - `e`: Soroban environment.
    /// - `current_price`: The current price of token_b in terms of token_a (scaled appropriately).
    ///
    /// # Returns
    /// - `Ok(())` on success.
    /// - `Err(Error::NotInitialized)` if not initialized.
    /// - `Err(Error::InvalidPrice)` if price <= 0.
    /// - `Err(Error::InsufficientTimeElapsed)` if not enough time has passed since last update.
    pub fn update_price(e: Env, current_price: i128) -> Result<(), Error> {
        if !e.storage().instance().has(&DataKey::TokenA) {
            return Err(Error::NotInitialized);
        }
        if current_price <= 0 {
            return Err(Error::InvalidPrice);
        }

        let now = e.ledger().timestamp();
        let mut state = Self::read_state(&e);
        let min_interval: u64 = e
            .storage()
            .instance()
            .get(&DataKey::MinUpdateIntervalSeconds)
            .unwrap_or(0);

        if state.last_update_timestamp > 0
            && now - state.last_update_timestamp < min_interval
        {
            return Err(Error::InsufficientTimeElapsed);
        }

        let elapsed = if state.last_update_timestamp == 0 {
            0
        } else {
            now - state.last_update_timestamp
        };
        state.cumulative_price += state.last_price * elapsed as i128;
        state.total_time += elapsed;
        state.last_update_timestamp = now;
        state.last_price = current_price;

        // One write for the whole accumulator state.
        Self::write_state(&e, &state);

        Ok(())
    }

    /// Returns the Time-Weighted Average Price since initialization.
    ///
    /// # Parameters
    /// - `e`: Soroban environment.
    ///
    /// # Returns
    /// - The TWAP as i128, or 0 if no updates.
    pub fn get_twap(e: Env) -> i128 {
        let state = Self::read_state(&e);
        if state.total_time == 0 {
            0
        } else {
            state.cumulative_price / state.total_time as i128
        }
    }

    /// Returns the token pair addresses.
    pub fn get_tokens(e: Env) -> (Address, Address) {
        let token_a: Address = e.storage().instance().get(&DataKey::TokenA).unwrap();
        let token_b: Address = e.storage().instance().get(&DataKey::TokenB).unwrap();
        (token_a, token_b)
    }

    /// Returns the packed TWAP accumulator state.
    pub fn get_state(e: Env) -> TwapStorage {
        Self::read_state(&e)
    }

    /// Reads the packed accumulator state, defaulting to all zeros when the
    /// contract is not initialized or no update has happened yet.
    fn read_state(e: &Env) -> TwapStorage {
        e.storage()
            .instance()
            .get(&DataKey::State)
            .unwrap_or_default()
    }

    /// Writes the packed accumulator state in a single instance-storage write.
    fn write_state(e: &Env, state: &TwapStorage) {
        e.storage().instance().set(&DataKey::State, state);
    }
}

#[contractimpl]
impl PriceOracle for TwapOracle {
    /// Returns the latest TWAP price.
    fn latest_price(e: Env) -> i128 {
        Self::get_twap(e)
    }
}