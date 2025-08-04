use crate::types::cosmos_tx::{Fee, Coin};
use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, Gas};
type Balance = u128;
use std::collections::HashMap;

/// Fee processing errors
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FeeError {
    /// Insufficient fee provided
    InsufficientFee { required: String, provided: String },
    /// Invalid fee denomination
    InvalidDenom(String),
    /// Fee calculation overflow
    CalculationOverflow,
    /// Account has insufficient balance
    InsufficientBalance { account: String, required: String, balance: String },
    /// Fee grant not found
    FeeGrantNotFound { granter: String, grantee: String },
    /// Fee grant exceeded
    FeeGrantExceeded { limit: String, requested: String },
    /// Invalid fee amount
    InvalidAmount(String),
    /// Gas limit exceeded
    GasLimitExceeded { limit: u64, requested: u64 },
}

impl std::fmt::Display for FeeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FeeError::InsufficientFee { required, provided } => {
                write!(f, "Insufficient fee: required {}, provided {}", required, provided)
            }
            FeeError::InvalidDenom(denom) => write!(f, "Invalid fee denomination: {}", denom),
            FeeError::CalculationOverflow => write!(f, "Fee calculation overflow"),
            FeeError::InsufficientBalance { account, required, balance } => {
                write!(f, "Account {} has insufficient balance: required {}, balance {}", account, required, balance)
            }
            FeeError::FeeGrantNotFound { granter, grantee } => {
                write!(f, "Fee grant not found from {} to {}", granter, grantee)
            }
            FeeError::FeeGrantExceeded { limit, requested } => {
                write!(f, "Fee grant exceeded: limit {}, requested {}", limit, requested)
            }
            FeeError::InvalidAmount(msg) => write!(f, "Invalid fee amount: {}", msg),
            FeeError::GasLimitExceeded { limit, requested } => {
                write!(f, "Gas limit exceeded: limit {}, requested {}", limit, requested)
            }
        }
    }
}

impl std::error::Error for FeeError {}

/// Fee processing configuration
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FeeConfig {
    /// Minimum gas price in yoctoNEAR per gas unit
    pub min_gas_price: Balance,
    /// Maximum gas per transaction
    pub max_gas_per_tx: Gas,
    /// Default gas limit if not specified
    pub default_gas_limit: Gas,
    /// Supported fee denominations and their conversion rates to yoctoNEAR
    pub denom_conversions: HashMap<String, Balance>,
    /// Enable fee grants
    pub enable_fee_grants: bool,
    /// Maximum fee grant allowance
    pub max_fee_grant: Balance,
}

impl Default for FeeConfig {
    fn default() -> Self {
        let mut denom_conversions = HashMap::new();
        // Default conversion rates (1 unit = X yoctoNEAR)
        denom_conversions.insert("near".to_string(), 1_000_000_000_000_000_000_000_000); // 1 NEAR
        denom_conversions.insert("unear".to_string(), 1_000_000_000_000_000); // Custom unit for fee calculation
        
        Self {
            min_gas_price: 100_000_000, // 0.0000001 NEAR per gas unit
            max_gas_per_tx: Gas::from_tgas(300), // 300 TGas max
            default_gas_limit: Gas::from_tgas(30), // 30 TGas default
            denom_conversions,
            enable_fee_grants: true,
            max_fee_grant: 10_000_000_000_000_000_000_000_000, // 10 NEAR max grant
        }
    }
}

/// Fee grant information
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct FeeGrant {
    /// Granter address
    pub granter: String,
    /// Grantee address
    pub grantee: String,
    /// Spending limit
    pub spend_limit: Vec<Coin>,
    /// Expiration (block height)
    pub expiration: Option<u64>,
}

/// Fee processor for adapting Cosmos fees to NEAR gas
#[derive(BorshSerialize, BorshDeserialize)]
pub struct FeeProcessor {
    /// Fee configuration
    config: FeeConfig,
    /// Fee grants storage (granter -> grantee -> grant)
    fee_grants: HashMap<String, HashMap<String, FeeGrant>>,
    /// Accumulated fees per denomination
    accumulated_fees: HashMap<String, Balance>,
}

