//! # Test Module
//!
//! Skeleton tests for the Haven Registry contract.
//!
//! These tests use the Soroban SDK's `testutils` feature to simulate
//! contract interactions in a local test environment.
//!
//! ## Running Tests
//!
//! ```bash
//! cd contracts
//! cargo test
//! ```
//!
//! ## Contributing
//!
//! Each test below is a stub with a clear description of what it should verify.
//! Implementing these tests is a great `good-first-issue` for contributors!

#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env, String};
use soroban_sdk::testutils::Events;

use crate::{HavenRegistry, HavenRegistryClient};

/// Helper: create a test environment and deploy the contract.
fn setup() -> (Env, HavenRegistryClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(HavenRegistry, ());
    let client = HavenRegistryClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);

    (env, client, admin)
}

/// Helper: generate a fake hashed IMEI (32 bytes).
fn fake_hashed_imei(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
    ])
}

// ---------------------------------------------------------------------------
// Device Registration Tests
// ---------------------------------------------------------------------------

#[test]
fn test_register_device() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");

    let device = client.register_device(&owner, &hashed_imei, &model);

    assert_eq!(device.owner, owner);
    assert_eq!(device.is_stolen, false);
    assert_eq!(device.device_model, model);

    // Verify the DeviceRegistered event was emitted
    let events = env.events().all();
    assert!(!events.is_empty(), "Expected at least one event to be emitted");

    let event = events.last().unwrap();

    // Event structure: (contract_address, topics, data)
    let (_contract_id, topics, _data) = event;

    // Verify topics contain "dev_reg" and "register"
    assert_eq!(topics.len(), 2);

    // TODO: Verify the device count was incremented
}

#[test]
fn test_register_device_emits_event() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "Samsung Galaxy S24");

    // Register the device
    client.register_device(&owner, &hashed_imei, &model);

    // Verify event emission
    let events = env.events().all();
    assert_eq!(events.len(), 1, "Expected exactly one event to be emitted");

    let event = events.first().unwrap();
    let (_contract_id, topics, _data) = event;

    // Verify the event has the correct topic structure
    assert_eq!(topics.len(), 2, "Expected two topics: dev_reg and register");

    // The topics should be symbols for "dev_reg" and "register"
    // We verify the count and structure, actual symbol validation would require
    // converting Val to Symbol which is more complex in tests
}

#[test]
#[should_panic(expected = "device already registered")]
fn test_register_device_duplicate() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");

    client.register_device(&owner, &hashed_imei, &model);
    // This should panic — same IMEI registered twice
    client.register_device(&owner, &hashed_imei, &model);
}

#[test]
fn test_get_device() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "Samsung Galaxy S24");

    client.register_device(&owner, &hashed_imei, &model);
    let device = client.get_device(&hashed_imei);

    assert_eq!(device.owner, owner);
    assert_eq!(device.device_model, model);
}

// ---------------------------------------------------------------------------
// Killswitch Tests
// ---------------------------------------------------------------------------

#[test]
fn test_report_stolen() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    client.register_device(&owner, &hashed_imei, &model);
    client.report_stolen(&owner, &hashed_imei, &1_000_000i128, &contact);

    let device = client.get_device(&hashed_imei);
    assert_eq!(device.is_stolen, true);

    let bounty = client.get_bounty(&hashed_imei);
    assert_eq!(bounty, 1_000_000i128);

    // TODO: Verify the actual token transfer occurred
    // TODO: Verify the DeviceStolen event was emitted
}

#[test]
#[should_panic(expected = "device already reported as stolen")]
fn test_report_stolen_twice() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    client.register_device(&owner, &hashed_imei, &model);
    client.report_stolen(&owner, &hashed_imei, &1_000_000i128, &contact);
    // Should panic — already reported
    client.report_stolen(&owner, &hashed_imei, &1_000_000i128, &contact);
}

// ---------------------------------------------------------------------------
// Killswitch — Minimum Bounty Validation Tests (Issue #4)
// ---------------------------------------------------------------------------

#[test]
#[should_panic(expected = "bounty amount must be positive")]
fn test_report_stolen_zero_bounty() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    client.register_device(&owner, &hashed_imei, &model);
    // Should panic — zero bounty amount is rejected
    client.report_stolen(&owner, &hashed_imei, &0i128, &contact);
}

#[test]
#[should_panic(expected = "bounty amount must be positive")]
fn test_report_stolen_negative_bounty() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    client.register_device(&owner, &hashed_imei, &model);
    // Should panic — negative bounty amount is rejected
    client.report_stolen(&owner, &hashed_imei, &-1i128, &contact);
}

#[test]
#[should_panic(expected = "bounty amount below minimum threshold")]
fn test_report_stolen_below_minimum() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    client.register_device(&owner, &hashed_imei, &model);
    // Should panic — 50 is below the MIN_BOUNTY_AMOUNT of 100
    client.report_stolen(&owner, &hashed_imei, &50i128, &contact);
}

#[test]
fn test_report_stolen_minimum_bounty() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    client.register_device(&owner, &hashed_imei, &model);
    // Exactly the minimum bounty amount — should succeed
    client.report_stolen(&owner, &hashed_imei, &100i128, &contact);

    let device = client.get_device(&hashed_imei);
    assert_eq!(device.is_stolen, true);

    let bounty = client.get_bounty(&hashed_imei);
    assert_eq!(bounty, 100i128);
}

// ---------------------------------------------------------------------------
// Recovery Tests
// ---------------------------------------------------------------------------

#[test]
fn test_confirm_recovery() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let finder = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");
    let contact = String::from_str(&env, "owner@email.com");

    // Register → Report Stolen → Recover
    client.register_device(&owner, &hashed_imei, &model);
    client.report_stolen(&owner, &hashed_imei, &1_000_000i128, &contact);
    client.confirm_recovery(&owner, &hashed_imei, &finder);

    let device = client.get_device(&hashed_imei);
    assert_eq!(device.is_stolen, false);

    let bounty = client.get_bounty(&hashed_imei);
    assert_eq!(bounty, 0i128);

    // TODO: Verify the bounty was transferred to the finder
    // TODO: Verify the DeviceRecovered event was emitted
}

#[test]
#[should_panic(expected = "device is not reported as stolen")]
fn test_recover_not_stolen() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let finder = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");

    client.register_device(&owner, &hashed_imei, &model);
    // Should panic — device isn't stolen
    client.confirm_recovery(&owner, &hashed_imei, &finder);
}

// ---------------------------------------------------------------------------
// Insurance Tests
// ---------------------------------------------------------------------------

#[test]
fn test_file_insurance_claim() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let insurer = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");

    client.register_device(&owner, &hashed_imei, &model);
    client.file_insurance_claim(&owner, &hashed_imei, &insurer);

    let device = client.get_device(&hashed_imei);
    assert_eq!(device.insurer, Some(insurer));

    // TODO: Verify the InsuranceClaimed event was emitted
    // TODO: Verify the owner can no longer report stolen or recover
}

#[test]
#[should_panic(expected = "insurance claim already filed")]
fn test_file_insurance_claim_twice() {
    let (env, client, _admin) = setup();
    let owner = Address::generate(&env);
    let insurer = Address::generate(&env);
    let hashed_imei = fake_hashed_imei(&env);
    let model = String::from_str(&env, "iPhone 15 Pro");

    client.register_device(&owner, &hashed_imei, &model);
    client.file_insurance_claim(&owner, &hashed_imei, &insurer);
    // Should panic — already claimed
    client.file_insurance_claim(&owner, &hashed_imei, &insurer);
}
