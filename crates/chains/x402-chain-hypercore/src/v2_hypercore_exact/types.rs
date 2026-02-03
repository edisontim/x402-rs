//! V2 HyperCore "exact" payment scheme types.

use serde::{Deserialize, Serialize};
use x402_types::lit_str;
use x402_types::proto::v2;

lit_str!(ExactScheme, "exact");

/// The V2 HyperCore exact scheme verify request.
pub type VerifyRequest = v2::VerifyRequest<PaymentPayload, PaymentRequirements>;

/// The V2 HyperCore exact scheme settle request.
pub type SettleRequest = VerifyRequest;

/// The payment payload for HyperCore exact scheme.
pub type PaymentPayload = v2::PaymentPayload<PaymentRequirements, HyperCorePayload>;

/// The payment requirements for HyperCore exact scheme.
///
/// For HyperCore USDC payments:
/// - `asset` is the token name (e.g., "USDC"), not a contract address
/// - `pay_to` is the recipient's Ethereum-style address
///
/// The extra field contains USDC metadata like `{ "name": "USDC", "version": "1" }`.
pub type PaymentRequirements = v2::PaymentRequirements<ExactScheme, String, String, UsdcExtra>;

/// Extra metadata for USDC payments (used for EIP-712 domain).
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct UsdcExtra {
    /// Token name, e.g., "USDC".
    pub name: String,
    /// Token version, e.g., "1".
    pub version: String,
}

/// The HyperCore-specific payload containing the signed transfer action.
///
/// This matches the flat structure sent by HyperCore clients:
/// ```json
/// {
///   "hyperliquidChain": "Testnet",
///   "signatureChainId": "0xa4b1",
///   "destination": "0x...",
///   "amount": "1",
///   "time": 1234567890,
///   "token": "USDC",
///   "signature": "0x..."
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperCorePayload {
    /// Chain identifier ("Mainnet" or "Testnet").
    pub hyperliquid_chain: String,
    /// The chain ID used for EIP-712 signing (hex format, e.g., "0xa4b1" for Arbitrum).
    /// Defaults to "0xa4b1" if not provided.
    #[serde(default = "default_signature_chain_id")]
    pub signature_chain_id: String,
    /// Destination address (42-character hex).
    pub destination: String,
    /// Amount of USD to send as a string (e.g., "1" for $1).
    pub amount: String,
    /// Timestamp in milliseconds.
    pub time: u64,
    /// Token being transferred (e.g., "USDC").
    pub token: String,
    /// The EIP-712 signature (hex-encoded, 65 bytes with v, r, s).
    pub signature: String,
}

fn default_signature_chain_id() -> String {
    "0xa4b1".to_string()
}

impl HyperCorePayload {
    /// Convert to a usdSend action structure for verification.
    pub fn to_usd_send_action(&self) -> HyperCoreUsdSendAction {
        HyperCoreUsdSendAction {
            action_type: "usdSend".to_string(),
            hyperliquid_chain: self.hyperliquid_chain.clone(),
            signature_chain_id: self.signature_chain_id.clone(),
            destination: self.destination.clone(),
            amount: self.amount.clone(),
            time: self.time,
        }
    }
}

/// The usdSend action structure for HyperCore transfers.
///
/// This is the internal representation used for EIP-712 signing and API calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperCoreUsdSendAction {
    /// Action type, always "usdSend".
    #[serde(rename = "type")]
    pub action_type: String,
    /// Chain identifier ("Mainnet" or "Testnet").
    pub hyperliquid_chain: String,
    /// The chain ID used for signing (hex format, e.g., "0xa4b1" for Arbitrum).
    pub signature_chain_id: String,
    /// Destination address (42-character hex).
    pub destination: String,
    /// Amount of USD to send as a string (e.g., "1.5").
    pub amount: String,
    /// Timestamp in milliseconds.
    pub time: u64,
}

impl HyperCoreUsdSendAction {
    /// Creates a new usdSend action.
    pub fn new(chain: &str, destination: &str, amount: &str, time: u64) -> Self {
        Self {
            action_type: "usdSend".to_string(),
            hyperliquid_chain: chain.to_string(),
            // Default to Arbitrum chain ID for signing
            signature_chain_id: "0xa4b1".to_string(),
            destination: destination.to_string(),
            amount: amount.to_string(),
            time,
        }
    }
}