impl FeeProcessor {
    /// Create a new fee processor
    pub fn new(config: FeeConfig) -> Self {
        Self {
            config,
            fee_grants: HashMap::new(),
            accumulated_fees: HashMap::new(),
        }
    }

    /// Process transaction fees
    pub fn process_transaction_fees(
        &mut self,
        fee: &Fee,
        payer: &str,
        granter: Option<&str>,
    ) -> Result<Balance, FeeError> {
        // Calculate total fee in yoctoNEAR
        let total_fee_yocto = self.calculate_fee_in_yocto(&fee.amount)?;
        
        // Calculate required gas fee
        let gas_fee = self.calculate_gas_fee(fee.gas_limit)?;
        
        // Ensure total fee covers gas cost
        if total_fee_yocto < gas_fee {
            return Err(FeeError::InsufficientFee {
                required: gas_fee.to_string(),
                provided: total_fee_yocto.to_string(),
            });
        }

        // Handle fee payment through granter if specified
        if let Some(granter_addr) = granter {
            if !granter_addr.is_empty() {
                self.process_granted_fee(granter_addr, payer, &fee.amount)?;
            } else {
                // Direct fee payment - in practice this would deduct from account
                // For now, we just validate and track
                self.validate_fee_payment(payer, total_fee_yocto)?;
            }
        } else {
            // Direct fee payment - in practice this would deduct from account
            // For now, we just validate and track
            self.validate_fee_payment(payer, total_fee_yocto)?;
        }

        // Track accumulated fees
        for coin in &fee.amount {
            let amount_yocto = self.convert_to_yocto(&coin)?;
            *self.accumulated_fees.entry(coin.denom.clone()).or_insert(0) += amount_yocto;
        }

        Ok(total_fee_yocto)
    }

    /// Calculate minimum required fee for a transaction
    pub fn calculate_minimum_fee(&self, gas_limit: u64) -> Fee {
        let gas_fee_yocto = gas_limit.saturating_mul(self.config.min_gas_price as u64) as u128;
        
        // Convert from yoctoNEAR to target denomination using ceiling division
        let conversion_rate = *self.config.denom_conversions.get("unear").unwrap_or(&1_000_000_000_000_000);
        let amount = (gas_fee_yocto + conversion_rate - 1) / conversion_rate; // Ceiling division
        
        Fee {
            amount: vec![Coin {
                denom: "unear".to_string(),
                amount: amount.to_string(),
            }],
            gas_limit,
            payer: String::new(),
            granter: String::new(),
        }
    }

    /// Calculate gas fee in yoctoNEAR
    pub fn calculate_gas_fee(&self, gas_limit: u64) -> Result<Balance, FeeError> {
        // Check gas limit
        if gas_limit > self.config.max_gas_per_tx.as_gas() {
            return Err(FeeError::GasLimitExceeded {
                limit: self.config.max_gas_per_tx.as_gas(),
                requested: gas_limit,
            });
        }

        // Calculate fee (gas_limit * min_gas_price)
        gas_limit
            .checked_mul(self.config.min_gas_price as u64)
            .map(|fee| fee as Balance)
            .ok_or(FeeError::CalculationOverflow)
    }

    /// Convert fee amounts to yoctoNEAR
    pub fn calculate_fee_in_yocto(&self, coins: &[Coin]) -> Result<Balance, FeeError> {
        let mut total = 0u128;

        for coin in coins {
            let amount_yocto = self.convert_to_yocto(coin)?;
            total = total.checked_add(amount_yocto)
                .ok_or(FeeError::CalculationOverflow)?;
        }

        Ok(total)
    }

