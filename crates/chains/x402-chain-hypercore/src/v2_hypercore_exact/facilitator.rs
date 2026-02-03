//! V2 HyperCore exact scheme facilitator implementation.

use std::collections::HashMap;
use std::sync::Arc;
use x402_types::chain::ChainProviderOps;
use x402_types::proto;
use x402_types::proto::{PaymentVerificationError, v2};
use x402_types::scheme::{
    X402SchemeFacilitator, X402SchemeFacilitatorBuilder, X402SchemeFacilitatorError,
};

use crate::USDC;
use crate::V2HyperCoreExact;
use crate::chain::{HyperCoreChainProvider, HyperCoreChainReference};
use crate::v2_hypercore_exact::types;
use crate::v2_hypercore_exact::types::ExactScheme;

/// The V2 HyperCore exact scheme facilitator.
pub struct V2HyperCoreExactFacilitator {
    provider: Arc<HyperCoreChainProvider>,
}

impl X402SchemeFacilitatorBuilder<Arc<HyperCoreChainProvider>> for V2HyperCoreExact {
    fn build(
        &self,
        provider: Arc<HyperCoreChainProvider>,
        _config: Option<serde_json::Value>,
    ) -> Result<Box<dyn X402SchemeFacilitator>, Box<dyn std::error::Error>> {
        Ok(Box::new(V2HyperCoreExactFacilitator { provider }))
    }
}

#[async_trait::async_trait]
impl X402SchemeFacilitator for V2HyperCoreExactFacilitator {
    async fn verify(
        &self,
        request: &proto::VerifyRequest,
    ) -> Result<proto::VerifyResponse, X402SchemeFacilitatorError> {
        let request = types::VerifyRequest::from_proto(request.clone())?;
        let verification = verify_transfer(&self.provider, &request).await?;
        Ok(v2::VerifyResponse::valid(verification.payer).into())
    }

    async fn settle(
        &self,
        request: &proto::SettleRequest,
    ) -> Result<proto::SettleResponse, X402SchemeFacilitatorError> {
        let request = types::SettleRequest::from_proto(request.clone())?;
        let verification = verify_transfer(&self.provider, &request).await?;
        let payer = verification.payer.clone();
        let tx_id = settle_transaction(&self.provider, verification).await?;
        Ok(v2::SettleResponse::Success {
            payer,
            transaction: tx_id,
            network: self.provider.chain_id().to_string(),
        }
        .into())
    }

    async fn supported(&self) -> Result<proto::SupportedResponse, X402SchemeFacilitatorError> {
        let chain_id = self.provider.chain_id();

        let kinds: Vec<proto::SupportedPaymentKind> = vec![proto::SupportedPaymentKind {
            x402_version: proto::v2::X402Version2.into(),
            scheme: ExactScheme.to_string(),
            network: chain_id.to_string(),
            extra: None,
        }];

        let signers = {
            let mut signers = HashMap::with_capacity(1);
            signers.insert(chain_id, self.provider.signer_addresses());
            signers
        };

        Ok(proto::SupportedResponse {
            kinds,
            extensions: Vec::new(),
            signers,
        })
    }
}

/// Result of verifying a HyperCore transfer.
pub struct VerifyTransferResult {
    /// The payer's address.
    pub payer: String,
    /// The verified action.
    pub action: types::HyperCoreUsdSendAction,
    /// The signature.
    pub signature: String,
    /// The nonce.
    pub nonce: u64,
}

