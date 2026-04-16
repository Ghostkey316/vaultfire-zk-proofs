# Vaultfire ZK Trust Attestations

**Real zero-knowledge proofs for on-chain AI agent trust verification.**

Powered by [RISC Zero](https://risczero.com) zkVM — STARK proofs wrapped in Groth16 SNARKs for cheap on-chain verification across Base, Avalanche, Arbitrum, and Polygon.

> **Disclaimer:** This software is experimental and unaudited. Smart contracts have not been formally audited. Use at your own risk. Nothing in this repository constitutes financial, legal, or investment advice.

---

## What This Does

Vaultfire agents can prove trust properties about themselves **without revealing the underlying data**:

| Proof Type | What's Proven (Public) | What Stays Private |
|------------|----------------------|-------------------|
| **Trust Score** | Score >= threshold | The actual score |
| **Registration Age** | Registered >= N days | The exact registration date |
| **Bond Total** | Total bonds >= threshold | Individual bond amounts |
| **Cross-Chain Trust** | Chain A facts verified on Chain B | Source chain state details |

### Why This Matters

Without ZK proofs, any partner can read an agent's exact trust score, bond amounts, and registration date directly from the blockchain. With ZK proofs, agents control what they reveal — proving they meet requirements without exposing competitive intelligence.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  Guest Programs (Rust — runs inside RISC Zero zkVM)     │
│                                                         │
│  trust_score_proof.rs    → proves score >= threshold     │
│  registration_age_proof.rs → proves age >= min_days      │
│  bond_total_proof.rs     → proves bonds >= threshold     │
│  cross_chain_trust.rs    → proves Chain A state on B     │
│                                                         │
│  Private inputs NEVER leave the zkVM.                   │
│  Only journal (public outputs) are committed.           │
└────────────────────────┬────────────────────────────────┘
                         │ Generates Groth16 receipt
                         ▼
┌─────────────────────────────────────────────────────────┐
│  Host / Publisher (Rust — runs locally or via Boundless) │
│                                                         │
│  1. Feeds private inputs to guest                       │
│  2. Generates proof (local GPU or Boundless)             │
│  3. Outputs seal + journal for on-chain submission      │
└────────────────────────┬────────────────────────────────┘
                         │ Submits proof on-chain
                         ▼
┌─────────────────────────────────────────────────────────┐
│  VaultfireTrustAttestation.sol (Solidity — on-chain)    │
│                                                         │
│  Calls IRiscZeroVerifier.verify(seal, imageId, hash)    │
│  Stores attestation if proof is valid                   │
│  Other contracts can query: isAgentTrustScoreVerified() │
│                                                         │
│  Gas cost: ~250,000 per verification (pennies on L2s)   │
└─────────────────────────────────────────────────────────┘
```

---

## Deployments

VaultfireTrustAttestation is live on all four chains:

| Chain | Chain ID | VaultfireTrustAttestation | RISC Zero VerifierRouter |
|-------|----------|---------------------------|---------------------------|
| **Base** | 8453 | `0x472dF1dD6D8218D0BF748e910E32861dAb88EDA6` | `0x0b144E07A0826182B6b59788c34b32Bfa86Fb711` |
| **Avalanche** | 43114 | `0xf92baef9523BC264144F80F9c31D5c5C017c6Da8` | `0x0b144E07A0826182B6b59788c34b32Bfa86Fb711` |
| **Arbitrum** | 42161 | `0xE2f75A4B14ffFc1f9C2b1ca22Fdd6877E5BD5045` | `0x0b144E07A0826182B6b59788c34b32Bfa86Fb711` |
| **Polygon** | 137 | `0x8568F4020FCD55915dB3695558dD6D2532599e56` | `0xdBAD523786971B75A7b1c1CFdCfECDeb59A764B9` |

### Image IDs (generated from compiled guest programs)

| Guest Program | Image ID |
|---------------|----------|
| `trust_score_proof` | `0x3ccef0f0d6aa3bc53e3b226f021d9b15a2ab4d9b15724fa838fb1ea4a30f527d` |
| `registration_age_proof` | `0x6f8562d11114397c3990314fbeba9139f9c5412cb5fe8b7573c691454e5d4dd8` |
| `bond_total_proof` | `0x3bc8520b7d8f99777848e93455f280d91c8a696e5f63c50da9c04061c7c99aef` |

---

## Project Structure

```
vaultfire-zk-proofs/
├── Cargo.toml                          # Workspace root
├── methods/
│   ├── guest/src/bin/
│   │   ├── trust_score_proof.rs        # ZK: score >= threshold
│   │   ├── registration_age_proof.rs   # ZK: age >= min_days
│   │   ├── bond_total_proof.rs         # ZK: bonds >= threshold
│   │   └── cross_chain_trust.rs        # ZK: Steel cross-chain
│   ├── src/lib.rs                      # Exports ELF + IMAGE_IDs
│   └── build.rs                        # risc0-build integration
├── apps/
│   └── src/bin/
│       └── publisher.rs                # CLI proof generator
├── contracts/
│   ├── src/
│   │   └── VaultfireTrustAttestation.sol
│   ├── script/
│   │   └── Deploy.s.sol
│   └── foundry.toml
└── README.md
```

---

## Quick Start

### Prerequisites

```bash
# Install RISC Zero toolchain
curl -L https://risczero.com/install | bash
rzup install

# Install Foundry (for Solidity contracts)
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Build

```bash
# Build Rust guest programs + host
cargo build --release

# Install Solidity dependencies
cd contracts
forge install risc0/risc0-ethereum
forge install foundry-rs/forge-std
forge build
```

### Generate Proofs

```bash
# Development mode (fast, no real proof)
RISC0_DEV_MODE=1 cargo run --release --bin publisher -- \
  trust-score --score 87 --threshold 80 --agent 0xYOUR_AGENT_ADDRESS

# Production mode (real Groth16 proof — requires GPU or Boundless)
RISC0_DEV_MODE=0 cargo run --release --bin publisher -- \
  trust-score --score 87 --threshold 80 --agent 0xYOUR_AGENT_ADDRESS

# Using Boundless decentralized proving (replaces Bonsai, which shut down Dec 2025)
# See https://docs.boundless.network for setup
BOUNDLESS_MARKET=https://market.boundless.network \
BOUNDLESS_WALLET_KEY=your_key \
cargo run --release --bin publisher -- \
  trust-score --score 87 --threshold 80 --agent 0xYOUR_AGENT_ADDRESS
```

### Deploy Contracts

```bash
cd contracts

# Deploy to Base
DEPLOYER_KEY=$YOUR_KEY forge script script/Deploy.s.sol \
  --rpc-url https://mainnet.base.org --broadcast

# Deploy to Avalanche
DEPLOYER_KEY=$YOUR_KEY forge script script/Deploy.s.sol \
  --rpc-url https://api.avax.network/ext/bc/C/rpc --broadcast

# Deploy to Arbitrum
DEPLOYER_KEY=$YOUR_KEY forge script script/Deploy.s.sol \
  --rpc-url https://arb1.arbitrum.io/rpc --broadcast

# Deploy to Polygon
DEPLOYER_KEY=$YOUR_KEY forge script script/Deploy.s.sol \
  --rpc-url https://polygon-rpc.com --broadcast
```

---

## Security

- **Private keys**: NEVER hardcoded anywhere. Always passed as environment variables.
- **Image IDs**: Cryptographically bind contracts to specific guest programs. A proof from a modified program will be rejected.
- **Replay protection**: Each proof can only be submitted once (journal hash tracking).
- **Staleness checks**: Registration age proofs must be generated within 24 hours.
- **Overflow protection**: Bond amounts capped at 1M ETH per bond.
- **Dev mode safety**: Production contracts should use `disable-dev-mode` feature flag to prevent fake proofs.

---

## Gas Costs

| Operation | Gas | Cost on L2 |
|-----------|-----|------------|
| Trust score proof verification | ~250,000 | ~$0.002 |
| Registration age proof verification | ~250,000 | ~$0.002 |
| Bond total proof verification | ~260,000 | ~$0.002 |
| Cross-chain proof verification | ~300,000 | ~$0.003 |

Proof generation (off-chain): $0.04–$0.40 via Boundless (RISC Zero's decentralized proving marketplace).

---

## Integration with Vaultfire Protocol

The `VaultfireTrustAttestation` contract exposes query functions that other Vaultfire contracts can call:

```solidity
// Can other contracts check if an agent is verified?
attestation.isAgentTrustScoreVerified(agent, minThreshold, maxAge)
attestation.isAgentRegistrationAgeVerified(agent, minDays, maxAge)
attestation.isAgentBondTotalVerified(agent, minThresholdWei, maxAge)
attestation.isAgentCrossChainVerified(agent, sourceChainId, maxAge)
```

This enables composable trust — Partnership Bonds can require ZK-verified trust scores, Accountability Bonds can require ZK-verified bond totals, and cross-chain bridges can require ZK-verified source chain attestations.

---

## Existing Vaultfire Infrastructure

This ZK proof system integrates with Vaultfire's existing 16-contract stack deployed across all four chains:

- **ERC8004IdentityRegistry** — Agent identity (what we prove age for)
- **ERC8004ReputationRegistry** — Trust scores (what we prove thresholds for)
- **AIPartnershipBondsV2** / **AIAccountabilityBondsV2** — Bond data (what we prove totals for)
- **PrivacyGuarantees** — On-chain consent and data deletion (complementary privacy layer)
- **AntiSurveillance** — Anti-monitoring protections
- **VaultfireTeleporterBridge** — Cross-chain bridge (what cross-chain proofs enhance)

---

## License

MIT

---

*Built by the Vaultfire Protocol team. Powered by RISC Zero.*
