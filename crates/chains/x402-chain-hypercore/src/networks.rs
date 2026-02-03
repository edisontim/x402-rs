//! Known HyperCore networks and token deployments.
//!
//! This module provides constants and helpers for working with known
//! HyperCore networks and their token deployments.

use crate::chain::HyperCoreChainReference;

/// Trait for types that can provide network-specific instances.
pub trait KnownNetworkHyperCore: Sized {
    /// Returns the mainnet instance.
    fn hypercore() -> Self {
        Self::hypercore_mainnet()
    }

    /// Returns the mainnet instance.
    fn hypercore_mainnet() -> Self;

    /// Returns the testnet instance.
    fn hypercore_testnet() -> Self;
}

impl KnownNetworkHyperCore for HyperCoreChainReference {
    fn hypercore_mainnet() -> Self {
        Self::Mainnet
    }

    fn hypercore_testnet() -> Self {
        Self::Testnet
    }
}

/// USDC on HyperCore.
///
/// Note: On HyperCore, USDC is the native settlement currency and is
/// transferred using `usdSend` rather than a token address.
pub struct USDC;

impl USDC {
    /// Returns the number of decimals for USDC (6).
    pub const DECIMALS: u8 = 6;

    /// Format an amount for HyperCore API (string with appropriate precision).
    pub fn format_amount(atomic_units: u64) -> String {
        let whole = atomic_units / 1_000_000;
        let frac = atomic_units % 1_000_000;
        if frac == 0 {
            whole.to_string()
        } else {
            format!("{}.{:06}", whole, frac)
                .trim_end_matches('0')
                .to_string()
        }
    }

    /// Parse an amount from HyperCore API format to atomic units.
    pub fn parse_amount(s: &str) -> Result<u64, String> {
        let parts: Vec<&str> = s.split('.').collect();
        match parts.len() {
            1 => {
                let whole: u64 = parts[0]
                    .parse()
                    .map_err(|e| format!("Invalid amount: {}", e))?;
                Ok(whole * 1_000_000)
            }
            2 => {
                let whole: u64 = parts[0]
                    .parse()
                    .map_err(|e| format!("Invalid amount: {}", e))?;
                let frac_str = format!("{:0<6}", parts[1]);
                let frac: u64 = frac_str[..6]
                    .parse()
                    .map_err(|e| format!("Invalid amount: {}", e))?;
                Ok(whole * 1_000_000 + frac)
            }
            _ => Err(format!("Invalid amount format: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usdc_format_amount() {
        assert_eq!(USDC::format_amount(1_000_000), "1");
        assert_eq!(USDC::format_amount(1_500_000), "1.5");
        assert_eq!(USDC::format_amount(1_234_567), "1.234567");
        assert_eq!(USDC::format_amount(500_000), "0.5");
    }

    #[test]
    fn test_usdc_parse_amount() {
        assert_eq!(USDC::parse_amount("1").unwrap(), 1_000_000);
        assert_eq!(USDC::parse_amount("1.5").unwrap(), 1_500_000);
        assert_eq!(USDC::parse_amount("1.234567").unwrap(), 1_234_567);
        assert_eq!(USDC::parse_amount("0.5").unwrap(), 500_000);
    }
}