/// Verify a HyperCore transfer request.
pub async fn verify_transfer(
    provider: &HyperCoreChainProvider,
    request: &types::VerifyRequest,
) -> Result<VerifyTransferResult, PaymentVerificationError> {
    let payload = &request.payment_payload;
    let requirements = &request.payment_requirements;

    // Validate accepted == requirements
    let accepted = &payload.accepted;
    if accepted != requirements {
        return Err(PaymentVerificationError::AcceptedRequirementsMismatch);
    }

    // Validate chain ID
    let chain_id = provider.chain_id();
    let payload_chain_id = &accepted.network;
    if payload_chain_id != &chain_id {
        return Err(PaymentVerificationError::UnsupportedChain);
    }

    // Extract the action and signature
    let hypercore_payload = &payload.payload;
    let action = &hypercore_payload.action;
    let signature = &hypercore_payload.signature;
    let nonce = hypercore_payload.nonce;

    // Validate action type
    if action.action_type != "usdSend" {
        return Err(PaymentVerificationError::InvalidFormat(format!(
            "Expected usdSend action, got {}",
            action.action_type
        )));
    }

    // Validate chain matches
    let expected_chain = provider.chain_reference().as_api_str();
    if action.hyperliquid_chain != expected_chain {
        return Err(PaymentVerificationError::InvalidFormat(format!(
            "Chain mismatch: expected {}, got {}",
            expected_chain, action.hyperliquid_chain
        )));
    }

    // Validate destination matches pay_to
    let expected_recipient = requirements.pay_to.to_string().to_lowercase();
    let action_destination = action.destination.to_lowercase();
    if action_destination != expected_recipient {
        return Err(PaymentVerificationError::RecipientMismatch);
    }

    // Validate amount
    let expected_amount = USDC::parse_amount(&requirements.amount).map_err(|e| {
        PaymentVerificationError::InvalidFormat(format!("Failed to parse expected amount: {}", e))
    })?;
    let action_amount = USDC::parse_amount(&action.amount).map_err(|e| {
        PaymentVerificationError::InvalidFormat(format!("Failed to parse action amount: {}", e))
    })?;
    if action_amount != expected_amount {
        return Err(PaymentVerificationError::InvalidPaymentAmount);
    }

    // Validate the timestamp is reasonable (within 5 minutes of nonce)
    let time_diff = if action.time > nonce {
        action.time - nonce
    } else {
        nonce - action.time
    };
    if time_diff > 300_000 {
        // 5 minutes
        return Err(PaymentVerificationError::InvalidFormat(
            "Timestamp and nonce mismatch exceeds 5 minutes".to_string(),
        ));
    }

    // Recover the signer address from the EIP-712 signature
    let payer = recover_signer_from_usd_send(action, signature)?;

    // Optionally verify the payer has sufficient balance
    // This is a soft check - the settlement will fail if insufficient
    #[cfg(feature = "telemetry")]
    {
        match provider.get_usdc_balance(&payer).await {
            Ok(balance) => {
                let balance_amount = USDC::parse_amount(&balance).unwrap_or(0);
                if balance_amount < expected_amount {
                    tracing::warn!(
                        payer = %payer,
                        balance = %balance,
                        required = %requirements.amount,
                        "Payer may have insufficient balance"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(payer = %payer, error = %e, "Failed to check payer balance");
            }
        }
    }

    Ok(VerifyTransferResult {
        payer,
        action: action.clone(),
        signature: signature.clone(),
        nonce,
    })
}

/// Settle a verified transfer by submitting it to HyperCore.
///
/// This function forwards the user's pre-signed `usdSend` action to the HyperCore
/// exchange API. The user has already signed the action, and the facilitator
/// submits it on their behalf.
pub async fn settle_transaction(
    provider: &HyperCoreChainProvider,
    verification: VerifyTransferResult,
) -> Result<String, PaymentVerificationError> {
    // Build the API URL based on chain
    let api_url = match provider.chain_reference() {
        HyperCoreChainReference::Mainnet => "https://api.hyperliquid.xyz/exchange",
        HyperCoreChainReference::Testnet => "https://api.hyperliquid-testnet.xyz/exchange",
    };

    // Parse the signature into r, s, v components
    let sig_bytes = hex::decode(
        verification
            .signature
            .strip_prefix("0x")
            .unwrap_or(&verification.signature),
    )
    .map_err(|e| PaymentVerificationError::InvalidFormat(format!("Invalid signature: {}", e)))?;

    if sig_bytes.len() != 65 {
        return Err(PaymentVerificationError::InvalidFormat(format!(
            "Signature must be 65 bytes, got {}",
            sig_bytes.len()
        )));
    }

    let r = format!("0x{}", hex::encode(&sig_bytes[0..32]));
    let s = format!("0x{}", hex::encode(&sig_bytes[32..64]));
    let v = sig_bytes[64] as u64;

    // Build the request body
    let request_body = serde_json::json!({
        "action": {
            "type": "usdSend",
            "hyperliquidChain": verification.action.hyperliquid_chain,
            "signatureChainId": verification.action.signature_chain_id,
            "destination": verification.action.destination,
            "amount": verification.action.amount,
            "time": verification.action.time
        },
        "nonce": verification.nonce,
        "signature": {
            "r": r,
            "s": s,
            "v": v
        }
    });

    // Submit to HyperCore
    let client = reqwest::Client::new();
    let response = client
        .post(api_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            PaymentVerificationError::TransactionSimulation(format!(
                "Failed to submit to HyperCore: {}",
                e
            ))
        })?;

    let status = response.status();
    let body: serde_json::Value = response.json().await.map_err(|e| {
        PaymentVerificationError::TransactionSimulation(format!("Failed to parse response: {}", e))
    })?;

    // Check for success
    if !status.is_success() {
        return Err(PaymentVerificationError::TransactionSimulation(format!(
            "HyperCore API error: {} - {:?}",
            status, body
        )));
    }

    // Check response status
    let response_status = body.get("status").and_then(|s| s.as_str());
    if response_status != Some("ok") {
        let error = body
            .get("response")
            .and_then(|r| r.get("data"))
            .and_then(|d| d.get("statuses"))
            .and_then(|s| s.get(0))
            .and_then(|s| s.get("error"))
            .and_then(|e| e.as_str())
            .unwrap_or("Unknown error");
        return Err(PaymentVerificationError::TransactionSimulation(format!(
            "HyperCore rejected transfer: {}",
            error
        )));
    }

    #[cfg(feature = "telemetry")]
    tracing::info!(
        payer = %verification.payer,
        destination = %verification.action.destination,
        amount = %verification.action.amount,
        "HyperCore transfer settled"
    );

    // Return a transaction identifier
    // HyperCore doesn't return traditional tx hashes, so we create a unique ID
    let payer_prefix = if verification.payer.len() >= 10 {
        &verification.payer[..10]
    } else {
        &verification.payer
    };

    Ok(format!(
        "hypercore:{}:{}:{}",
        verification.nonce, verification.action.destination, payer_prefix
    ))
}

/// Recover the signer address from a usdSend EIP-712 signature.
fn recover_signer_from_usd_send(
    action: &types::HyperCoreUsdSendAction,
    signature: &str,
) -> Result<String, PaymentVerificationError> {
    use ethers::types::{H256, RecoveryMessage, Signature};

    // Parse the signature
    let sig_bytes =
        hex::decode(signature.strip_prefix("0x").unwrap_or(signature)).map_err(|e| {
            PaymentVerificationError::InvalidSignature(format!("Invalid signature hex: {}", e))
        })?;

    if sig_bytes.len() != 65 {
        return Err(PaymentVerificationError::InvalidSignature(format!(
            "Signature must be 65 bytes, got {}",
            sig_bytes.len()
        )));
    }

    // Construct the EIP-712 typed data hash for usdSend
    // This follows the Hyperliquid signing spec
    let domain_separator = compute_domain_separator(&action.signature_chain_id)?;
    let struct_hash = compute_usd_send_struct_hash(action)?;
    let message_hash = compute_typed_data_hash(&domain_separator, &struct_hash);

    // Recover the address
    let signature = Signature::try_from(sig_bytes.as_slice()).map_err(|e| {
        PaymentVerificationError::InvalidSignature(format!("Invalid signature format: {}", e))
    })?;

    let recovery_message = RecoveryMessage::Hash(H256::from_slice(&message_hash));
    let recovered_address = signature.recover(recovery_message).map_err(|e| {
        PaymentVerificationError::InvalidSignature(format!("Failed to recover signer: {}", e))
    })?;

    Ok(format!("0x{}", hex::encode(recovered_address.as_bytes())))
}

/// Compute the EIP-712 domain separator for HyperliquidSignTransaction.
fn compute_domain_separator(chain_id_hex: &str) -> Result<[u8; 32], PaymentVerificationError> {
    use ethers::utils::keccak256;

    // Domain type hash: keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)")
    let domain_type_hash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );

    // Name: "HyperliquidSignTransaction"
    let name_hash = keccak256(b"HyperliquidSignTransaction");

    // Version: "1"
    let version_hash = keccak256(b"1");

    // Parse chain ID
    let chain_id_str = chain_id_hex.strip_prefix("0x").unwrap_or(chain_id_hex);
    let chain_id = u64::from_str_radix(chain_id_str, 16)
        .map_err(|e| PaymentVerificationError::InvalidFormat(format!("Invalid chain ID: {}", e)))?;

    // Verifying contract: 0x0000000000000000000000000000000000000000
    let verifying_contract = [0u8; 32];

    // Encode and hash
    let mut encoded = Vec::with_capacity(160);
    encoded.extend_from_slice(&domain_type_hash);
    encoded.extend_from_slice(&name_hash);
    encoded.extend_from_slice(&version_hash);
    encoded.extend_from_slice(&encode_u256(chain_id));
    encoded.extend_from_slice(&verifying_contract);

    Ok(keccak256(&encoded))
}

