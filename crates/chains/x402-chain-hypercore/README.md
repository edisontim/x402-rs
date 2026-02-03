# x402-chain-hypercore

HyperCore (Hyperliquid native layer) chain support for the x402 payment protocol.

## Overview

This crate provides implementations of the x402 payment protocol for Hyperliquid's HyperCore,
the native on-chain order book and transfer layer (not the HyperEVM).

## Features

- **V2 Protocol Support**: Implements V2 protocol with CAIP-2 chain ID addressing
- **USDC Transfers**: Native `usdSend` action for USDC payments
- **Spot Token Transfers**: Native `spotSend` action for any spot token
- **EIP-712 Signing**: Uses familiar Ethereum-style wallet signatures

## Supported Networks

- `hypercore:mainnet` - Hyperliquid Mainnet
- `hypercore:testnet` - Hyperliquid Testnet

## Feature Flags

- `facilitator` - Facilitator-side payment verification and settlement
- `telemetry` - OpenTelemetry tracing support

## Usage

```rust
use x402_chain_hypercore::{V2HyperCoreExact, HyperCoreChainProvider};
use x402_types::scheme::X402SchemeFacilitatorBuilder;

let provider = HyperCoreChainProvider::from_config(&config).await?;
let facilitator = V2HyperCoreExact.build(provider, None)?;

// Verify payment
let verify_response = facilitator.verify(&verify_request).await?;

// Settle payment
let settle_response = facilitator.settle(&settle_request).await?;
```

## Configuration

```json
{
  "chains": {
    "hypercore:mainnet": {
      "api_url": "https://api.hyperliquid.xyz",
      "signer": "$HYPERCORE_PRIVATE_KEY"
    }
  }
}
```
