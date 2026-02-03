#![cfg_attr(docsrs, feature(doc_auto_cfg))]

//! HyperCore chain support for the x402 payment protocol.
//!
//! This crate provides implementations of the x402 payment protocol for Hyperliquid's
//! HyperCore native layer. It supports the V2 protocol with USDC and spot token transfers.
//!
//! # Features
//!
//! - **V2 Protocol Support**: Implements V2 protocol with CAIP-2 chain ID addressing
//! - **USDC Transfers**: Native `usdSend` action for USDC payments
//! - **Spot Token Transfers**: Native `spotSend` action for any spot token
//! - **EIP-712 Signing**: Uses familiar Ethereum-style wallet signatures
//!
//! # Architecture
//!
//! The crate is organized into several modules:
//!
//! - [`chain`] - Core HyperCore chain types, providers, and configuration
//! - [`v2_hypercore_exact`] - V2 protocol implementation with CAIP-2 chain IDs
//!
//! # Feature Flags
//!
//! - `facilitator` - Facilitator-side payment verification and settlement
//! - `telemetry` - OpenTelemetry tracing support
//!
//! # Usage Examples
//!
//! ## Facilitator: Verifying and Settling
//!
//! ```ignore
//! use x402_chain_hypercore::{V2HyperCoreExact, HyperCoreChainProvider};
//! use x402_types::scheme::X402SchemeFacilitatorBuilder;
//!
//! let provider = HyperCoreChainProvider::from_config(&config).await?;
//! let facilitator = V2HyperCoreExact.build(provider, None)?;
//!
//! // Verify payment
//! let verify_response = facilitator.verify(&verify_request).await?;
//!
//! // Settle payment
//! let settle_response = facilitator.settle(&settle_request).await?;
//! ```

pub mod chain;
pub mod v2_hypercore_exact;

mod networks;
pub use networks::*;

pub use v2_hypercore_exact::V2HyperCoreExact;
