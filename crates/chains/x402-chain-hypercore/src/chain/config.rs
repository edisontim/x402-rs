//! HyperCore chain configuration.

use serde::{Deserialize, Serialize};
use std::ops::Deref;
use std::str::FromStr;
use url::Url;
use x402_types::chain::ChainId;
use x402_types::config::LiteralOrEnv;

use crate::chain::HyperCoreChainReference;

/// HyperCore chain configuration.
#[derive(Debug, Clone)]
pub struct HyperCoreChainConfig {
    /// The chain reference (mainnet or testnet).
    pub chain_reference: HyperCoreChainReference,
    /// The inner configuration fields.
    pub inner: HyperCoreChainConfigInner,
}

impl HyperCoreChainConfig {
    /// Returns the signer configuration if present.
    pub fn signer(&self) -> Option<&HyperCoreSignerConfig> {
        self.inner.signer.as_ref()
    }

    /// Returns the API URL.
    pub fn api_url(&self) -> &Url {
        self.inner.api_url.inner()
    }

    /// Returns the chain reference.
    pub fn chain_reference(&self) -> HyperCoreChainReference {
        self.chain_reference
    }

    /// Returns the CAIP-2 chain ID.
    pub fn chain_id(&self) -> ChainId {
        self.chain_reference.into()
    }
}

/// Configuration fields for HyperCore chains.
///
/// # Example - Using environment variables (recommended for deployments)
///
/// ```json
/// {
///   "hypercore:mainnet": {
///     "api_url": "$HYPERCORE_API_URL",
///     "signer": "$HYPERCORE_PRIVATE_KEY"
///   }
/// }
/// ```
///
/// Set these environment variables:
/// - `HYPERCORE_API_URL="https://api.hyperliquid.xyz"` (or testnet URL)
/// - `HYPERCORE_PRIVATE_KEY="0x..."`
///
/// # Example - Literal values in config
///
/// ```json
/// {
///   "hypercore:mainnet": {
///     "api_url": "https://api.hyperliquid.xyz",
///     "signer": "$HYPERCORE_PRIVATE_KEY"
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperCoreChainConfigInner {
    /// API URL for HyperCore (required).
    /// Mainnet: https://api.hyperliquid.xyz
    /// Testnet: https://api.hyperliquid-testnet.xyz
    /// Supports literal URLs or environment variable references like "$HYPERCORE_API_URL".
    #[serde(default = "hypercore_config_defaults::default_api_url")]
    pub api_url: LiteralOrEnv<Url>,

    /// Signer configuration for this chain (optional, required for settling payments).
    /// A hex-encoded private key or env var reference like "$HYPERCORE_PRIVATE_KEY".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signer: Option<HyperCoreSignerConfig>,
}

mod hypercore_config_defaults {
    use super::*;

    pub fn default_api_url() -> LiteralOrEnv<Url> {
        LiteralOrEnv::from_literal(
            Url::parse("https://api.hyperliquid.xyz").expect("valid default URL"),
        )
    }
}

/// A validated HyperCore private key (32 bytes).
///
/// HyperCore uses Ethereum-style secp256k1 private keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperCorePrivateKey(Vec<u8>);

impl HyperCorePrivateKey {
    /// Parse a hex string into a private key.
    pub fn from_hex(s: &str) -> Result<Self, String> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))?;

        if bytes.len() != 32 {
            return Err(format!(
                "Private key must be 32 bytes, got {} bytes",
                bytes.len()
            ));
        }

        Ok(Self(bytes))
    }

    /// Encode the private key as hex.
    pub fn to_hex(&self) -> String {
        format!("0x{}", hex::encode(&self.0))
    }

    /// Returns the raw bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl Serialize for HyperCorePrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl FromStr for HyperCorePrivateKey {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl std::fmt::Display for HyperCorePrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Type alias for HyperCore signer configuration.
///
/// Uses `LiteralOrEnv` to support both literal hex keys and environment variable references.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HyperCoreSignerConfig(LiteralOrEnv<HyperCorePrivateKey>);

impl Deref for HyperCoreSignerConfig {
    type Target = HyperCorePrivateKey;

    fn deref(&self) -> &Self::Target {
        self.0.inner()
    }
}
