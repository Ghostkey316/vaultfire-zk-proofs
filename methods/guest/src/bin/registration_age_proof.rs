// Vaultfire ZK Registration Age Proof
//
// Proves: agent has been registered >= min_days
// WITHOUT revealing the exact registration timestamp.
//
// Private inputs (never revealed):
//   - registered_at_unix: u64 (exact registration timestamp)
//
// Public outputs (committed to journal):
//   - agent_address: [u8; 20]
//   - min_days: u64 (minimum days proven)
//   - proof_timestamp: u64 (when the proof was generated — for staleness checks)
//   - meets_requirement: bool
//
// On-chain: the contract also checks that proof_timestamp is recent
//           (within 24 hours of block.timestamp) to prevent replay attacks.

#![no_main]
risc0_zkvm::guest::entry!(main);

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

const SECONDS_PER_DAY: u64 = 86400;

#[derive(Serialize, Deserialize, Debug)]
pub struct RegistrationAgeJournal {
    pub agent_address: [u8; 20],
    pub min_days: u64,
    pub proof_timestamp: u64,
    pub meets_requirement: bool,
}

fn main() {
    // ── Private inputs ──
    let registered_at_unix: u64 = env::read();

    // ── Public context ──
    let agent_address: [u8; 20] = env::read();
    let min_days: u64 = env::read();
    let current_time_unix: u64 = env::read();

    // ── Validate inputs ──
    assert!(
        registered_at_unix <= current_time_unix,
        "Registration timestamp cannot be in the future"
    );
    assert!(
        current_time_unix > 1_700_000_000,
        "Current time seems invalid (before Nov 2023)"
    );

    // ── Core computation (private) ──
    let days_registered = (current_time_unix - registered_at_unix) / SECONDS_PER_DAY;
    let meets_requirement = days_registered >= min_days;

    // ── Commit public outputs ──
    let journal = RegistrationAgeJournal {
        agent_address,
        min_days,
        proof_timestamp: current_time_unix,
        meets_requirement,
    };

    env::commit(&journal);
}
