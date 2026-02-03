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
    /// The verified action (constructed from the payload).
    pub action: types::HyperCoreUsdSendAction,
    /// The signature.
    pub signature: String,
    /// The nonce (timestamp used as nonce).
    pub nonce: u64,
    /// The token being transferred (e.g., "USDC:0" for spot USDC).
    pub token: String,
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

    // Extract the flat payload fields
    let hypercore_payload = &payload.payload;
    let signature = &hypercore_payload.signature;

    // Validate token is USDC
    if hypercore_payload.token != "USDC" {
        return Err(PaymentVerificationError::InvalidFormat(format!(
            "Expected USDC token, got {}",
            hypercore_payload.token
        )));
    }

    // Validate chain matches
    let expected_chain = provider.chain_reference().as_api_str();
    if hypercore_payload.hyperliquid_chain != expected_chain {
        return Err(PaymentVerificationError::InvalidFormat(format!(
            "Chain mismatch: expected {}, got {}",
            expected_chain, hypercore_payload.hyperliquid_chain
        )));
    }

    // Validate destination matches pay_to
    let expected_recipient = requirements.pay_to.to_lowercase();
    let payload_destination = hypercore_payload.destination.to_lowercase();
    if payload_destination != expected_recipient {
        return Err(PaymentVerificationError::RecipientMismatch);
    }

    // Validate amount
    let expected_amount: u64 = requirements.amount.parse().map_err(|e| {
        PaymentVerificationError::InvalidFormat(format!("Failed to parse expected amount: {}", e))
    })?;
    let payload_amount = USDC::parse_amount(&hypercore_payload.amount).map_err(|e| {
        PaymentVerificationError::InvalidFormat(format!("Failed to parse payload amount: {}", e))
    })?;
    if payload_amount != expected_amount {
        return Err(PaymentVerificationError::InvalidPaymentAmount);
    }

    // Convert flat payload to action structure for signature verification
    let action = hypercore_payload.to_usd_send_action();

    // Use the payload timestamp as the nonce
    let nonce = hypercore_payload.time;

    // Recover the signer address from the EIP-712 signature (spotSend)
    let payer = recover_signer_from_spot_send(&action, &hypercore_payload.token, signature)?;

    match provider.get_usdc_balance(&payer).await {
        Ok(balance) => {
            let balance_amount = USDC::parse_amount(&balance).unwrap_or(0);
            if balance_amount < expected_amount {
                #[cfg(feature = "telemetry")]
                tracing::warn!(
                    payer = %payer,
                    balance_raw = %balance,
                    balance_atomic = %balance_amount,
                    required_atomic = %expected_amount,
                    "Payer has insufficient balance"
                );
                return Err(PaymentVerificationError::InsufficientFunds);
            }
        }
        Err(e) => {
            #[cfg(feature = "telemetry")]
            tracing::warn!(payer = %payer, error = %e, "Failed to check payer balance");
            // Continue - let the settlement fail if balance is actually insufficient
        }
    }

    Ok(VerifyTransferResult {
        payer,
        action,
        signature: signature.clone(),
        nonce,
        token: hypercore_payload.token.clone(),
    })
}

