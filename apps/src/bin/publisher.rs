// Vaultfire ZK Proof Publisher
//
// Generates RISC Zero proofs for trust attestations and outputs
// the proof data needed for on-chain submission.
//
// Usage:
//   # Trust score proof (prove score >= threshold without revealing score)
//   publisher trust-score --score 87 --threshold 80 --agent 0xA054...84F
//
//   # Registration age proof (prove registered >= N days)
//   publisher registration-age --registered-at 1701388800 --min-days 90 --agent 0xA054...84F
//
//   # Bond total proof (prove total bonds >= threshold)
//   publisher bond-total --bonds "2400000000000000000,1800000000000000000" --threshold 3000000000000000000 --agent 0xA054...84F
//
// IMPORTANT:
//   - Set RISC0_DEV_MODE=1 for development (skips real proof generation)
//   - Set RISC0_DEV_MODE=0 for production (generates real Groth16 proofs)
//   - For cloud proving, set BONSAI_API_URL and BONSAI_API_KEY
//   - NEVER pass private keys as CLI arguments — use environment variables

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use risc0_zkvm::{default_prover, ExecutorEnv};
use serde::{Deserialize, Serialize};

// Import the guest program ELF binaries and IMAGE_IDs
use vaultfire_zk_methods::{
    TRUST_SCORE_PROOF_ELF, TRUST_SCORE_PROOF_ID,
    REGISTRATION_AGE_PROOF_ELF, REGISTRATION_AGE_PROOF_ID,
    BOND_TOTAL_PROOF_ELF, BOND_TOTAL_PROOF_ID,
};

/* ═══════════════════════════════════════════════════════════════════════
   CLI DEFINITION
═══════════════════════════════════════════════════════════════════════ */

#[derive(Parser)]
#[command(name = "vaultfire-zk")]
#[command(about = "Generate ZK trust attestation proofs for the Vaultfire protocol")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Prove trust score >= threshold
    TrustScore {
        /// Agent's actual trust score (PRIVATE — never revealed in proof)
        #[arg(long)]
        score: u64,
        /// Threshold to prove against (PUBLIC)
        #[arg(long)]
        threshold: u64,
        /// Agent's Ethereum address (0x...)
        #[arg(long)]
        agent: String,
    },
    /// Prove registration age >= min_days
    RegistrationAge {
        /// Unix timestamp of agent registration (PRIVATE)
        #[arg(long)]
        registered_at: u64,
        /// Minimum days to prove (PUBLIC)
        #[arg(long)]
        min_days: u64,
        /// Agent's Ethereum address (0x...)
        #[arg(long)]
        agent: String,
    },
    /// Prove total bond value >= threshold
    BondTotal {
        /// Comma-separated bond amounts in wei (PRIVATE)
        #[arg(long)]
        bonds: String,
        /// Minimum total threshold in wei (PUBLIC)
        #[arg(long)]
        threshold: u128,
        /// Agent's Ethereum address (0x...)
        #[arg(long)]
        agent: String,
    },
}

/* ═══════════════════════════════════════════════════════════════════════
   JOURNAL TYPES (must match guest programs exactly)
═══════════════════════════════════════════════════════════════════════ */

