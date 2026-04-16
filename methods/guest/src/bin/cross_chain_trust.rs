// Vaultfire ZK Cross-Chain Trust Attestation
//
// Proves: a fact about an agent's state on Chain A,
//         verifiable on Chain B, using RISC Zero Steel.
//
// Steel reads EVM storage via Merkle proofs inside the zkVM,
// producing a proof that can be verified on any chain with a
// RISC Zero verifier deployed.
//
// Example: Prove on Avalanche that an agent registered on Base
//          has a trust score >= 80, without Chain B needing to
//          query Chain A directly.
//
// This enables trustless cross-chain reputation portability —
// a core Vaultfire capability that no other protocol offers.
//
// NOTE: This guest requires the "steel" feature flag:
//       cargo build --features steel

#![no_main]
risc0_zkvm::guest::entry!(main);

use alloy_primitives::{Address, U256};
use alloy_sol_types::sol;
use risc0_steel::{ethereum::EthEvmInput, Contract};
use risc0_zkvm::guest::env;
use serde::{Deserialize, Serialize};

// Define the on-chain interface we're reading from Chain A
sol! {
    interface IVaultfireIdentityRegistry {
        function getAgent(address agent)
            external
            view
            returns (string memory agentURI, bool active, string memory agentType, uint256 registeredAt);
        function isAgentActive(address agent) external view returns (bool);
    }

    interface IVaultfireReputationRegistry {
        function getReputation(address agent)
            external
            view
            returns (uint256 averageRating, uint256 totalFeedbacks, uint256 verifiedFeedbacks, uint256 lastUpdated);
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CrossChainJournal {
    /// The agent being attested
    pub agent_address: [u8; 20],
    /// Source chain ID (where the data lives)
    pub source_chain_id: u64,
    /// Whether the agent is active on the source chain
    pub is_active: bool,
    /// Whether the agent meets the reputation threshold
    pub meets_reputation_threshold: bool,
    /// The reputation threshold proven
    pub reputation_threshold: u64,
    /// Block number of the source chain state
    pub source_block_number: u64,
}

fn main() {
    // ── Read Steel EVM input (Merkle proofs of Chain A state) ──
    let evm_input: EthEvmInput = env::read();

    // ── Read parameters ──
    let identity_registry: [u8; 20] = env::read();
    let reputation_registry: [u8; 20] = env::read();
    let agent_address_bytes: [u8; 20] = env::read();
    let reputation_threshold: u64 = env::read();
    let source_chain_id: u64 = env::read();
    let source_block_number: u64 = env::read();

    // Convert to alloy types
    let agent = Address::from(agent_address_bytes);
    let id_registry = Address::from(identity_registry);
    let rep_registry = Address::from(reputation_registry);

    // ── Verify Chain A state using Steel ──
    // This internally verifies Merkle proofs — if the state doesn't match
    // the claimed block, the proof generation will fail.
    let evm_env = evm_input.into_env();

    // Query identity registry on Chain A
    let id_contract = Contract::new(id_registry, &evm_env);
    let is_active_call = IVaultfireIdentityRegistry::isAgentActiveCall {
        agent,
    };
    let is_active = id_contract.call_builder(&is_active_call).call()._0;

    // Query reputation registry on Chain A
    let rep_contract = Contract::new(rep_registry, &evm_env);
    let rep_call = IVaultfireReputationRegistry::getReputationCall {
        agent,
    };
    let rep_result = rep_contract.call_builder(&rep_call).call();
    let average_rating = rep_result.averageRating;

    // Check threshold
    let meets_threshold = average_rating >= U256::from(reputation_threshold);

    // ── Commit public outputs ──
    let journal = CrossChainJournal {
        agent_address: agent_address_bytes,
        source_chain_id,
        is_active,
        meets_reputation_threshold: meets_threshold,
        reputation_threshold,
        source_block_number,
    };

    env::commit(&journal);
}
