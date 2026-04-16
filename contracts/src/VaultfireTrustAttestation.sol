// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";

/// @title VaultfireTrustAttestation
/// @notice On-chain ZK trust attestation registry for the Vaultfire protocol.
///         Accepts RISC Zero Groth16 proofs that attest to agent trust properties
///         WITHOUT revealing the underlying private data.
///
///         Deployed on: Base, Avalanche, Arbitrum, Polygon
///         Uses canonical RISC Zero VerifierRouter on each chain.
///
/// @dev Proof types:
///      1. Trust Score  — proves score >= threshold (score stays private)
///      2. Registration Age — proves age >= min_days (exact date stays private)
///      3. Bond Total — proves bonds >= threshold (individual amounts stay private)
///      4. Cross-Chain — proves Chain A facts, verifiable on Chain B
///
/// @custom:disclaimer This contract is experimental and unaudited. Use at your own risk.
contract VaultfireTrustAttestation {
    /* ═══════════════════════════════════════════════════════════════════════
       STATE
    ═══════════════════════════════════════════════════════════════════════ */

    /// @notice The RISC Zero VerifierRouter — already deployed on all chains
    IRiscZeroVerifier public immutable verifier;

    /// @notice Protocol admin (deployer)
    address public admin;

    /// @notice Image IDs for each guest program (set after `cargo build`)
    /// @dev These cryptographically bind the contract to specific guest binaries.
    ///      A proof generated from a different program will be rejected.
    bytes32 public trustScoreImageId;
    bytes32 public registrationAgeImageId;
    bytes32 public bondTotalImageId;
    bytes32 public crossChainImageId;

    /* ═══════════════════════════════════════════════════════════════════════
       ATTESTATION STORAGE
    ═══════════════════════════════════════════════════════════════════════ */

    struct TrustScoreAttestation {
        uint64 threshold;
        uint64 verifiedAt;
        bool aboveThreshold;
    }

    struct RegistrationAgeAttestation {
        uint64 minDays;
        uint64 proofTimestamp;
        uint64 verifiedAt;
        bool meetsRequirement;
    }

    struct BondTotalAttestation {
        uint128 thresholdWei;
        uint32 bondCount;
        uint64 verifiedAt;
        bool meetsThreshold;
    }

    struct CrossChainAttestation {
        uint64 sourceChainId;
        uint64 sourceBlockNumber;
        uint64 reputationThreshold;
        uint64 verifiedAt;
        bool isActive;
        bool meetsReputationThreshold;
    }

    /// @notice Trust score attestations by agent address
    mapping(address => TrustScoreAttestation) public trustScoreAttestations;

    /// @notice Registration age attestations by agent address
    mapping(address => RegistrationAgeAttestation) public registrationAgeAttestations;

    /// @notice Bond total attestations by agent address
    mapping(address => BondTotalAttestation) public bondTotalAttestations;

    /// @notice Cross-chain attestations by agent address + source chain
    mapping(address => mapping(uint64 => CrossChainAttestation)) public crossChainAttestations;

    /// @notice Replay protection — track processed journal hashes
    mapping(bytes32 => bool) public processedProofs;

    /* ═══════════════════════════════════════════════════════════════════════
       EVENTS
    ═══════════════════════════════════════════════════════════════════════ */

    event TrustScoreProven(address indexed agent, uint64 threshold, bool result);
    event RegistrationAgeProven(address indexed agent, uint64 minDays, bool result);
    event BondTotalProven(address indexed agent, uint128 thresholdWei, uint32 bondCount, bool result);
    event CrossChainProven(address indexed agent, uint64 sourceChainId, uint64 sourceBlock, bool active, bool meetsThreshold);
    event ImageIdUpdated(string proofType, bytes32 newImageId);

    /* ═══════════════════════════════════════════════════════════════════════
       ERRORS
    ═══════════════════════════════════════════════════════════════════════ */

    error Unauthorized();
    error ImageIdNotSet();
    error ProofAlreadyProcessed();
    error StaleProof();
    error InvalidJournalLength();

    /* ═══════════════════════════════════════════════════════════════════════
       CONSTRUCTOR
    ═══════════════════════════════════════════════════════════════════════ */

    /// @param _verifier The RISC Zero VerifierRouter address for this chain
    constructor(address _verifier) {
        verifier = IRiscZeroVerifier(_verifier);
        admin = msg.sender;
    }

    /* ═══════════════════════════════════════════════════════════════════════
       ADMIN — Set Image IDs (after cargo build generates them)
    ═══════════════════════════════════════════════════════════════════════ */

    modifier onlyAdmin() {
        if (msg.sender != admin) revert Unauthorized();
        _;
    }

    function setTrustScoreImageId(bytes32 _imageId) external onlyAdmin {
        trustScoreImageId = _imageId;
        emit ImageIdUpdated("trust_score", _imageId);
    }

    function setRegistrationAgeImageId(bytes32 _imageId) external onlyAdmin {
        registrationAgeImageId = _imageId;
        emit ImageIdUpdated("registration_age", _imageId);
    }

    function setBondTotalImageId(bytes32 _imageId) external onlyAdmin {
        bondTotalImageId = _imageId;
        emit ImageIdUpdated("bond_total", _imageId);
    }

    function setCrossChainImageId(bytes32 _imageId) external onlyAdmin {
        crossChainImageId = _imageId;
        emit ImageIdUpdated("cross_chain", _imageId);
    }

    function transferAdmin(address _newAdmin) external onlyAdmin {
        admin = _newAdmin;
    }

    /* ═══════════════════════════════════════════════════════════════════════
       PROOF SUBMISSION — Trust Score
    ═══════════════════════════════════════════════════════════════════════ */

    /// @notice Submit a ZK proof that an agent's trust score >= threshold
    /// @param journalData ABI-encoded journal from the guest program
    /// @param seal The RISC Zero Groth16 proof bytes
    function submitTrustScoreProof(
        bytes calldata journalData,
        bytes calldata seal
    ) external {
        if (trustScoreImageId == bytes32(0)) revert ImageIdNotSet();

        // Replay protection
        bytes32 journalHash = sha256(journalData);
        if (processedProofs[journalHash]) revert ProofAlreadyProcessed();

        // Verify the ZK proof — reverts if invalid
        verifier.verify(seal, trustScoreImageId, journalHash);

        // Mark as processed
        processedProofs[journalHash] = true;

        // Decode journal (matches TrustScoreJournal in guest)
        (
            bytes20 agentAddr,
            uint64 threshold,
            bool aboveThreshold
        ) = abi.decode(journalData, (bytes20, uint64, bool));

        address agent = address(agentAddr);

        // Store attestation
        trustScoreAttestations[agent] = TrustScoreAttestation({
            threshold: threshold,
            verifiedAt: uint64(block.timestamp),
            aboveThreshold: aboveThreshold
        });

        emit TrustScoreProven(agent, threshold, aboveThreshold);
    }

    /* ═══════════════════════════════════════════════════════════════════════
       PROOF SUBMISSION — Registration Age
    ═══════════════════════════════════════════════════════════════════════ */

    /// @notice Submit a ZK proof that an agent has been registered >= min_days
    /// @param journalData ABI-encoded journal from the guest program
    /// @param seal The RISC Zero Groth16 proof bytes
    function submitRegistrationAgeProof(
        bytes calldata journalData,
        bytes calldata seal
    ) external {
        if (registrationAgeImageId == bytes32(0)) revert ImageIdNotSet();

        bytes32 journalHash = sha256(journalData);
        if (processedProofs[journalHash]) revert ProofAlreadyProcessed();

        verifier.verify(seal, registrationAgeImageId, journalHash);
        processedProofs[journalHash] = true;

        (
            bytes20 agentAddr,
            uint64 minDays,
            uint64 proofTimestamp,
            bool meetsRequirement
        ) = abi.decode(journalData, (bytes20, uint64, uint64, bool));

        // Staleness check: proof must be generated within last 24 hours
        if (block.timestamp - proofTimestamp > 86400) revert StaleProof();

        address agent = address(agentAddr);

        registrationAgeAttestations[agent] = RegistrationAgeAttestation({
            minDays: minDays,
            proofTimestamp: proofTimestamp,
            verifiedAt: uint64(block.timestamp),
            meetsRequirement: meetsRequirement
        });

        emit RegistrationAgeProven(agent, minDays, meetsRequirement);
    }

    /* ═══════════════════════════════════════════════════════════════════════
       PROOF SUBMISSION — Bond Total
    ═══════════════════════════════════════════════════════════════════════ */

    /// @notice Submit a ZK proof that an agent's total bonds >= threshold
    /// @param journalData ABI-encoded journal from the guest program
    /// @param seal The RISC Zero Groth16 proof bytes
    function submitBondTotalProof(
        bytes calldata journalData,
        bytes calldata seal
    ) external {
        if (bondTotalImageId == bytes32(0)) revert ImageIdNotSet();

        bytes32 journalHash = sha256(journalData);
        if (processedProofs[journalHash]) revert ProofAlreadyProcessed();

        verifier.verify(seal, bondTotalImageId, journalHash);
        processedProofs[journalHash] = true;

        (
            bytes20 agentAddr,
            uint128 thresholdWei,
            uint32 bondCount,
            bool meetsThreshold
        ) = abi.decode(journalData, (bytes20, uint128, uint32, bool));

        address agent = address(agentAddr);

        bondTotalAttestations[agent] = BondTotalAttestation({
            thresholdWei: thresholdWei,
            bondCount: bondCount,
            verifiedAt: uint64(block.timestamp),
            meetsThreshold: meetsThreshold
        });

        emit BondTotalProven(agent, thresholdWei, bondCount, meetsThreshold);
    }

    /* ═══════════════════════════════════════════════════════════════════════
       PROOF SUBMISSION — Cross-Chain Trust
    ═══════════════════════════════════════════════════════════════════════ */

    /// @notice Submit a ZK proof of cross-chain trust attestation
    /// @param journalData ABI-encoded journal from the Steel guest program
    /// @param seal The RISC Zero Groth16 proof bytes
    function submitCrossChainProof(
        bytes calldata journalData,
        bytes calldata seal
    ) external {
        if (crossChainImageId == bytes32(0)) revert ImageIdNotSet();

        bytes32 journalHash = sha256(journalData);
        if (processedProofs[journalHash]) revert ProofAlreadyProcessed();

        verifier.verify(seal, crossChainImageId, journalHash);
        processedProofs[journalHash] = true;

        (
            bytes20 agentAddr,
            uint64 sourceChainId,
            bool isActive,
            bool meetsReputationThreshold,
            uint64 reputationThreshold,
            uint64 sourceBlockNumber
        ) = abi.decode(journalData, (bytes20, uint64, bool, bool, uint64, uint64));

        address agent = address(agentAddr);

        crossChainAttestations[agent][sourceChainId] = CrossChainAttestation({
            sourceChainId: sourceChainId,
            sourceBlockNumber: sourceBlockNumber,
            reputationThreshold: reputationThreshold,
            verifiedAt: uint64(block.timestamp),
            isActive: isActive,
            meetsReputationThreshold: meetsReputationThreshold
        });

        emit CrossChainProven(agent, sourceChainId, sourceBlockNumber, isActive, meetsReputationThreshold);
    }

    /* ═══════════════════════════════════════════════════════════════════════
       QUERY FUNCTIONS — For other Vaultfire contracts to check attestations
    ═══════════════════════════════════════════════════════════════════════ */

    /// @notice Check if an agent has a fresh trust score attestation above a minimum
    /// @param agent The agent address to check
    /// @param minThreshold The minimum threshold the attestation must prove
    /// @param maxAge Maximum age of the attestation in seconds (0 = no limit)
    function isAgentTrustScoreVerified(
        address agent,
        uint64 minThreshold,
        uint256 maxAge
    ) external view returns (bool) {
        TrustScoreAttestation memory att = trustScoreAttestations[agent];
        if (!att.aboveThreshold) return false;
        if (att.threshold < minThreshold) return false;
        if (maxAge > 0 && block.timestamp - att.verifiedAt > maxAge) return false;
        return true;
    }

    /// @notice Check if an agent has a fresh registration age attestation
    function isAgentRegistrationAgeVerified(
        address agent,
        uint64 minDays,
        uint256 maxAge
    ) external view returns (bool) {
        RegistrationAgeAttestation memory att = registrationAgeAttestations[agent];
        if (!att.meetsRequirement) return false;
        if (att.minDays < minDays) return false;
        if (maxAge > 0 && block.timestamp - att.verifiedAt > maxAge) return false;
        return true;
    }

    /// @notice Check if an agent has a fresh bond total attestation
    function isAgentBondTotalVerified(
        address agent,
        uint128 minThresholdWei,
        uint256 maxAge
    ) external view returns (bool) {
        BondTotalAttestation memory att = bondTotalAttestations[agent];
        if (!att.meetsThreshold) return false;
        if (att.thresholdWei < minThresholdWei) return false;
        if (maxAge > 0 && block.timestamp - att.verifiedAt > maxAge) return false;
        return true;
    }

    /// @notice Check if an agent has a fresh cross-chain attestation from a specific source chain
    function isAgentCrossChainVerified(
        address agent,
        uint64 sourceChainId,
        uint256 maxAge
    ) external view returns (bool) {
        CrossChainAttestation memory att = crossChainAttestations[agent][sourceChainId];
        if (!att.isActive) return false;
        if (!att.meetsReputationThreshold) return false;
        if (maxAge > 0 && block.timestamp - att.verifiedAt > maxAge) return false;
        return true;
    }
}
