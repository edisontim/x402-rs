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
/// This matches the flat structure sent by HyperCore clients. It carries a
/// `sendAsset` action (not the legacy `spotSend`), which is required for
/// settlement to work under Hyperliquid Unified Account Mode — that mode
/// disables the standalone spot-send action.
///
/// ```json
/// {
///   "hyperliquidChain": "Testnet",
///   "signatureChainId": "0xa4b1",
///   "destination": "0x...",
///   "sourceDex": "spot",
///   "destinationDex": "spot",
///   "token": "USDC",
///   "amount": "1",
///   "fromSubAccount": "",
///   "nonce": 1234567890,
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
    /// Source dex for the transfer. "spot" for spot balances, "" for the default perp dex.
    #[serde(default = "default_spot_dex")]
    pub source_dex: String,
    /// Destination dex for the transfer. "spot" for spot balances, "" for the default perp dex.
    #[serde(default = "default_spot_dex")]
    pub destination_dex: String,
    /// Amount of USD to send as a string (e.g., "1" for $1).
    pub amount: String,
    /// Nonce / timestamp in milliseconds.
    pub nonce: u64,
    /// Token being transferred (e.g., "USDC").
    pub token: String,
    /// Sub-account to send from, or empty string for a normal account transfer.
    #[serde(default)]
    pub from_sub_account: String,
    /// The EIP-712 signature (hex-encoded, 65 bytes with v, r, s).
    pub signature: String,
}

fn default_signature_chain_id() -> String {
    "0xa4b1".to_string()
}

fn default_spot_dex() -> String {
    "spot".to_string()
}

impl HyperCorePayload {
    /// Convert to a sendAsset action structure for verification.
    pub fn to_send_asset_action(&self) -> HyperCoreSendAssetAction {
        HyperCoreSendAssetAction {
            action_type: "sendAsset".to_string(),
            hyperliquid_chain: self.hyperliquid_chain.clone(),
            signature_chain_id: self.signature_chain_id.clone(),
            destination: self.destination.clone(),
            source_dex: self.source_dex.clone(),
            destination_dex: self.destination_dex.clone(),
            token: self.token.clone(),
            amount: self.amount.clone(),
            from_sub_account: self.from_sub_account.clone(),
            nonce: self.nonce,
        }
    }
}

/// The sendAsset action structure for HyperCore transfers.
///
/// This is the internal representation used for EIP-712 signature recovery and
/// the exchange API call. `sendAsset` is unified-account-compatible, unlike the
/// legacy `spotSend`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HyperCoreSendAssetAction {
    /// Action type, always "sendAsset".
    #[serde(rename = "type")]
    pub action_type: String,
    /// Chain identifier ("Mainnet" or "Testnet").
    pub hyperliquid_chain: String,
    /// The chain ID used for signing (hex format, e.g., "0xa4b1" for Arbitrum).
    pub signature_chain_id: String,
    /// Destination address (42-character hex).
    pub destination: String,
    /// Source dex ("spot" for spot balances, "" for the default perp dex).
    pub source_dex: String,
    /// Destination dex ("spot" for spot balances, "" for the default perp dex).
    pub destination_dex: String,
    /// Token being transferred (e.g., "USDC").
    pub token: String,
    /// Amount of USD to send as a string (e.g., "1.5").
    pub amount: String,
    /// Sub-account to send from, or empty string for a normal account transfer.
    pub from_sub_account: String,
    /// Nonce / timestamp in milliseconds.
    pub nonce: u64,
}
