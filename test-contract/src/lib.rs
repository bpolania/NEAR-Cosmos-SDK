use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    /// Account balances (mimicking our bank module)
    balances: UnorderedMap<AccountId, Balance>,
    /// Current block height (for testing block processing)
    block_height: u64,
    /// Simple validator set (for staking module testing)
    validators: UnorderedMap<AccountId, ValidatorInfo>,
    /// Governance parameters (for governance module testing)
    parameters: UnorderedMap<String, String>,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ValidatorInfo {
    pub delegated_stake: Balance,
    pub is_active: bool,
}

#[near_bindgen]
impl CosmosContract {
    #[init]
    pub fn new() -> Self {
        Self {
            balances: UnorderedMap::new(b"b"),
            block_height: 0,
            validators: UnorderedMap::new(b"v"),
            parameters: UnorderedMap::new(b"p"),
        }
    }

    // Bank Module Functions
    pub fn transfer(&mut self, sender: AccountId, receiver: AccountId, amount: Balance) {
        let sender_balance = self.balances.get(&sender).unwrap_or(0);
        assert!(sender_balance >= amount, "Insufficient balance");
        
        let receiver_balance = self.balances.get(&receiver).unwrap_or(0);
        
        self.balances.insert(&sender, &(sender_balance - amount));
        self.balances.insert(&receiver, &(receiver_balance + amount));
        
        env::log_str(&format!("Transfer: {} -> {} amount: {}", sender, receiver, amount));
    }

    pub fn mint(&mut self, receiver: AccountId, amount: Balance) {
        let current_balance = self.balances.get(&receiver).unwrap_or(0);
        self.balances.insert(&receiver, &(current_balance + amount));
        
        env::log_str(&format!("Mint: {} amount: {}", receiver, amount));
    }

    pub fn get_balance(&self, account: AccountId) -> Balance {
        self.balances.get(&account).unwrap_or(0)
    }

    // Staking Module Functions
    pub fn add_validator(&mut self, address: AccountId) {
        let validator = ValidatorInfo {
            delegated_stake: 0,
            is_active: true,
        };
        self.validators.insert(&address, &validator);
        
        env::log_str(&format!("Added validator: {}", address));
    }

    pub fn delegate(&mut self, delegator: AccountId, validator: AccountId, amount: Balance) {
        // Simulate transferring tokens to staking pool
        self.transfer(delegator, "staking_pool.testnet".parse().unwrap(), amount);
        
        // Update validator info
        let mut validator_info = self.validators.get(&validator).expect("Validator not found");
        validator_info.delegated_stake += amount;
        self.validators.insert(&validator, &validator_info);
        
        env::log_str(&format!("Delegated: {} -> {} amount: {}", delegator, validator, amount));
    }

    pub fn undelegate(&mut self, delegator: AccountId, validator: AccountId, amount: Balance) {
        let mut validator_info = self.validators.get(&validator).expect("Validator not found");
        assert!(validator_info.delegated_stake >= amount, "Insufficient delegation");
        
        validator_info.delegated_stake -= amount;
        self.validators.insert(&validator, &validator_info);
        
        // In real implementation, this would go to unbonding queue
        // For testing, immediately return tokens
        self.transfer("staking_pool.testnet".parse().unwrap(), delegator, amount);
        
        env::log_str(&format!("Undelegated: {} from {} amount: {}", delegator, validator, amount));
    }

    // Governance Module Functions
    pub fn submit_proposal(&mut self, title: String, description: String, param_key: String, param_value: String) -> u64 {
        // Simple proposal ID generation
        let proposal_id = self.block_height + 1;
        
        env::log_str(&format!("Proposal submitted: {} - {}", proposal_id, title));
        proposal_id
    }

    pub fn vote(&mut self, proposal_id: u64, option: u8) {
        let voter = env::predecessor_account_id();
        
        env::log_str(&format!("Vote cast: {} on proposal {} with option {}", voter, proposal_id, option));
    }

    pub fn get_parameter(&self, key: String) -> Option<String> {
        self.parameters.get(&key)
    }

    // Block Processing Functions
    pub fn process_block(&mut self) {
        self.block_height += 1;
        
        // Simulate reward distribution (5% of total staked)
        let total_staked: Balance = self.validators
            .values()
            .map(|v| v.delegated_stake)
            .sum();
        
        if total_staked > 0 {
            let rewards = total_staked * 5 / 100;
            self.mint("staking_pool.testnet".parse().unwrap(), rewards);
        }
        
        env::log_str(&format!("Processed block: {}", self.block_height));
    }

    // Utility functions
    pub fn get_block_height(&self) -> u64 {
        self.block_height
    }
}