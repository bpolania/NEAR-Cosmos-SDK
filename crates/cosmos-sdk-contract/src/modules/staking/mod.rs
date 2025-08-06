use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::env;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::Balance;
// use crate::modules::bank::BankModule; // Not needed currently
// use crate::modules::ibc::transfer::FungibleTokenPacketData; // Not needed currently

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum ValidatorStatus {
    Unspecified,
    Unbonded,
    Unbonding,
    Bonded,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Validator {
    pub address: String,
    pub operator_address: String,
    pub consensus_pubkey: Vec<u8>,
    pub jailed: bool,
    pub status: ValidatorStatus,
    pub tokens: Balance,
    pub delegator_shares: String,
    pub description: ValidatorDescription,
    pub unbonding_height: u64,
    pub unbonding_time: u64,
    pub commission: Commission,
    pub min_self_delegation: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct ValidatorDescription {
    pub moniker: String,
    pub identity: String,
    pub website: String,
    pub security_contact: String,
    pub details: String,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Commission {
    pub commission_rates: CommissionRates,
    pub update_time: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct CommissionRates {
    pub rate: String,
    pub max_rate: String,
    pub max_change_rate: String,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Delegation {
    pub delegator_address: String,
    pub validator_address: String,
    pub shares: String,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct UnbondingDelegation {
    pub delegator_address: String,
    pub validator_address: String,
    pub entries: Vec<UnbondingDelegationEntry>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct UnbondingDelegationEntry {
    pub creation_height: u64,
    pub completion_time: u64,
    pub initial_balance: Balance,
    pub balance: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Pool {
    pub not_bonded_tokens: Balance,
    pub bonded_tokens: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct Params {
    pub unbonding_time: u64,
    pub max_validators: u32,
    pub max_entries: u32,
    pub historical_entries: u32,
    pub bond_denom: String,
    pub min_commission_rate: String,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakingModule {
    validators: UnorderedMap<String, Validator>,
    delegations: UnorderedMap<String, Delegation>,
    unbonding_delegations: UnorderedMap<String, UnbondingDelegation>,
    pool: Pool,
    params: Params,
}

impl StakingModule {
    pub fn new() -> Self {
        Self {
            validators: UnorderedMap::new(b"v".to_vec()),
            delegations: UnorderedMap::new(b"d".to_vec()),
            unbonding_delegations: UnorderedMap::new(b"u".to_vec()),
            pool: Pool {
                not_bonded_tokens: 0,
                bonded_tokens: 0,
            },
            params: Params {
                unbonding_time: 1814400, // 21 days in seconds
                max_validators: 100,
                max_entries: 7,
                historical_entries: 10000,
                bond_denom: "stake".to_string(),
                min_commission_rate: "0.0".to_string(),
            },
        }
    }

    // Validator management
    pub fn create_validator(
        &mut self,
        validator_address: String,
        pubkey: Vec<u8>,
        moniker: String,
        identity: Option<String>,
        website: Option<String>,
        security_contact: Option<String>,
        details: Option<String>,
        commission_rate: String,
        commission_max_rate: String,
        commission_max_change_rate: String,
        min_self_delegation: Balance,
        self_delegation: Balance,
    ) -> Result<(), String> {
        // Check if validator already exists
        if self.validators.get(&validator_address).is_some() {
            return Err("Validator already exists".to_string());
        }

        let validator = Validator {
            address: validator_address.clone(),
            operator_address: validator_address.clone(),
            consensus_pubkey: pubkey,
            jailed: false,
            status: ValidatorStatus::Bonded,
            tokens: self_delegation,
            delegator_shares: self_delegation.to_string(),
            description: ValidatorDescription {
                moniker,
                identity: identity.unwrap_or_default(),
                website: website.unwrap_or_default(),
                security_contact: security_contact.unwrap_or_default(),
                details: details.unwrap_or_default(),
            },
            unbonding_height: 0,
            unbonding_time: 0,
            commission: Commission {
                commission_rates: CommissionRates {
                    rate: commission_rate,
                    max_rate: commission_max_rate,
                    max_change_rate: commission_max_change_rate,
                },
                update_time: env::block_timestamp(),
            },
            min_self_delegation,
        };

        self.validators.insert(&validator_address, &validator);
        self.pool.bonded_tokens += self_delegation;

        env::log_str(&format!("Created validator: {}", validator_address));
        Ok(())
    }

    pub fn edit_validator(
        &mut self,
        validator_address: String,
        moniker: Option<String>,
        identity: Option<String>,
        website: Option<String>,
        security_contact: Option<String>,
        details: Option<String>,
        commission_rate: Option<String>,
        min_self_delegation: Option<Balance>,
    ) -> Result<(), String> {
        let mut validator = self.validators.get(&validator_address)
            .ok_or("Validator not found")?;

        if let Some(moniker) = moniker {
            validator.description.moniker = moniker;
        }
        if let Some(identity) = identity {
            validator.description.identity = identity;
        }
        if let Some(website) = website {
            validator.description.website = website;
        }
        if let Some(security_contact) = security_contact {
            validator.description.security_contact = security_contact;
        }
        if let Some(details) = details {
            validator.description.details = details;
        }
        if let Some(commission_rate) = commission_rate {
            validator.commission.commission_rates.rate = commission_rate;
            validator.commission.update_time = env::block_timestamp();
        }
        if let Some(min_self_delegation) = min_self_delegation {
            validator.min_self_delegation = min_self_delegation;
        }

        self.validators.insert(&validator_address, &validator);
        env::log_str(&format!("Edited validator: {}", validator_address));
        Ok(())
    }

    // Delegation functions
    pub fn delegate(&mut self, delegator: String, validator_address: String, amount: Balance) -> Result<(), String> {
        let mut validator = self.validators.get(&validator_address)
            .ok_or("Validator not found")?;

        if validator.status != ValidatorStatus::Bonded {
            return Err("Validator not bonded".to_string());
        }

        // Update validator
        validator.tokens += amount;
        let new_shares = amount; // Simplified 1:1 share ratio
        validator.delegator_shares = (validator.delegator_shares.parse::<Balance>().unwrap_or(0) + new_shares).to_string();
        self.validators.insert(&validator_address, &validator);

        // Create or update delegation
        let delegation_key = format!("{}#{}", delegator, validator_address);
        let delegation = Delegation {
            delegator_address: delegator.clone(),
            validator_address: validator_address.clone(),
            shares: new_shares.to_string(),
        };
        self.delegations.insert(&delegation_key, &delegation);

        // Update pool
        self.pool.bonded_tokens += amount;

        env::log_str(&format!("Delegated {} from {} to {}", amount, delegator, validator_address));
        Ok(())
    }

    pub fn undelegate(&mut self, delegator: String, validator_address: String, amount: Balance) -> Result<u64, String> {
        let delegation_key = format!("{}#{}", delegator, validator_address);
        let mut delegation = self.delegations.get(&delegation_key)
            .ok_or("Delegation not found")?;

        let current_shares: Balance = delegation.shares.parse().map_err(|_| "Invalid shares")?;
        if current_shares < amount {
            return Err("Insufficient delegation".to_string());
        }

        // Update delegation
        let new_shares = current_shares - amount;
        if new_shares == 0 {
            self.delegations.remove(&delegation_key);
        } else {
            delegation.shares = new_shares.to_string();
            self.delegations.insert(&delegation_key, &delegation);
        }

        // Update validator
        let mut validator = self.validators.get(&validator_address).unwrap();
        validator.tokens -= amount;
        let total_shares: Balance = validator.delegator_shares.parse().unwrap_or(0);
        validator.delegator_shares = (total_shares - amount).to_string();
        self.validators.insert(&validator_address, &validator);

        // Create unbonding delegation
        let completion_time = env::block_timestamp() + self.params.unbonding_time * 1_000_000_000; // Convert to nanoseconds
        let unbonding_key = format!("{}#{}", delegator, validator_address);
        
        let mut unbonding = self.unbonding_delegations.get(&unbonding_key)
            .unwrap_or(UnbondingDelegation {
                delegator_address: delegator.clone(),
                validator_address: validator_address.clone(),
                entries: vec![],
            });

        unbonding.entries.push(UnbondingDelegationEntry {
            creation_height: env::block_height(),
            completion_time,
            initial_balance: amount,
            balance: amount,
        });

        self.unbonding_delegations.insert(&unbonding_key, &unbonding);

        // Update pool
        self.pool.bonded_tokens -= amount;
        self.pool.not_bonded_tokens += amount;

        env::log_str(&format!("Started unbonding {} from {} to {}", amount, delegator, validator_address));
        Ok(completion_time)
    }

    pub fn redelegate(&mut self, delegator: String, validator_src: String, validator_dst: String, amount: Balance) -> Result<u64, String> {
        // Simplified redelegation - just move delegation
        self.undelegate(delegator.clone(), validator_src, amount)?;
        self.delegate(delegator, validator_dst, amount)?;
        
        let completion_time = env::block_timestamp() + self.params.unbonding_time * 1_000_000_000;
        Ok(completion_time)
    }

    // Query functions
    pub fn get_validator(&self, validator_address: String) -> Option<Validator> {
        self.validators.get(&validator_address)
    }

    pub fn get_all_validators(&self) -> Vec<Validator> {
        self.validators.values().collect()
    }

    pub fn get_bonded_validators(&self) -> Vec<Validator> {
        self.validators.values()
            .filter(|v| v.status == ValidatorStatus::Bonded)
            .collect()
    }

    pub fn get_delegation(&self, delegator: String, validator_address: String) -> Option<Delegation> {
        let key = format!("{}#{}", delegator, validator_address);
        self.delegations.get(&key)
    }

    pub fn get_delegations(&self, delegator: String) -> Vec<Delegation> {
        self.delegations.values()
            .filter(|d| d.delegator_address == delegator)
            .collect()
    }

    pub fn get_validator_delegations(&self, validator_address: String) -> Vec<Delegation> {
        self.delegations.values()
            .filter(|d| d.validator_address == validator_address)
            .collect()
    }

    pub fn get_unbonding_delegation(&self, delegator: String, validator_address: String) -> Option<UnbondingDelegation> {
        let key = format!("{}#{}", delegator, validator_address);
        self.unbonding_delegations.get(&key)
    }

    pub fn get_unbonding_delegations(&self, delegator: String) -> Vec<UnbondingDelegation> {
        self.unbonding_delegations.values()
            .filter(|ud| ud.delegator_address == delegator)
            .collect()
    }

    pub fn get_pool(&self) -> Pool {
        self.pool.clone()
    }

    pub fn get_params(&self) -> Params {
        self.params.clone()
    }

    // Rewards and slashing
    pub fn withdraw_delegator_reward(&mut self, delegator: String, validator_address: String) -> Result<Balance, String> {
        // Simplified reward calculation - 5% of delegation
        if let Some(delegation) = self.get_delegation(delegator, validator_address) {
            let shares: Balance = delegation.shares.parse().map_err(|_| "Invalid shares")?;
            let reward = shares * 5 / 100; // 5% reward
            Ok(reward)
        } else {
            Err("Delegation not found".to_string())
        }
    }

    pub fn slash_validator(&mut self, validator_address: String, _height: u64, _power: u64, slash_fraction: String) -> Result<Balance, String> {
        let mut validator = self.validators.get(&validator_address)
            .ok_or("Validator not found")?;

        let slash_rate: f64 = slash_fraction.parse().map_err(|_| "Invalid slash fraction")?;
        let slashed_amount = (validator.tokens as f64 * slash_rate) as Balance;
        
        validator.tokens -= slashed_amount;
        validator.jailed = true;
        validator.status = ValidatorStatus::Unbonding;
        
        self.validators.insert(&validator_address, &validator);
        self.pool.bonded_tokens -= slashed_amount;

        env::log_str(&format!("Slashed validator {} by {}", validator_address, slashed_amount));
        Ok(slashed_amount)
    }

    pub fn refund_tokens(&mut self, _data: serde_json::Value) -> Result<(), String> {
        // Placeholder for token refund logic
        Ok(())
    }

    pub fn add_validator(&mut self, validator: Validator) -> Result<(), String> {
        if self.validators.get(&validator.address).is_some() {
            return Err("Validator already exists".to_string());
        }
        
        self.validators.insert(&validator.address, &validator);
        env::log_str(&format!("Added validator: {}", validator.address));
        Ok(())
    }

    pub fn begin_block(&mut self, _height: u64) {
        // Begin block processing - update validator set, process slashing, etc.
        env::log_str("Staking module begin block processing");
    }

    pub fn end_block(&mut self, _height: u64) {
        // End block processing - finalize validator updates, distribute rewards, etc.
        env::log_str("Staking module end block processing");
    }
}