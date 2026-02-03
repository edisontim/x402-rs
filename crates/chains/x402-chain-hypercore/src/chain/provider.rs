//! HyperCore chain provider.

use ethers::signers::{LocalWallet, Signer};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use std::fmt::{Debug, Formatter};
use std::sync::Arc;
use x402_types::chain::{ChainId, ChainProviderOps};
use x402_types::scheme::X402SchemeFacilitatorError;

use crate::chain::config::HyperCoreChainConfig;
use crate::chain::types::{Address, HyperCoreChainReference};

/// Errors that can occur when interacting with a HyperCore chain provider.
#[derive(thiserror::Error, Debug)]
#[allow(clippy::enum_variant_names)]
pub enum HyperCoreChainProviderError {
    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    /// API request error.
    #[error("API error: {0}")]
    ApiError(String),
    /// Wallet/signing error.
    #[error("Wallet error: {0}")]
    WalletError(String),
}

impl From<HyperCoreChainProviderError> for X402SchemeFacilitatorError {
    fn from(value: HyperCoreChainProviderError) -> Self {
        Self::OnchainFailure(value.to_string())
    }
}

/// Provider for interacting with HyperCore (Hyperliquid native layer).
///
/// This provider handles balance queries and transfer submissions for
/// HyperCore-based x402 payments.
///
/// # Configuration
///
/// The provider requires:
/// - A chain reference (mainnet or testnet)
/// - An API URL
/// - Optionally, a signer private key for settling payments
pub struct HyperCoreChainProvider {
    /// The HyperCore network this provider connects to.
    chain: HyperCoreChainReference,
    /// The signer wallet for signing actions.
    wallet: Option<LocalWallet>,
    /// The signer address (derived from wallet).
    signer_address: Option<Address>,
    /// The Info client for read operations.
    info_client: Arc<InfoClient>,
}

impl Debug for HyperCoreChainProvider {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HyperCoreChainProvider")
            .field("chain", &self.chain)
            .field("signer_address", &self.signer_address)
            .finish()
    }
}

impl Clone for HyperCoreChainProvider {
    fn clone(&self) -> Self {
        Self {
            chain: self.chain,
            wallet: self.wallet.clone(),
            signer_address: self.signer_address.clone(),
            info_client: Arc::clone(&self.info_client),
        }
    }
}

impl HyperCoreChainProvider {
    /// Creates a new provider from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The private key is invalid
    /// - The client cannot be initialized
    pub async fn from_config(
        config: &HyperCoreChainConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let chain = config.chain_reference();
        let base_url = match chain {
            HyperCoreChainReference::Mainnet => BaseUrl::Mainnet,
            HyperCoreChainReference::Testnet => BaseUrl::Testnet,
        };

        // Create Info client (read-only, no wallet needed)
        let info_client = InfoClient::new(None, Some(base_url)).await?;

        // Parse private key and create wallet if signer is provided
        let (wallet, signer_address) = if let Some(signer) = config.signer() {
            let private_key_bytes = signer.as_bytes();
            let wallet = LocalWallet::from_bytes(private_key_bytes)
                .map_err(|e| format!("Invalid private key: {}", e))?;

            let address_bytes: [u8; 20] = wallet.address().into();
            let signer_address = Address::new(address_bytes);

            (Some(wallet), Some(signer_address))
        } else {
            (None, None)
        };

        let provider = Self::new(chain, wallet, signer_address, Arc::new(info_client));

        #[cfg(feature = "telemetry")]
        {
            let chain_id: ChainId = chain.into();
            if let Some(ref address) = provider.signer_address {
                tracing::info!(
                    chain = %chain_id,
                    address = %address,
                    "Initialized HyperCore provider with signer"
                );
            } else {
                tracing::info!(
                    chain = %chain_id,
                    "Initialized HyperCore provider without signer"
                );
            }
        }

        Ok(provider)
    }

    /// Creates a new HyperCore chain provider.
    pub fn new(
        chain: HyperCoreChainReference,
        wallet: Option<LocalWallet>,
        signer_address: Option<Address>,
        info_client: Arc<InfoClient>,
    ) -> Self {
        Self {
            chain,
            wallet,
            signer_address,
            info_client,
        }
    }

    /// Returns a reference to the Info client.
    pub fn info_client(&self) -> &InfoClient {
        &self.info_client
    }

    /// Returns the wallet, if configured.
    pub fn wallet(&self) -> Option<&LocalWallet> {
        self.wallet.as_ref()
    }

    /// Returns the signer address, if configured.
    pub fn signer_address(&self) -> Option<&Address> {
        self.signer_address.as_ref()
    }

    /// Returns the chain reference.
    pub fn chain_reference(&self) -> HyperCoreChainReference {
        self.chain
    }

    /// Query user's USDC balance on HyperCore.
    pub async fn get_usdc_balance(
        &self,
        user: &str,
    ) -> Result<String, HyperCoreChainProviderError> {
        let user_state = self
            .info_client
            .user_state(user.parse().map_err(|e| {
                HyperCoreChainProviderError::ApiError(format!("Invalid address: {}", e))
            })?)
            .await
            .map_err(|e| HyperCoreChainProviderError::ApiError(e.to_string()))?;

        // Return the withdrawable balance
        Ok(user_state.withdrawable)
    }
}

impl ChainProviderOps for HyperCoreChainProvider {
    fn signer_addresses(&self) -> Vec<String> {
        if let Some(address) = &self.signer_address {
            vec![address.to_string()]
        } else {
            vec![]
        }
    }

    fn chain_id(&self) -> ChainId {
        self.chain.into()
    }
}
