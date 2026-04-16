// Vaultfire ZK Bond Total Proof
//
// Proves: total bond value >= threshold
// WITHOUT revealing individual bond amounts or the exact total.
//
// Private inputs (never revealed):
//   - bond_amounts: Vec<u128> (individual bond values in wei)
//
// Public outputs (committed to journal):
//   - agent_address: [u8; 20]
//   - threshold_wei: u128 (minimum total bond value proven)
//   - bond_count: u32 (number of bonds — reveals count but not values)
//   - meets_threshold: bool
//
// This is critical for the AI Accountability Bonds and AI Partnership Bonds —
// agents can prove they have sufficient skin in the game without revealing
// their exact financial commitment to potential adversaries.

#![no_main]
risc0_zkvm::guest::entry!(main);

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct BondTotalJournal {
    pub agent_address: [u8; 20],
    pub threshold_wei: u128,
    pub bond_count: u32,
    pub meets_threshold: bool,
}

fn main() {
    // ── Private inputs ──
    let bond_amounts: Vec<u128> = env::read();

    // ── Public context ──
    let agent_address: [u8; 20] = env::read();
    let threshold_wei: u128 = env::read();

    // ── Validate inputs ──
    assert!(!bond_amounts.is_empty(), "No bonds provided");
    assert!(
        bond_amounts.len() <= 1000,
        "Too many bonds (max 1000 per proof)"
    );

    // Verify no individual bond is unrealistically large (overflow protection)
    for (i, &amount) in bond_amounts.iter().enumerate() {
        assert!(
            amount <= 1_000_000_000_000_000_000_000_000, // 1M ETH max per bond
            "Bond {} has unrealistic value",
            i
        );
    }

    // ── Core computation (private) ──
    // Sum all bonds — the individual amounts stay hidden
    let total: u128 = bond_amounts.iter().sum();
    let bond_count = bond_amounts.len() as u32;
    let meets_threshold = total >= threshold_wei;

    // ── Commit public outputs ──
    let journal = BondTotalJournal {
        agent_address,
        threshold_wei,
        bond_count,
        meets_threshold,
    };

    env::commit(&journal);
}
