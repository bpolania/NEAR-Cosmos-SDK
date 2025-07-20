use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

pub type Balance = u128;

mod bank;
mod governance;
mod staking;

use bank::BankModule;
use governance::GovernanceModule;
use staking::StakingModule;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    bank_module: BankModule,
    staking_module: StakingModule,
    governance_module: GovernanceModule,
    block_height: u64,
}

#[near_bindgen]
impl CosmosContract {
    #[init]
    pub fn new() -> Self {
        Self {
            bank_module: BankModule::new(),
            staking_module: StakingModule::new(),
            governance_module: GovernanceModule::new(),
            block_height: 0,
        }
    }

    // Bank Module Functions
    pub fn transfer(&mut self, receiver: AccountId, amount: Balance) -> String {
        let sender = env::predecessor_account_id();
        self.bank_module.transfer(&sender, &receiver, amount);
        format!("Transferred {} from {} to {}", amount, sender, receiver)
    }

    pub fn mint(&mut self, receiver: AccountId, amount: Balance) -> String {
        self.bank_module.mint(&receiver, amount);
        format!("Minted {} to {}", amount, receiver)
    }

    pub fn get_balance(&self, account: AccountId) -> Balance {
        self.bank_module.get_balance(&account)
    }

    // Staking Module Functions
    pub fn add_validator(&mut self, validator: AccountId) -> String {
        self.staking_module.add_validator(&validator);
        format!("Added validator {}", validator)
    }

    pub fn delegate(&mut self, validator: AccountId, amount: Balance) -> String {
        let delegator = env::predecessor_account_id();
        self.staking_module.delegate(&delegator, &validator, amount, &mut self.bank_module);
        format!("Delegated {} to {} from {}", amount, validator, delegator)
    }

    pub fn undelegate(&mut self, validator: AccountId, amount: Balance) -> String {
        let delegator = env::predecessor_account_id();
        self.staking_module.undelegate(&delegator, &validator, amount, self.block_height);
        format!("Undelegated {} from {} by {}", amount, validator, delegator)
    }

    // Governance Module Functions
    pub fn submit_proposal(&mut self, title: String, description: String, param_key: String, param_value: String) -> u64 {
        let proposer = env::predecessor_account_id();
        let proposal_id = self.governance_module.submit_proposal(&proposer, title, description, param_key, param_value, self.block_height);
        env::log_str(&format!("Submitted proposal {} by {}", proposal_id, proposer));
        proposal_id
    }

    pub fn vote(&mut self, proposal_id: u64, option: u8) -> String {
        let voter = env::predecessor_account_id();
        self.governance_module.vote(&voter, proposal_id, option);
        format!("Voted {} on proposal {} by {}", option, proposal_id, voter)
    }

    pub fn get_parameter(&self, key: String) -> String {
        self.governance_module.get_parameter(&key)
    }

    // Block Processing
    pub fn process_block(&mut self) -> String {
        self.block_height += 1;
        
        // Begin block processing
        self.staking_module.begin_block(self.block_height);
        
        // End block processing
        self.staking_module.end_block(self.block_height, &mut self.bank_module);
        self.governance_module.end_block(self.block_height);
        
        format!("Processed block {}", self.block_height)
    }

    // View functions
    pub fn get_block_height(&self) -> u64 {
        self.block_height
    }
}