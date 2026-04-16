// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import {Script, console} from "forge-std/Script.sol";
import {VaultfireTrustAttestation} from "../src/VaultfireTrustAttestation.sol";

/// @notice Deploy VaultfireTrustAttestation to any supported chain.
///
/// Usage:
///   # Base (verifier router: 0x0b144e07a0826182b6b59788c34b32bfa86fb711)
///   forge script script/Deploy.s.sol --rpc-url $BASE_RPC --broadcast --verify
///
///   # Avalanche (verifier router: 0x0b144E07A0826182B6b59788c34b32Bfa86Fb711)
///   forge script script/Deploy.s.sol --rpc-url $AVALANCHE_RPC --broadcast --verify
///
///   # Arbitrum (verifier router: 0x0b144e07a0826182b6b59788c34b32bfa86fb711)
///   forge script script/Deploy.s.sol --rpc-url $ARBITRUM_RPC --broadcast --verify
///
///   # Polygon PoS (verifier router: 0xdBAD523786971B75A7b1c1CFdCfECDeb59A764B9)
///   forge script script/Deploy.s.sol --rpc-url $POLYGON_RPC --broadcast --verify
///
/// IMPORTANT: Set DEPLOYER_KEY as an environment variable. NEVER hardcode private keys.
///
/// RISC Zero Verifier Router Addresses (canonical — already deployed):
///   Base:      0x0b144e07a0826182b6b59788c34b32bfa86fb711
///   Arbitrum:  0x0b144e07a0826182b6b59788c34b32bfa86fb711
///   Avalanche: 0x0b144E07A0826182B6b59788c34b32Bfa86Fb711
///   Polygon:   0xdBAD523786971B75A7b1c1CFdCfECDeb59A764B9
contract DeployVaultfireTrustAttestation is Script {
    // Canonical RISC Zero VerifierRouter addresses
    address constant VERIFIER_BASE = 0x0b144e07A0826182b6B59788c34b32bfA86fb711;
    address constant VERIFIER_ARBITRUM = 0x0b144e07A0826182b6B59788c34b32bfA86fb711;
    address constant VERIFIER_AVALANCHE = 0x0b144E07A0826182B6b59788c34b32Bfa86Fb711;
    address constant VERIFIER_POLYGON = 0xdBAD523786971B75A7b1c1CFdCfECDeb59A764B9;

    function getVerifierForChain() internal view returns (address) {
        uint256 chainId = block.chainid;

        if (chainId == 8453) return VERIFIER_BASE;          // Base
        if (chainId == 42161) return VERIFIER_ARBITRUM;      // Arbitrum
        if (chainId == 43114) return VERIFIER_AVALANCHE;     // Avalanche
        if (chainId == 137) return VERIFIER_POLYGON;         // Polygon PoS

        // Testnets
        if (chainId == 84532) return VERIFIER_BASE;          // Base Sepolia
        if (chainId == 421614) return VERIFIER_ARBITRUM;     // Arbitrum Sepolia
        if (chainId == 43113) return VERIFIER_AVALANCHE;     // Avalanche Fuji

        revert("Unsupported chain — add verifier address");
    }

    function run() external {
        uint256 deployerKey = vm.envUint("DEPLOYER_KEY");
        address verifier = getVerifierForChain();

        console.log("Chain ID:", block.chainid);
        console.log("Verifier:", verifier);

        vm.startBroadcast(deployerKey);

        VaultfireTrustAttestation attestation = new VaultfireTrustAttestation(verifier);
        console.log("VaultfireTrustAttestation deployed:", address(attestation));

        vm.stopBroadcast();
    }
}