/// Compute the struct hash for a usdSend action.
fn compute_usd_send_struct_hash(
    action: &types::HyperCoreUsdSendAction,
) -> Result<[u8; 32], PaymentVerificationError> {
    use ethers::utils::keccak256;

    // Type hash: keccak256("HyperliquidTransaction:UsdSend(string hyperliquidChain,string destination,string amount,uint64 time)")
    let type_hash = keccak256(
        b"HyperliquidTransaction:UsdSend(string hyperliquidChain,string destination,string amount,uint64 time)",
    );

    let chain_hash = keccak256(action.hyperliquid_chain.as_bytes());
    let destination_hash = keccak256(action.destination.as_bytes());
    let amount_hash = keccak256(action.amount.as_bytes());

    // Encode and hash
    let mut encoded = Vec::with_capacity(160);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&chain_hash);
    encoded.extend_from_slice(&destination_hash);
    encoded.extend_from_slice(&amount_hash);
    encoded.extend_from_slice(&encode_u64(action.time));

    Ok(keccak256(&encoded))
}

/// Compute the final EIP-712 typed data hash.
fn compute_typed_data_hash(domain_separator: &[u8; 32], struct_hash: &[u8; 32]) -> [u8; 32] {
    use ethers::utils::keccak256;

    let mut message = Vec::with_capacity(66);
    message.extend_from_slice(b"\x19\x01");
    message.extend_from_slice(domain_separator);
    message.extend_from_slice(struct_hash);

    keccak256(&message)
}

/// Encode a u256 value (left-padded to 32 bytes).
fn encode_u256(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    bytes
}

/// Encode a u64 value (left-padded to 32 bytes).
fn encode_u64(value: u64) -> [u8; 32] {
    let mut bytes = [0u8; 32];
    bytes[24..].copy_from_slice(&value.to_be_bytes());
    bytes
}
