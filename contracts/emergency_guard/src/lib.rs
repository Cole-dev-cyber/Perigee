#![no_std]
//! # EmergencyGuard — standardized emergency controls for Soroban contracts
//!
//! Standalone circuit-breaker contract ([CONTRACT-5](#499)).
//!
//! All of the reusable logic — granular pause bitmask, multi-signature admin
//! committee, threshold validation, typed events — now lives in the shared
//! [`Perigee-guards`] library crate as [`Guard`], which is re-exported here as
//! [`DefaultEmergencyGuard`]. This crate only:
//!
//! 1. exposes the `EmergencyGuard` contract entry points (a thin delegation
//!    layer over the core), and
//! 2. re-exports the guard core, so host contracts that already embed guard
//!    storage via `emergency_guard` keep compiling and behaving identically.
//!
//! Storage layout, event topics, error codes, and public entry points are
//! unchanged; no on-chain state migration is required.

#[cfg(feature = "contract")]
use soroban_sdk::{contract, contractimpl};
use soroban_sdk::{Address, Env, Vec};

/// Re-export the reusable guard core and all of its public types, events and
/// emitters so existing imports (`emergency_guard::{DefaultEmergencyGuard,
/// PauseType, GuardError, …}`) keep resolving unchanged.
pub use Perigee_guards::*;
/// Backward-compatible name for the canonical [`Guard`] core.
pub use Perigee_guards::Guard as DefaultEmergencyGuard;

#[cfg_attr(feature = "contract", contract)]
pub struct EmergencyGuard;

#[cfg_attr(feature = "contract", contractimpl)]
impl EmergencyGuard {
    /// Initialize the emergency guard with a list of admins and required threshold.
    pub fn initialize(env: Env, admins: Vec<Address>, threshold: u32) -> Result<(), GuardError> {
        DefaultEmergencyGuard::init_guard(&env, admins, threshold)
    }

    /// Returns the raw pause-state bitmask.
    pub fn get_pause_state(env: Env) -> u32 {
        DefaultEmergencyGuard::get_pause_state(&env)
    }

    /// Check if an operation is paused.
    pub fn is_paused(env: Env, operation: u32) -> bool {
        DefaultEmergencyGuard::is_paused(&env, operation)
    }

    /// Gas-optimized pause probe: single storage read + inline bitwise AND.
    #[inline(always)]
    pub fn is_paused_ref(env: &Env, operation: u32) -> bool {
        DefaultEmergencyGuard::is_paused(env, operation)
    }

    /// Panics when the requested operation bit is set in the pause bitmask.
    #[inline(always)]
    pub fn ensure_not_paused(env: &Env, operation: u32) {
        DefaultEmergencyGuard::ensure_not_paused(env, operation);
    }

    /// Set pause state for a specific operation (any single admin can do this).
    pub fn set_pause(
        env: Env,
        admin: Address,
        operation: u32,
        paused: bool,
    ) -> Result<(), GuardError> {
        DefaultEmergencyGuard::set_pause_by_admin(&env, &admin, operation, paused)
    }

    /// Emergency pause all operations (requires multi-sig approval).
    pub fn emergency_pause(env: Env, approvers: Vec<Address>) -> Result<(), GuardError> {
        DefaultEmergencyGuard::emergency_pause_all(&env, approvers)
    }

    /// Resume all operations (requires multi-sig approval).
    pub fn resume(env: Env, approvers: Vec<Address>) -> Result<(), GuardError> {
        DefaultEmergencyGuard::resume_all(&env, approvers)
    }

    /// Check if an address is an admin.
    pub fn is_admin_public(env: Env, addr: Address) -> bool {
        DefaultEmergencyGuard::is_admin(&env, &addr)
    }

    /// Add new admin (multi-sig required).
    pub fn add_admin(
        env: Env,
        approvers: Vec<Address>,
        new_admin: Address,
    ) -> Result<(), GuardError> {
        DefaultEmergencyGuard::add_admin(&env, approvers, new_admin)
    }

    /// Remove admin (multi-sig required).
    pub fn remove_admin(
        env: Env,
        approvers: Vec<Address>,
        admin: Address,
    ) -> Result<(), GuardError> {
        DefaultEmergencyGuard::remove_admin(&env, approvers, admin)
    }

    /// Rotate admin (multi-sig required).
    pub fn rotate_admin(
        env: Env,
        approvers: Vec<Address>,
        old_admin: Address,
        new_admin: Address,
    ) -> Result<(), GuardError> {
        DefaultEmergencyGuard::rotate_admin(&env, approvers, old_admin, new_admin)
    }

    /// Get list of current admins.
    pub fn get_admins(env: Env) -> Vec<Address> {
        DefaultEmergencyGuard::get_admins(&env)
    }

    /// Get required signature threshold.
    pub fn get_threshold(env: Env) -> u32 {
        DefaultEmergencyGuard::get_threshold(&env)
    }

    /// Public wrapper to validate approvers against the stored threshold.
    pub fn validate_multi_sig(env: Env, approvers: Vec<Address>) -> Result<(), GuardError> {
        DefaultEmergencyGuard::validate_multi_sig(&env, &approvers)
    }
}

#[cfg(test)]
mod test;