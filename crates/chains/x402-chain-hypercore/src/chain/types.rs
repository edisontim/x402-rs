//! HyperCore chain types.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Debug, Display, Formatter};
use std::str::FromStr;
use x402_types::chain::ChainId;

/// The CAIP-2 namespace for HyperCore chains.
pub const HYPERCORE_NAMESPACE: &str = "hypercore";

/// A HyperCore chain reference - mainnet or testnet.
///
/// HyperCore uses string identifiers:
/// - `mainnet` for production
/// - `testnet` for testing
///
/// # Example
///
/// ```
/// use x402_chain_hypercore::chain::HyperCoreChainReference;
/// use x402_types::chain::ChainId;
///
/// let mainnet = HyperCoreChainReference::mainnet();
/// let chain_id: ChainId = mainnet.into();
/// assert_eq!(chain_id.to_string(), "hypercore:mainnet");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum HyperCoreChainReference {
    /// Hyperliquid Mainnet
    Mainnet,
    /// Hyperliquid Testnet
    Testnet,
}

impl HyperCoreChainReference {
    /// Returns the mainnet chain reference.
    pub fn mainnet() -> Self {
        Self::Mainnet
    }

    /// Returns the testnet chain reference.
    pub fn testnet() -> Self {
        Self::Testnet
    }

    /// Returns the string representation used in API calls.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Mainnet => "Mainnet",
            Self::Testnet => "Testnet",
        }
    }

    /// Returns the chain reference string for CAIP-2.
    pub fn as_caip2_ref(&self) -> &'static str {
        match self {
            Self::Mainnet => "mainnet",
            Self::Testnet => "testnet",
        }
    }
}

impl Debug for HyperCoreChainReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "HyperCoreChainReference({})", self.as_caip2_ref())
    }
}

impl Display for HyperCoreChainReference {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_caip2_ref())
    }
}

impl FromStr for HyperCoreChainReference {
    type Err = HyperCoreChainReferenceFormatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mainnet" => Ok(Self::Mainnet),
            "testnet" => Ok(Self::Testnet),
            _ => Err(HyperCoreChainReferenceFormatError::InvalidReference(
                s.to_string(),
            )),
        }
    }
}

impl Serialize for HyperCoreChainReference {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_caip2_ref())
    }
}

impl<'de> Deserialize<'de> for HyperCoreChainReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl From<HyperCoreChainReference> for ChainId {
    fn from(value: HyperCoreChainReference) -> Self {
        ChainId::new(HYPERCORE_NAMESPACE, value.as_caip2_ref().to_string())
    }
}

impl TryFrom<ChainId> for HyperCoreChainReference {
    type Error = HyperCoreChainReferenceFormatError;

    fn try_from(value: ChainId) -> Result<Self, Self::Error> {
        if value.namespace != HYPERCORE_NAMESPACE {
            return Err(HyperCoreChainReferenceFormatError::InvalidNamespace(
                value.namespace,
            ));
        }
        Self::from_str(&value.reference)
    }
}

/// Error type for parsing HyperCore chain references.
#[derive(Debug, thiserror::Error)]
pub enum HyperCoreChainReferenceFormatError {
    /// The namespace was not "hypercore".
    #[error("Invalid namespace {0}, expected hypercore")]
    InvalidNamespace(String),
    /// The reference was not a valid HyperCore network (mainnet or testnet).
    #[error("Invalid hypercore chain reference {0}, expected mainnet or testnet")]
    InvalidReference(String),
}

/// A HyperCore address (Ethereum-style 20-byte address).
///
/// HyperCore uses Ethereum addresses for account identification.
///
/// # Example
///
/// ```
/// use x402_chain_hypercore::chain::Address;
/// use std::str::FromStr;
///
/// let addr = Address::from_str("0x742d35Cc6634C0532925a3b844Bc9e7595f1b0aA").unwrap();
/// assert!(addr.to_string().starts_with("0x"));
/// ```
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Address([u8; 20]);

impl Address {
    /// Creates a new address from bytes.
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Returns the address bytes.
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Returns the address as a checksummed hex string.
    pub fn to_checksum_string(&self) -> String {
        format!("0x{}", hex::encode(self.0))
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl FromStr for Address {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| format!("Invalid hex: {}", e))?;
        if bytes.len() != 20 {
            return Err(format!(
                "Address must be 20 bytes, got {} bytes",
                bytes.len()
            ));
        }
        let mut arr = [0u8; 20];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

impl Serialize for Address {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Address {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

/// Token identifier for HyperCore spot tokens.
///
/// HyperCore spot tokens use the format `NAME:tokenId` (e.g., `PURR:0xc4bf3f870c0e9465323c0b6ed28096c2`).
/// USDC is handled separately via `usdSend`.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpotToken {
    /// Token name (e.g., "PURR")
    pub name: String,
    /// Token ID (hex string)
    pub token_id: String,
}

impl SpotToken {
    /// Creates a new spot token identifier.
    pub fn new(name: impl Into<String>, token_id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            token_id: token_id.into(),
        }
    }

    /// Returns the token string in HyperCore format (NAME:tokenId).
    pub fn to_hypercore_string(&self) -> String {
        format!("{}:{}", self.name, self.token_id)
    }
}

impl Display for SpotToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.token_id)
    }
}

impl FromStr for SpotToken {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(format!(
                "Invalid spot token format: {}, expected NAME:tokenId",
                s
            ));
        }
        Ok(Self {
            name: parts[0].to_string(),
            token_id: parts[1].to_string(),
        })
    }
}
