use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};

pub type Balance = u128;

mod modules;

use modules::bank::BankModule;
use modules::gov::GovernanceModule;
use modules::staking::StakingModule;
use modules::ibc::client::tendermint::{TendermintLightClientModule, Header, Height};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CosmosContract {
    bank_module: BankModule,
    staking_module: StakingModule,
    governance_module: GovernanceModule,
    ibc_client_module: TendermintLightClientModule,
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
            ibc_client_module: TendermintLightClientModule::new(),
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

    // IBC Client Module Functions
    pub fn ibc_create_client(
        &mut self,
        chain_id: String,
        trust_period: u64,
        unbonding_period: u64,
        max_clock_drift: u64,
        initial_header: Header,
    ) -> String {
        self.ibc_client_module.create_client(chain_id, trust_period, unbonding_period, max_clock_drift, initial_header)
    }

    pub fn ibc_update_client(&mut self, client_id: String, header: Header) -> bool {
        self.ibc_client_module.update_client(client_id, header)
    }

    pub fn ibc_verify_membership(
        &self,
        client_id: String,
        height: u64,
        key: Vec<u8>,
        value: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_membership(client_id, height, key, value, proof)
    }

    pub fn ibc_verify_non_membership(
        &self,
        client_id: String,
        height: u64,
        key: Vec<u8>,
        proof: Vec<u8>,
    ) -> bool {
        self.ibc_client_module.verify_non_membership(client_id, height, key, proof)
    }

    pub fn ibc_get_client_state(&self, client_id: String) -> Option<modules::ibc::client::tendermint::ClientState> {
        self.ibc_client_module.get_client_state(client_id)
    }

    pub fn ibc_get_consensus_state(&self, client_id: String, height: u64) -> Option<modules::ibc::client::tendermint::ConsensusState> {
        self.ibc_client_module.get_consensus_state(client_id, height)
    }

    pub fn ibc_get_latest_height(&self, client_id: String) -> Option<Height> {
        self.ibc_client_module.get_latest_height(client_id)
    }

    pub fn ibc_prune_expired_consensus_state(&mut self, client_id: String, height: u64) -> bool {
        self.ibc_client_module.prune_expired_consensus_state(client_id, height)
    }

    // View functions
    pub fn get_block_height(&self) -> u64 {
        self.block_height
    }
}