    /// Convert a single coin to yoctoNEAR
    fn convert_to_yocto(&self, coin: &Coin) -> Result<Balance, FeeError> {
        // Get conversion rate
        let rate = self.config.denom_conversions
            .get(&coin.denom)
            .ok_or_else(|| FeeError::InvalidDenom(coin.denom.clone()))?;

        // Parse amount
        let amount: u128 = coin.amount.parse()
            .map_err(|_| FeeError::InvalidAmount(coin.amount.clone()))?;

        // Calculate yoctoNEAR amount
        amount.checked_mul(*rate)
            .ok_or(FeeError::CalculationOverflow)
    }

    /// Validate that payer can afford the fee
    fn validate_fee_payment(&self, _payer: &str, _amount: Balance) -> Result<(), FeeError> {
        // In a real implementation, this would check account balance
        // For now, we assume validation passes
        Ok(())
    }

    /// Process fee payment through a fee grant
    fn process_granted_fee(
        &mut self,
        granter: &str,
        grantee: &str,
        fee_amount: &[Coin],
    ) -> Result<(), FeeError> {
        // Get fee grant
        let grant = self.fee_grants
            .get_mut(granter)
            .and_then(|grants| grants.get_mut(grantee))
            .ok_or_else(|| FeeError::FeeGrantNotFound {
                granter: granter.to_string(),
                grantee: grantee.to_string(),
            })?;

        // Check expiration
        if let Some(expiration) = grant.expiration {
            if env::block_height() > expiration {
                return Err(FeeError::FeeGrantNotFound {
                    granter: granter.to_string(),
                    grantee: grantee.to_string(),
                });
            }
        }

        // Check and update spend limit
        for fee_coin in fee_amount {
            let mut limit_updated = false;
            
            for limit_coin in &mut grant.spend_limit {
                if limit_coin.denom == fee_coin.denom {
                    let limit_amount: u128 = limit_coin.amount.parse()
                        .map_err(|_| FeeError::InvalidAmount(limit_coin.amount.clone()))?;
                    let fee_amount: u128 = fee_coin.amount.parse()
                        .map_err(|_| FeeError::InvalidAmount(fee_coin.amount.clone()))?;

                    if fee_amount > limit_amount {
                        return Err(FeeError::FeeGrantExceeded {
                            limit: limit_coin.amount.clone(),
                            requested: fee_coin.amount.clone(),
                        });
                    }

                    // Update remaining limit
                    limit_coin.amount = (limit_amount - fee_amount).to_string();
                    limit_updated = true;
                    break;
                }
            }

            if !limit_updated {
                return Err(FeeError::InvalidDenom(fee_coin.denom.clone()));
            }
        }

        Ok(())
    }

    /// Grant fee allowance
    pub fn grant_fee_allowance(&mut self, grant: FeeGrant) -> Result<(), FeeError> {
        if !self.config.enable_fee_grants {
            return Err(FeeError::FeeGrantNotFound {
                granter: grant.granter.clone(),
                grantee: grant.grantee.clone(),
            });
        }

        // Validate grant amount
        let total_grant = self.calculate_fee_in_yocto(&grant.spend_limit)?;
        if total_grant > self.config.max_fee_grant {
            return Err(FeeError::FeeGrantExceeded {
                limit: self.config.max_fee_grant.to_string(),
                requested: total_grant.to_string(),
            });
        }

        // Store grant
        self.fee_grants
            .entry(grant.granter.clone())
            .or_insert_with(HashMap::new)
            .insert(grant.grantee.clone(), grant);

        Ok(())
    }

    /// Revoke fee allowance
    pub fn revoke_fee_allowance(&mut self, granter: &str, grantee: &str) -> Result<(), FeeError> {
        self.fee_grants
            .get_mut(granter)
            .and_then(|grants| grants.remove(grantee))
            .ok_or_else(|| FeeError::FeeGrantNotFound {
                granter: granter.to_string(),
                grantee: grantee.to_string(),
            })?;

        Ok(())
    }

