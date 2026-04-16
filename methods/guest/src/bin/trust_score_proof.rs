// Vaultfire ZK Trust Score Proof
//
// Proves: agent's trust score >= threshold
// WITHOUT revealing the actual trust score.
//
// Private inputs (never revealed):
//   - trust_score: u64 (the agent's actual trust score)
//
// Public outputs (committed to journal):
//   - agent_address: [u8; 20] (which agent this is about)
//   - threshold: u64 (what threshold was proven)
//   - above_threshold: bool (whether score >= threshold)
//
// On-chain: verifier sees "agent X has trust score >= 80"
//           but NEVER learns the actual score (e.g., 87)

#![no_main]
risc0_zkvm::guest::entry!(main);

use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

/// Journal — the public output committed to the proof.
/// This is what the Solidity contract will decode and trust.
#[derive(Serialize, Deserialize, Debug)]
pub struct TrustScoreJournal {
    /// The agent's Ethereum address (20 bytes)
    pub agent_address: [u8; 20],
    /// The threshold that was proven against
    pub threshold: u64,
    /// Whether the agent's score meets or exceeds the threshold
    pub above_threshold: bool,
}

fn main() {
    // ── Read private inputs from host ──
    // These are NEVER included in the proof output.
    let trust_score: u64 = env::read();

    // ── Read public context ──
    let agent_address: [u8; 20] = env::read();
    let threshold: u64 = env::read();

    // ── Validate inputs ──
    // Trust scores are 0-100 in Vaultfire
    assert!(trust_score <= 100, "Invalid trust score: must be 0-100");
    assert!(threshold <= 100, "Invalid threshold: must be 0-100");

    // ── Core computation (private) ──
    let above_threshold = trust_score >= threshold;

    // ── Commit public outputs to journal ──
    // Only these values become part of the proof.
    // The actual trust_score is NEVER revealed.
    let journal = TrustScoreJournal {
        agent_address,
        threshold,
        above_threshold,
    };

    env::commit(&journal);
}