#[derive(Serialize, Deserialize, Debug)]
struct TrustScoreJournal {
    agent_address: [u8; 20],
    threshold: u64,
    above_threshold: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct RegistrationAgeJournal {
    agent_address: [u8; 20],
    min_days: u64,
    proof_timestamp: u64,
    meets_requirement: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct BondTotalJournal {
    agent_address: [u8; 20],
    threshold_wei: u128,
    bond_count: u32,
    meets_threshold: bool,
}

/* ═══════════════════════════════════════════════════════════════════════
   HELPERS
═══════════════════════════════════════════════════════════════════════ */

fn parse_address(addr: &str) -> Result<[u8; 20]> {
    let addr = addr.strip_prefix("0x").unwrap_or(addr);
    let bytes = hex::decode(addr).context("Invalid hex address")?;
    if bytes.len() != 20 {
        anyhow::bail!("Address must be 20 bytes, got {}", bytes.len());
    }
    let mut result = [0u8; 20];
    result.copy_from_slice(&bytes);
    Ok(result)
}

fn current_unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/* ═══════════════════════════════════════════════════════════════════════
   PROOF GENERATION
═══════════════════════════════════════════════════════════════════════ */

fn generate_trust_score_proof(score: u64, threshold: u64, agent: [u8; 20]) -> Result<()> {
    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Vaultfire ZK Trust Score Proof                 ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Agent:     0x{}  ║", hex::encode(agent));
    println!("║  Threshold: {:<38}║", threshold);
    println!("║  Score:     [PRIVATE — not in proof]            ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // Build executor environment with inputs
    let env = ExecutorEnv::builder()
        .write(&score)?       // Private: actual trust score
        .write(&agent)?       // Public context: agent address
        .write(&threshold)?   // Public context: threshold
        .build()?;

    println!("Generating proof...");
    let prover = default_prover();
    let prove_info = prover.prove(env, TRUST_SCORE_PROOF_ELF)?;
    let receipt = prove_info.receipt;

    // Verify locally
    receipt.verify(TRUST_SCORE_PROOF_ID)
        .context("Local verification failed")?;
    println!("Local verification: PASSED");

    // Decode journal
    let journal: TrustScoreJournal = receipt.journal.decode()?;
    println!();
    println!("═══ PROOF OUTPUT ═══");
    println!("  Agent:           0x{}", hex::encode(journal.agent_address));
    println!("  Threshold:       {}", journal.threshold);
    println!("  Above threshold: {}", journal.above_threshold);
    println!();

    // Output data for on-chain submission
    println!("═══ ON-CHAIN SUBMISSION DATA ═══");
    println!("  Image ID:    0x{}", hex::encode(TRUST_SCORE_PROOF_ID));
    println!("  Journal hex: 0x{}", hex::encode(receipt.journal.bytes));
    println!("  Seal length: {} bytes", receipt.inner.groth16().map(|g| g.seal.len()).unwrap_or(0));

    Ok(())
}

fn generate_registration_age_proof(registered_at: u64, min_days: u64, agent: [u8; 20]) -> Result<()> {
    let current_time = current_unix_timestamp();

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Vaultfire ZK Registration Age Proof             ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Agent:     0x{}  ║", hex::encode(agent));
    println!("║  Min days:  {:<38}║", min_days);
    println!("║  Reg date:  [PRIVATE — not in proof]            ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    let env = ExecutorEnv::builder()
        .write(&registered_at)?   // Private: exact registration timestamp
        .write(&agent)?           // Public context
        .write(&min_days)?        // Public context
        .write(&current_time)?    // Public context (for staleness)
        .build()?;

    println!("Generating proof...");
    let prover = default_prover();
    let prove_info = prover.prove(env, REGISTRATION_AGE_PROOF_ELF)?;
    let receipt = prove_info.receipt;

    receipt.verify(REGISTRATION_AGE_PROOF_ID)
        .context("Local verification failed")?;
    println!("Local verification: PASSED");

    let journal: RegistrationAgeJournal = receipt.journal.decode()?;
    println!();
    println!("═══ PROOF OUTPUT ═══");
    println!("  Agent:             0x{}", hex::encode(journal.agent_address));
    println!("  Min days:          {}", journal.min_days);
    println!("  Proof timestamp:   {}", journal.proof_timestamp);
    println!("  Meets requirement: {}", journal.meets_requirement);
    println!();

    println!("═══ ON-CHAIN SUBMISSION DATA ═══");
    println!("  Image ID:    0x{}", hex::encode(REGISTRATION_AGE_PROOF_ID));
    println!("  Journal hex: 0x{}", hex::encode(receipt.journal.bytes));

    Ok(())
}

fn generate_bond_total_proof(bonds_str: &str, threshold: u128, agent: [u8; 20]) -> Result<()> {
    let bonds: Vec<u128> = bonds_str
        .split(',')
        .map(|s| s.trim().parse::<u128>())
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Invalid bond amounts — use comma-separated wei values")?;

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  Vaultfire ZK Bond Total Proof                  ║");
    println!("╠══════════════════════════════════════════════════╣");
    println!("║  Agent:     0x{}  ║", hex::encode(agent));
    println!("║  Threshold: {} wei{}", threshold, " ".repeat(29_usize.saturating_sub(threshold.to_string().len())));
    println!("║  Bonds:     {} bonds [AMOUNTS PRIVATE]{}║", bonds.len(), " ".repeat(19_usize.saturating_sub(bonds.len().to_string().len())));
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    let env = ExecutorEnv::builder()
        .write(&bonds)?          // Private: individual bond amounts
        .write(&agent)?          // Public context
        .write(&threshold)?      // Public context
        .build()?;

    println!("Generating proof...");
    let prover = default_prover();
    let prove_info = prover.prove(env, BOND_TOTAL_PROOF_ELF)?;
    let receipt = prove_info.receipt;

    receipt.verify(BOND_TOTAL_PROOF_ID)
        .context("Local verification failed")?;
    println!("Local verification: PASSED");

    let journal: BondTotalJournal = receipt.journal.decode()?;
    println!();
    println!("═══ PROOF OUTPUT ═══");
    println!("  Agent:          0x{}", hex::encode(journal.agent_address));
    println!("  Threshold:      {} wei", journal.threshold_wei);
    println!("  Bond count:     {}", journal.bond_count);
    println!("  Meets threshold: {}", journal.meets_threshold);
    println!();

    println!("═══ ON-CHAIN SUBMISSION DATA ═══");
    println!("  Image ID:    0x{}", hex::encode(BOND_TOTAL_PROOF_ID));
    println!("  Journal hex: 0x{}", hex::encode(receipt.journal.bytes));

    Ok(())
}

/* ═══════════════════════════════════════════════════════════════════════
   MAIN
═══════════════════════════════════════════════════════════════════════ */

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::TrustScore { score, threshold, agent } => {
            let addr = parse_address(&agent)?;
            generate_trust_score_proof(score, threshold, addr)?;
        }
        Commands::RegistrationAge { registered_at, min_days, agent } => {
            let addr = parse_address(&agent)?;
            generate_registration_age_proof(registered_at, min_days, addr)?;
        }
        Commands::BondTotal { bonds, threshold, agent } => {
            let addr = parse_address(&agent)?;
            generate_bond_total_proof(&bonds, threshold, addr)?;
        }
    }

    Ok(())
}