/// Settle a verified transfer by submitting it to HyperCore.
///
/// This function forwards the user's pre-signed `spotSend` action to the HyperCore
/// exchange API using the SDK. The user has already signed the action, and the
/// facilitator submits it on their behalf using a dummy signer.
pub async fn settle_transaction(
    provider: &HyperCoreChainProvider,
    verification: VerifyTransferResult,
) -> Result<String, PaymentVerificationError> {
    use alloy::primitives::Signature;
    use alloy::signers::local::PrivateKeySigner;
    use hyperliquid_rust_sdk::{BaseUrl, ExchangeClient};

    // Dummy private key - we don't sign anything, just forward the user's signature
    const DUMMY_PRIVATE_KEY: &str =
        "1111111111111111111111111111111111111111111111111111111111111111";

    let signer: PrivateKeySigner = DUMMY_PRIVATE_KEY
        .parse()
        .map_err(|e| PaymentVerificationError::InvalidFormat(format!("Invalid signer: {}", e)))?;

    let base_url = match provider.chain_reference() {
        HyperCoreChainReference::Mainnet => BaseUrl::Mainnet,
        HyperCoreChainReference::Testnet => BaseUrl::Testnet,
    };

    let exchange_client = ExchangeClient::new(None, signer, Some(base_url), None, None)
        .await
        .map_err(|e| {
            PaymentVerificationError::TransactionSimulation(format!(
                "Failed to create exchange client: {:?}",
                e
            ))
        })?;

    // Parse the user's signature into alloy's Signature type
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

    let sig_array: [u8; 65] = sig_bytes
        .try_into()
        .map_err(|_| PaymentVerificationError::InvalidFormat("Signature not 65 bytes".into()))?;
    let signature = Signature::try_from(&sig_array[..]).map_err(|e| {
        PaymentVerificationError::InvalidFormat(format!("Invalid signature format: {}", e))
    })?;

    // Build the spotSend action for USDC transfer from spot balance
    let action = serde_json::json!({
        "type": "spotSend",
        "hyperliquidChain": verification.action.hyperliquid_chain,
        "signatureChainId": verification.action.signature_chain_id,
        "destination": verification.action.destination,
        "token": verification.token,
        "amount": verification.action.amount,
        "time": verification.action.time
    });

    let response_status = exchange_client
        .post(action.clone(), signature, verification.nonce)
        .await
        .map_err(|e| {
            PaymentVerificationError::TransactionSimulation(format!(
                "HyperCore rejected transfer: {:?}",
                e
            ))
        })?;

    // Check if the response indicates an error
    // ExchangeResponseStatus is an enum with Ok(ExchangeResponse) and Err(String) variants
    use hyperliquid_rust_sdk::ExchangeResponseStatus;
    match &response_status {
        ExchangeResponseStatus::Ok(success) => {
            #[cfg(feature = "telemetry")]
            tracing::info!(
                payer = %verification.payer,
                destination = %verification.action.destination,
                amount = %verification.action.amount,
                token = %verification.token,
                result = ?success,
                "HyperCore spot transfer settled successfully"
            );
        }
        ExchangeResponseStatus::Err(error_msg) => {
            #[cfg(feature = "telemetry")]
            tracing::error!(
                payer = %verification.payer,
                destination = %verification.action.destination,
                amount = %verification.action.amount,
                token = %verification.token,
                error = %error_msg,
                "HyperCore spot transfer failed"
            );
            return Err(PaymentVerificationError::TransactionSimulation(format!(
                "HyperCore transfer failed: {}",
                error_msg
            )));
        }
    }

    // Return a transaction identifier
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

/// Recover the signer address from a spotSend EIP-712 signature.
fn recover_signer_from_spot_send(
    action: &types::HyperCoreUsdSendAction,
    token: &str,
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

    // Construct the EIP-712 typed data hash for spotSend
    // This follows the Hyperliquid signing spec
    let domain_separator = compute_domain_separator(&action.signature_chain_id)?;
    let struct_hash = compute_spot_send_struct_hash(action, token)?;
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

/// Compute the struct hash for a spotSend action.
fn compute_spot_send_struct_hash(
    action: &types::HyperCoreUsdSendAction,
    token: &str,
) -> Result<[u8; 32], PaymentVerificationError> {
    use ethers::utils::keccak256;

    // Type hash: keccak256("HyperliquidTransaction:SpotSend(string hyperliquidChain,string destination,string token,string amount,uint64 time)")
    let type_hash = keccak256(
        b"HyperliquidTransaction:SpotSend(string hyperliquidChain,string destination,string token,string amount,uint64 time)",
    );

    let chain_hash = keccak256(action.hyperliquid_chain.as_bytes());
    let destination_hash = keccak256(action.destination.as_bytes());
    let token_hash = keccak256(token.as_bytes());
    let amount_hash = keccak256(action.amount.as_bytes());

    // Encode and hash
    let mut encoded = Vec::with_capacity(192);
    encoded.extend_from_slice(&type_hash);
    encoded.extend_from_slice(&chain_hash);
    encoded.extend_from_slice(&destination_hash);
    encoded.extend_from_slice(&token_hash);
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