    /// Get fee grant
    pub fn get_fee_grant(&self, granter: &str, grantee: &str) -> Option<&FeeGrant> {
        self.fee_grants
            .get(granter)
            .and_then(|grants| grants.get(grantee))
    }

    /// Get accumulated fees
    pub fn get_accumulated_fees(&self) -> &HashMap<String, Balance> {
        &self.accumulated_fees
    }

    /// Clear accumulated fees (for distribution)
    pub fn clear_accumulated_fees(&mut self) -> HashMap<String, Balance> {
        std::mem::take(&mut self.accumulated_fees)
    }

    /// Update fee configuration
    pub fn update_config(&mut self, config: FeeConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &FeeConfig {
        &self.config
    }

    /// Add or update denomination conversion rate
    pub fn set_denom_conversion(&mut self, denom: String, rate: Balance) {
        self.config.denom_conversions.insert(denom, rate);
    }

    /// Estimate transaction cost in a specific denomination
    pub fn estimate_tx_cost(&self, gas_limit: u64, denom: &str) -> Result<Coin, FeeError> {
        let gas_fee_yocto = self.calculate_gas_fee(gas_limit)?;
        
        // Get conversion rate
        let rate = self.config.denom_conversions
            .get(denom)
            .ok_or_else(|| FeeError::InvalidDenom(denom.to_string()))?;

        // Convert from yoctoNEAR to target denomination
        // Use ceiling division to avoid losing precision
        let amount = (gas_fee_yocto + rate - 1) / rate;
        
        Ok(Coin {
            denom: denom.to_string(),
            amount: amount.to_string(),
        })
    }

    /// Calculate effective gas price based on provided fee
    pub fn calculate_effective_gas_price(&self, fee: &Fee) -> Result<Balance, FeeError> {
        let total_fee_yocto = self.calculate_fee_in_yocto(&fee.amount)?;
        
        if fee.gas_limit == 0 {
            return Err(FeeError::InvalidAmount("Gas limit cannot be zero".to_string()));
        }

        Ok(total_fee_yocto / fee.gas_limit as Balance)
    }

    /// Check if fee meets minimum requirements
    pub fn validate_minimum_fee(&self, fee: &Fee) -> Result<(), FeeError> {
        let required_fee = self.calculate_gas_fee(fee.gas_limit)?;
        let provided_fee = self.calculate_fee_in_yocto(&fee.amount)?;

        if provided_fee < required_fee {
            return Err(FeeError::InsufficientFee {
                required: required_fee.to_string(),
                provided: provided_fee.to_string(),
            });
        }

        Ok(())
    }
}

/// Utility functions for fee handling
pub mod utils {
    use super::*;

    /// Parse fee string in format "1000unear" or "1near"
    pub fn parse_fee_string(fee_str: &str) -> Result<Coin, FeeError> {
        // Find where the numeric part ends
        let numeric_end = fee_str.chars()
            .position(|c| !c.is_numeric())
            .unwrap_or(fee_str.len());

        if numeric_end == 0 || numeric_end == fee_str.len() {
            return Err(FeeError::InvalidAmount(fee_str.to_string()));
        }

        let amount = &fee_str[..numeric_end];
        let denom = &fee_str[numeric_end..];

        // Validate amount can be parsed
        let _: u128 = amount.parse()
            .map_err(|_| FeeError::InvalidAmount(amount.to_string()))?;

        Ok(Coin {
            denom: denom.to_string(),
            amount: amount.to_string(),
        })
    }

