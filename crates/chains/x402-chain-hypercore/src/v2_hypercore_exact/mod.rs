//! V2 HyperCore "exact" payment scheme implementation.
//!
//! This module implements the "exact" payment scheme for HyperCore using
//! the V2 x402 protocol. It uses CAIP-2 chain identifiers (hypercore:mainnet, hypercore:testnet).
//!
//! # Features
//!
//! - USDC transfers using HyperCore's native `usdSend` action
//! - EIP-712 typed data signing for secure authorization
//! - Balance verification before settlement
//!
//! # Usage
//!
//! ```ignore
//! use x402_chain_hypercore::v2_hypercore_exact::V2HyperCoreExact;
//!
//! // Create the scheme
//! let scheme = V2HyperCoreExact;
//! ```

#[cfg(feature = "facilitator")]
pub mod facilitator;
#[cfg(feature = "facilitator")]
pub use facilitator::*;

pub mod types;
pub use types::*;

use x402_types::scheme::X402SchemeId;

/// V2 HyperCore Exact payment scheme.
///
/// This scheme supports exact USDC payments on HyperCore using the native
/// `usdSend` action with EIP-712 signatures.
pub struct V2HyperCoreExact;

impl X402SchemeId for V2HyperCoreExact {
    fn namespace(&self) -> &str {
        "hypercore"
    }

    fn scheme(&self) -> &str {
        ExactScheme.as_ref()
    }
}
