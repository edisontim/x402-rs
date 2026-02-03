//! V2 HyperCore "exact" payment scheme types.

use serde::{Deserialize, Serialize};
use x402_types::lit_str;
use x402_types::proto::v2;

use crate::chain::Address;

lit_str!(ExactScheme, "exact");

/// The V2 HyperCore exact scheme verify request.
pub type VerifyRequest = v2::VerifyRequest<PaymentPayload, PaymentRequirements>;

/// The V2 HyperCore exact scheme settle request.
pub type SettleRequest = VerifyRequest;

/// The payment payload for HyperCore exact scheme.
pub type PaymentPayload = v2::PaymentPayload<PaymentRequirements, HyperCorePayload>;

/// The payment requirements for HyperCore exact scheme.
///
/// For HyperCore USDC payments, the asset field should be "USDC".
pub type PaymentRequirements = v2::PaymentRequirements<ExactScheme, String, Address, ()>;

/// The HyperCore-specific payload containing the signed transfer action.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperCorePayload {
    /// The signed `usdSend` action as JSON.
    /// Contains: type, hyperliquidChain, signatureChainId, destination, amount, time
    pub action: HyperCoreUsdSendAction,
    /// The EIP-712 signature (hex-encoded, 65 bytes with v, r, s).
    pub signature: String,
    /// The nonce (timestamp in milliseconds).
    pub nonce: u64,
}

/// The usdSend action structure for HyperCore transfers.
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