    /// Format fee for display
    pub fn format_fee(coins: &[Coin]) -> String {
        if coins.is_empty() {
            return "0".to_string();
        }

        coins.iter()
            .map(|coin| format!("{}{}", coin.amount, coin.denom))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Calculate priority based on fee (higher fee = higher priority)
    pub fn calculate_tx_priority(fee: &Fee, gas_price: Balance) -> u64 {
        // Simple priority calculation based on effective gas price
        let effective_price = (fee.gas_limit as Balance).saturating_mul(gas_price);
        
        // Normalize to a priority value (0-1000)
        std::cmp::min(effective_price / 1_000_000_000_000_000, 1000) as u64
    }

    /// Check if denomination is native NEAR
    pub fn is_near_denom(denom: &str) -> bool {
        matches!(denom, "near" | "unear" | "NEAR")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fee_processor() -> FeeProcessor {
        FeeProcessor::new(FeeConfig::default())
    }

    #[test]
    fn test_fee_processor_creation() {
        let processor = create_test_fee_processor();
        assert_eq!(processor.config.min_gas_price, 100_000_000);
        assert_eq!(processor.config.max_gas_per_tx, Gas::from_tgas(300));
    }

    #[test]
    fn test_calculate_minimum_fee() {
        let processor = create_test_fee_processor();
        let fee = processor.calculate_minimum_fee(100_000_000); // 0.1 TGas
        
        assert_eq!(fee.gas_limit, 100_000_000);
        assert_eq!(fee.amount.len(), 1);
        assert_eq!(fee.amount[0].denom, "unear");
    }

    #[test]
    fn test_calculate_gas_fee() {
        let processor = create_test_fee_processor();
        
        // Test normal gas limit
        let fee = processor.calculate_gas_fee(100_000_000).unwrap();
        assert_eq!(fee, 10_000_000_000_000_000); // 0.01 NEAR
        
        // Test gas limit exceeded
        let result = processor.calculate_gas_fee(400_000_000_000_000); // 400 TGas
        assert!(matches!(result, Err(FeeError::GasLimitExceeded { .. })));
    }

    #[test]
    fn test_convert_to_yocto() {
        let processor = create_test_fee_processor();
        
        // Test NEAR conversion
        let coin = Coin::new("near", "1");
        let yocto = processor.convert_to_yocto(&coin).unwrap();
        assert_eq!(yocto, 1_000_000_000_000_000_000_000_000);
        
        // Test custom unear conversion (1 million unear = 1 NEAR)
        let coin = Coin::new("unear", "1000000");
        let yocto = processor.convert_to_yocto(&coin).unwrap();
        assert_eq!(yocto, 1_000_000_000_000_000_000_000);
        
        // Test invalid denomination
        let coin = Coin::new("invalid", "1");
        let result = processor.convert_to_yocto(&coin);
        assert!(matches!(result, Err(FeeError::InvalidDenom(_))));
    }

    #[test]
    fn test_process_transaction_fees() {
        let mut processor = create_test_fee_processor();
        
        let fee = Fee {
            amount: vec![Coin::new("unear", "1000000")], // 1 NEAR
            gas_limit: 100_000_000, // 0.1 TGas
            payer: String::new(),
            granter: String::new(),
        };
        
        let total_fee = processor.process_transaction_fees(&fee, "alice", None).unwrap();
        assert_eq!(total_fee, 1_000_000_000_000_000_000_000); // 1 NEAR with new conversion rate
        
        // Check accumulated fees
        let accumulated = processor.get_accumulated_fees();
        assert_eq!(accumulated.get("unear"), Some(&1_000_000_000_000_000_000_000));
    }

    #[test]
    fn test_insufficient_fee() {
        let mut processor = create_test_fee_processor();
        
        // Required fee is 10 unear (0.01 NEAR), so 1 unear should be insufficient
        let fee = Fee {
            amount: vec![Coin::new("unear", "1")], // Too small - only 1 unear when 10 is needed
            gas_limit: 100_000_000, // 0.1 TGas
            payer: String::new(),
            granter: String::new(),
        };
        
        let result = processor.process_transaction_fees(&fee, "alice", None);
        assert!(matches!(result, Err(FeeError::InsufficientFee { .. })));
    }

    #[test]
    fn test_fee_grants() {
        let mut processor = create_test_fee_processor();
        
        // Grant fee allowance
        let grant = FeeGrant {
            granter: "bob".to_string(),
            grantee: "alice".to_string(),
            spend_limit: vec![Coin::new("unear", "5000000")], // 5 NEAR
            expiration: None,
        };
        
        processor.grant_fee_allowance(grant).unwrap();
        
        // Process fee using grant
        let fee = Fee {
            amount: vec![Coin::new("unear", "1000000")], // 1 NEAR
            gas_limit: 100_000_000,
            payer: String::new(),
            granter: "bob".to_string(),
        };
        
        processor.process_transaction_fees(&fee, "alice", Some("bob")).unwrap();
        
        // Check remaining grant
        let remaining_grant = processor.get_fee_grant("bob", "alice").unwrap();
        assert_eq!(remaining_grant.spend_limit[0].amount, "4000000");
    }

    #[test]
    fn test_fee_grant_exceeded() {
        let mut processor = create_test_fee_processor();
        
        // Grant small fee allowance
        let grant = FeeGrant {
            granter: "bob".to_string(),
            grantee: "alice".to_string(),
            spend_limit: vec![Coin::new("unear", "100")], // Very small
            expiration: None,
        };
        
        processor.grant_fee_allowance(grant).unwrap();
        
        // Try to use more than granted
        let fee = Fee {
            amount: vec![Coin::new("unear", "1000000")], // 1 NEAR
            gas_limit: 100_000_000,
            payer: String::new(),
            granter: "bob".to_string(),
        };
        
        let result = processor.process_transaction_fees(&fee, "alice", Some("bob"));
        assert!(matches!(result, Err(FeeError::FeeGrantExceeded { .. })));
    }

    #[test]
    fn test_estimate_tx_cost() {
        let processor = create_test_fee_processor();
        
        // Estimate in NEAR - should round up to 1 (smallest unit)
        let cost = processor.estimate_tx_cost(100_000_000, "near").unwrap();
        assert_eq!(cost.denom, "near");
        assert_eq!(cost.amount, "1"); // Rounds up to 1 NEAR (0.01 NEAR rounds up)
        
        // Estimate in microNEAR
        let cost = processor.estimate_tx_cost(100_000_000, "unear").unwrap();
        assert_eq!(cost.denom, "unear");
        assert_eq!(cost.amount, "10"); // 0.01 NEAR = 10 unear
    }

    #[test]
    fn test_parse_fee_string() {
        let coin = utils::parse_fee_string("1000unear").unwrap();
        assert_eq!(coin.amount, "1000");
        assert_eq!(coin.denom, "unear");
        
        let coin = utils::parse_fee_string("1near").unwrap();
        assert_eq!(coin.amount, "1");
        assert_eq!(coin.denom, "near");
        
        // Test invalid format
        let result = utils::parse_fee_string("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_format_fee() {
        let coins = vec![
            Coin::new("near", "1"),
            Coin::new("unear", "1000000"),
        ];
        
        let formatted = utils::format_fee(&coins);
        assert_eq!(formatted, "1near, 1000000unear");
    }

    #[test]
    fn test_calculate_effective_gas_price() {
        let processor = create_test_fee_processor();
        
        let fee = Fee {
            amount: vec![Coin::new("unear", "2000000")], // 2 NEAR
            gas_limit: 200_000_000, // 0.2 TGas
            payer: String::new(),
            granter: String::new(),
        };
        
        let gas_price = processor.calculate_effective_gas_price(&fee).unwrap();
        assert_eq!(gas_price, 10_000_000_000_000); // Gas price with new conversion rate
    }

    #[test]
    fn test_denom_conversion_updates() {
        let mut processor = create_test_fee_processor();
        
        // Add new denomination
        processor.set_denom_conversion("atom".to_string(), 5_000_000_000_000_000_000_000_000);
        
        // Test conversion
        let coin = Coin::new("atom", "1");
        let yocto = processor.convert_to_yocto(&coin).unwrap();
        assert_eq!(yocto, 5_000_000_000_000_000_000_000_000); // 5 NEAR worth
    }
}