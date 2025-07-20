use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, AccountId};
use crate::Balance;
use crate::bank::BankModule;

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Validator {
    pub address: AccountId,
    pub is_active: bool,
    pub total_delegated: Balance,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Delegation {
    pub validator: AccountId,
    pub amount: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Clone)]
pub struct UnbondingEntry {
    pub validator: AccountId,
    pub amount: Balance,
    pub release_height: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakingModule {
    validators: UnorderedMap<AccountId, Validator>,
    delegations: UnorderedMap<AccountId, Vec<Delegation>>,
    unbonding_queue: UnorderedMap<AccountId, Vec<UnbondingEntry>>,
}

impl StakingModule {
    pub fn new() -> Self {
        Self {
            validators: UnorderedMap::new(b"v".to_vec()),
            delegations: UnorderedMap::new(b"d".to_vec()),
            unbonding_queue: UnorderedMap::new(b"u".to_vec()),
        }
    }

    pub fn add_validator(&mut self, validator_address: &AccountId) {
        let validator = Validator {
            address: validator_address.clone(),
            is_active: true,
            total_delegated: 0,
        };
        
        self.validators.insert(validator_address, &validator);
        env::log_str(&format!("Staking: Added validator {}", validator_address));
    }

    pub fn delegate(&mut self, delegator: &AccountId, validator: &AccountId, amount: Balance, bank: &mut BankModule) {
        // Check if validator exists
        let mut validator_info = self.validators.get(validator)
            .expect("Validator not found");
        
        // Check if delegator has sufficient balance
        assert!(bank.has_balance(delegator, amount), "Insufficient balance for delegation");
        
        // Burn tokens from delegator (move to staking pool)
        bank.burn(delegator, amount);
        
        // Update validator total
        validator_info.total_delegated += amount;
        self.validators.insert(validator, &validator_info);
        
        // Update delegator's delegations
        let mut delegations = self.delegations.get(delegator).unwrap_or_default();
        
        // Find existing delegation or create new one
        if let Some(existing) = delegations.iter_mut().find(|d| d.validator == *validator) {
            existing.amount += amount;
        } else {
            delegations.push(Delegation {
                validator: validator.clone(),
                amount,
            });
        }
        
        self.delegations.insert(delegator, &delegations);
        env::log_str(&format!("Staking: Delegated {} to {} from {}", amount, validator, delegator));
    }

    pub fn undelegate(&mut self, delegator: &AccountId, validator: &AccountId, amount: Balance, current_height: u64) {
        let mut delegations = self.delegations.get(delegator)
            .expect("No delegations found");
        
        // Find and update delegation
        let delegation = delegations.iter_mut()
            .find(|d| d.validator == *validator)
            .expect("Delegation not found");
        
        assert!(delegation.amount >= amount, "Insufficient delegated amount");
        
        // Update delegation
        delegation.amount -= amount;
        if delegation.amount == 0 {
            delegations.retain(|d| d.validator != *validator);
        }
        
        if delegations.is_empty() {
            self.delegations.remove(delegator);
        } else {
            self.delegations.insert(delegator, &delegations);
        }
        
        // Update validator
        let mut validator_info = self.validators.get(validator).unwrap();
        validator_info.total_delegated -= amount;
        self.validators.insert(validator, &validator_info);
        
        // Add to unbonding queue (100 blocks unbonding period)
        let mut unbonding = self.unbonding_queue.get(delegator).unwrap_or_default();
        unbonding.push(UnbondingEntry {
            validator: validator.clone(),
            amount,
            release_height: current_height + 100,
        });
        self.unbonding_queue.insert(delegator, &unbonding);
        
        env::log_str(&format!("Staking: Undelegated {} from {} by {}", amount, validator, delegator));
    }

    pub fn begin_block(&mut self, _height: u64) {
        // Process any begin block logic for staking
    }

    pub fn end_block(&mut self, height: u64, bank: &mut BankModule) {
        // Process unbonding queue
        self.process_unbonding_queue(height, bank);
        
        // Distribute rewards (5% flat rate)
        self.distribute_rewards(bank);
    }

    fn process_unbonding_queue(&mut self, current_height: u64, bank: &mut BankModule) {
        let mut accounts_to_update = Vec::new();
        
        for (account, unbonding_entries) in self.unbonding_queue.iter() {
            let mut updated_entries = Vec::new();
            let mut released_amount = 0;
            
            for entry in &unbonding_entries {
                if entry.release_height <= current_height {
                    released_amount += entry.amount;
                    env::log_str(&format!("Staking: Released {} to {} at height {}", 
                        entry.amount, account, current_height));
                } else {
                    updated_entries.push(entry.clone());
                }
            }
            
            if released_amount > 0 {
                bank.mint(&account, released_amount);
            }
            
            accounts_to_update.push((account.clone(), updated_entries));
        }
        
        // Update unbonding queue
        for (account, updated_entries) in accounts_to_update {
            if updated_entries.is_empty() {
                self.unbonding_queue.remove(&account);
            } else {
                self.unbonding_queue.insert(&account, &updated_entries);
            }
        }
    }

    fn distribute_rewards(&mut self, bank: &mut BankModule) {
        // 5% reward rate per block for all delegators
        let reward_rate = 5; // 5%
        
        for (delegator, delegations) in self.delegations.iter() {
            for delegation in &delegations {
                let reward = delegation.amount * reward_rate / 100;
                if reward > 0 {
                    bank.mint(&delegator, reward);
                    env::log_str(&format!("Staking: Rewarded {} to {} for delegation to {}", 
                        reward, delegator, delegation.validator));
                }
            }
        }
    }
}