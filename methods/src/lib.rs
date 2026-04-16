// This file is auto-populated by risc0-build at compile time.
// It exports ELF binaries and IMAGE_ID constants for each guest program:
//
//   TRUST_SCORE_PROOF_ELF, TRUST_SCORE_PROOF_ID
//   REGISTRATION_AGE_PROOF_ELF, REGISTRATION_AGE_PROOF_ID
//   BOND_TOTAL_PROOF_ELF, BOND_TOTAL_PROOF_ID
//   CROSS_CHAIN_TRUST_ELF, CROSS_CHAIN_TRUST_ID

include!(concat!(env!("OUT_DIR"), "/methods.rs"));
