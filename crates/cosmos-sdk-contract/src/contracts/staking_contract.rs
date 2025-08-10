/// x/staking Module Contract
/// 
/// This contract handles all staking operations including:
/// - Validator registration and management
/// - Delegation and undelegation
/// - Reward distribution
/// - Slashing for misbehavior

use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use base64::{Engine as _, engine::general_purpose};

use crate::modules::staking::{StakingModule, Validator, Delegation, UnbondingDelegation};
use crate::Balance;

/// x/staking contract state
#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct StakingContract {
    /// The underlying staking module
    staking_module: StakingModule,
    /// Router contract that can call this module
    router_contract: Option<AccountId>,
    /// Contract owner for admin operations
    owner: AccountId,
}

/// Response from staking operations
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct StakingOperationResponse {
    pub success: bool,
    pub validator_address: Option<String>,
    pub delegator: Option<String>,
    pub amount: Option<Balance>,
    pub completion_time: Option<u64>,
    pub events: Vec<String>,
    pub error: Option<String>,
}

/// Validator creation request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct CreateValidatorRequest {
    pub validator_address: String,
    pub pubkey: String,
    pub moniker: String,
    pub identity: Option<String>,
    pub website: Option<String>,
    pub security_contact: Option<String>,
    pub details: Option<String>,
    pub commission_rate: String, // Decimal as string
    pub commission_max_rate: String,
    pub commission_max_change_rate: String,
    pub min_self_delegation: Balance,
    pub self_delegation: Balance,
}

/// Delegation request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct DelegateRequest {
    pub delegator: String,
    pub validator_address: String,
    pub amount: Balance,
}

/// Undelegation request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct UndelegateRequest {
    pub delegator: String,
    pub validator_address: String,
    pub amount: Balance,
}

/// Redelegation request
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct RedelegateRequest {
    pub delegator: String,
    pub validator_src_address: String,
    pub validator_dst_address: String,
    pub amount: Balance,
}

#[near_bindgen]
impl StakingContract {
    #[init]
    pub fn new(owner: AccountId, router_contract: Option<AccountId>) -> Self {
        Self {
            staking_module: StakingModule::new(),
            router_contract,
            owner,
        }
    }

    // =============================================================================
    // Validator Management Functions
    // =============================================================================

    /// Create a new validator
    pub fn create_validator(&mut self, request: CreateValidatorRequest) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        match self.staking_module.create_validator(
            request.validator_address.clone(),
            general_purpose::STANDARD.decode(&request.pubkey).unwrap_or_default(),
            request.moniker,
            request.identity,
            request.website,
            request.security_contact,
            request.details,
            request.commission_rate,
            request.commission_max_rate,
            request.commission_max_change_rate,
            request.min_self_delegation,
            request.self_delegation,
        ) {
            Ok(_) => {
                env::log_str(&format!("Created validator: {}", request.validator_address));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(request.validator_address),
                    delegator: None,
                    amount: Some(request.self_delegation),
                    completion_time: None,
                    events: vec!["create_validator".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Validator creation failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(request.validator_address),
                    delegator: None,
                    amount: Some(request.self_delegation),
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Edit validator information
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
    ) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        match self.staking_module.edit_validator(
            validator_address.clone(),
            moniker,
            identity,
            website,
            security_contact,
            details,
            commission_rate,
            min_self_delegation,
        ) {
            Ok(_) => {
                env::log_str(&format!("Updated validator: {}", validator_address));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(validator_address),
                    delegator: None,
                    amount: None,
                    completion_time: None,
                    events: vec!["edit_validator".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Validator update failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(validator_address),
                    delegator: None,
                    amount: None,
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    // =============================================================================
    // Delegation Functions
    // =============================================================================

    /// Delegate tokens to a validator
    pub fn delegate(&mut self, request: DelegateRequest) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        // Validate that the caller is either the delegator or authorized to act on their behalf
        let caller = env::predecessor_account_id();
        if caller != request.delegator && !self.is_router_or_owner(&caller) {
            return StakingOperationResponse {
                success: false,
                validator_address: Some(request.validator_address),
                delegator: Some(request.delegator),
                amount: Some(request.amount),
                completion_time: None,
                events: vec![],
                error: Some("Unauthorized: caller cannot delegate from this account".to_string()),
            };
        }

        match self.staking_module.delegate(
            request.delegator.to_string(),
            request.validator_address.clone(),
            request.amount,
        ) {
            Ok(_) => {
                env::log_str(&format!(
                    "Delegated {} from {} to validator {}", 
                    request.amount, 
                    request.delegator, 
                    request.validator_address
                ));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(request.validator_address),
                    delegator: Some(request.delegator),
                    amount: Some(request.amount),
                    completion_time: None,
                    events: vec!["delegate".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Delegation failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(request.validator_address),
                    delegator: Some(request.delegator),
                    amount: Some(request.amount),
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Begin unbonding delegation
    pub fn undelegate(&mut self, request: UndelegateRequest) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        // Validate that the caller is either the delegator or authorized
        let caller = env::predecessor_account_id();
        if caller != request.delegator && !self.is_router_or_owner(&caller) {
            return StakingOperationResponse {
                success: false,
                validator_address: Some(request.validator_address),
                delegator: Some(request.delegator),
                amount: Some(request.amount),
                completion_time: None,
                events: vec![],
                error: Some("Unauthorized: caller cannot undelegate from this account".to_string()),
            };
        }

        match self.staking_module.undelegate(
            request.delegator.to_string(),
            request.validator_address.clone(),
            request.amount,
        ) {
            Ok(completion_time) => {
                env::log_str(&format!(
                    "Started unbonding {} from {} to validator {}", 
                    request.amount, 
                    request.delegator, 
                    request.validator_address
                ));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(request.validator_address),
                    delegator: Some(request.delegator),
                    amount: Some(request.amount),
                    completion_time: Some(completion_time),
                    events: vec!["begin_unbonding".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Undelegation failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(request.validator_address),
                    delegator: Some(request.delegator),
                    amount: Some(request.amount),
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Begin redelegation from one validator to another
    pub fn redelegate(&mut self, request: RedelegateRequest) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        // Validate that the caller is either the delegator or authorized
        let caller = env::predecessor_account_id();
        if caller != request.delegator && !self.is_router_or_owner(&caller) {
            return StakingOperationResponse {
                success: false,
                validator_address: Some(request.validator_src_address.clone()),
                delegator: Some(request.delegator),
                amount: Some(request.amount),
                completion_time: None,
                events: vec![],
                error: Some("Unauthorized: caller cannot redelegate from this account".to_string()),
            };
        }

        match self.staking_module.redelegate(
            request.delegator.to_string(),
            request.validator_src_address.clone(),
            request.validator_dst_address.clone(),
            request.amount,
        ) {
            Ok(completion_time) => {
                env::log_str(&format!(
                    "Started redelegation {} from {} to {} by {}", 
                    request.amount,
                    request.validator_src_address,
                    request.validator_dst_address,
                    request.delegator
                ));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(request.validator_dst_address),
                    delegator: Some(request.delegator),
                    amount: Some(request.amount),
                    completion_time: Some(completion_time),
                    events: vec!["begin_redelegate".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Redelegation failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(request.validator_src_address),
                    delegator: Some(request.delegator),
                    amount: Some(request.amount),
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    // =============================================================================
    // Query Functions
    // =============================================================================

    /// Get validator information
    pub fn get_validator(&self, validator_address: String) -> Option<Validator> {
        self.assert_authorized_caller();
        self.staking_module.get_validator(validator_address)
    }

    /// Get all validators
    pub fn get_all_validators(&self) -> Vec<Validator> {
        self.assert_authorized_caller();
        self.staking_module.get_all_validators()
    }

    /// Get bonded validators (active set)
    pub fn get_bonded_validators(&self) -> Vec<Validator> {
        self.assert_authorized_caller();
        self.staking_module.get_bonded_validators()
    }

    /// Get delegation
    pub fn get_delegation(&self, delegator: AccountId, validator_address: String) -> Option<Delegation> {
        self.assert_authorized_caller();
        self.staking_module.get_delegation(delegator.to_string(), validator_address)
    }

    /// Get all delegations for a delegator
    pub fn get_delegations(&self, delegator: AccountId) -> Vec<Delegation> {
        self.assert_authorized_caller();
        self.staking_module.get_delegations(delegator.to_string())
    }

    /// Get all delegations to a validator
    pub fn get_validator_delegations(&self, validator_address: String) -> Vec<Delegation> {
        self.assert_authorized_caller();
        self.staking_module.get_validator_delegations(validator_address)
    }

    /// Get unbonding delegation
    pub fn get_unbonding_delegation(
        &self, 
        delegator: AccountId, 
        validator_address: String
    ) -> Option<UnbondingDelegation> {
        self.assert_authorized_caller();
        self.staking_module.get_unbonding_delegation(delegator.to_string(), validator_address)
    }

    /// Get all unbonding delegations for a delegator
    pub fn get_unbonding_delegations(&self, delegator: AccountId) -> Vec<UnbondingDelegation> {
        self.assert_authorized_caller();
        self.staking_module.get_unbonding_delegations(delegator.to_string())
    }

    /// Get staking pool information
    pub fn get_pool(&self) -> serde_json::Value {
        self.assert_authorized_caller();
        let pool = self.staking_module.get_pool();
        serde_json::json!({
            "not_bonded_tokens": pool.not_bonded_tokens,
            "bonded_tokens": pool.bonded_tokens
        })
    }

    /// Get staking parameters
    pub fn get_params(&self) -> serde_json::Value {
        self.assert_authorized_caller();
        let params = self.staking_module.get_params();
        serde_json::json!({
            "unbonding_time": params.unbonding_time,
            "max_validators": params.max_validators,
            "max_entries": params.max_entries,
            "historical_entries": params.historical_entries,
            "bond_denom": params.bond_denom,
            "min_commission_rate": params.min_commission_rate
        })
    }

    // =============================================================================
    // Reward and Slashing Functions
    // =============================================================================

    /// Withdraw delegation rewards
    pub fn withdraw_delegator_reward(
        &mut self, 
        delegator: AccountId, 
        validator_address: String
    ) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        // Validate that the caller is either the delegator or authorized
        let caller = env::predecessor_account_id();
        if caller != delegator && !self.is_router_or_owner(&caller) {
            return StakingOperationResponse {
                success: false,
                validator_address: Some(validator_address),
                delegator: Some(delegator.to_string()),
                amount: None,
                completion_time: None,
                events: vec![],
                error: Some("Unauthorized: caller cannot withdraw rewards from this account".to_string()),
            };
        }

        match self.staking_module.withdraw_delegator_reward(
            delegator.to_string(), 
            validator_address.clone()
        ) {
            Ok(amount) => {
                env::log_str(&format!(
                    "Withdrew {} rewards for {} from validator {}", 
                    amount, 
                    delegator, 
                    validator_address
                ));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(validator_address),
                    delegator: Some(delegator.to_string()),
                    amount: Some(amount),
                    completion_time: None,
                    events: vec!["withdraw_rewards".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Reward withdrawal failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(validator_address),
                    delegator: Some(delegator.to_string()),
                    amount: None,
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    /// Slash a validator for misbehavior (owner only)
    pub fn slash_validator(
        &mut self, 
        validator_address: String, 
        height: u64, 
        power: u64, 
        slash_fraction: String
    ) -> StakingOperationResponse {
        self.assert_owner();
        
        match self.staking_module.slash_validator(
            validator_address.clone(), 
            height, 
            power, 
            slash_fraction
        ) {
            Ok(slashed_amount) => {
                env::log_str(&format!(
                    "Slashed validator {} for {} tokens at height {}", 
                    validator_address, 
                    slashed_amount, 
                    height
                ));
                StakingOperationResponse {
                    success: true,
                    validator_address: Some(validator_address),
                    delegator: None,
                    amount: Some(slashed_amount),
                    completion_time: None,
                    events: vec!["slash_validator".to_string()],
                    error: None,
                }
            }
            Err(e) => {
                env::log_str(&format!("Validator slashing failed: {:?}", e));
                StakingOperationResponse {
                    success: false,
                    validator_address: Some(validator_address),
                    delegator: None,
                    amount: None,
                    completion_time: None,
                    events: vec![],
                    error: Some(format!("{:?}", e)),
                }
            }
        }
    }

    // =============================================================================
    // Cross-module Integration Functions
    // =============================================================================

    /// Process staking operation with bank transfer (called by router)
    pub fn process_staking_operation(&mut self, operation_data: Base64VecU8) -> StakingOperationResponse {
        self.assert_authorized_caller();
        
        // Decode and process staking request
        if let Ok(delegate_request) = serde_json::from_slice::<DelegateRequest>(&operation_data.0) {
            self.delegate(delegate_request)
        } else if let Ok(undelegate_request) = serde_json::from_slice::<UndelegateRequest>(&operation_data.0) {
            self.undelegate(undelegate_request)
        } else {
            StakingOperationResponse {
                success: false,
                validator_address: None,
                delegator: None,
                amount: None,
                completion_time: None,
                events: vec![],
                error: Some("Invalid staking operation data format".to_string()),
            }
        }
    }

    /// Validate staking operation (for pre-validation)
    pub fn validate_staking_operation(&self, validator_address: String, amount: Balance) -> bool {
        self.assert_authorized_caller();
        
        // Check if validator exists and is active
        if let Some(validator) = self.staking_module.get_validator(validator_address) {
            validator.status == crate::modules::staking::ValidatorStatus::Bonded && amount > 0
        } else {
            false
        }
    }

    // =============================================================================
    // Admin and Configuration Functions
    // =============================================================================

    /// Update the router contract address
    pub fn update_router_contract(&mut self, new_router: AccountId) {
        self.assert_owner();
        self.router_contract = Some(new_router.clone());
        env::log_str(&format!("Updated router contract to: {}", new_router));
    }

    /// Get current router contract
    pub fn get_router_contract(&self) -> Option<AccountId> {
        self.router_contract.clone()
    }

    /// Health check for the staking module
    pub fn health_check(&self) -> bool {
        // Check if the staking module is functioning
        true // In a full implementation, would perform actual health checks
    }

    /// Get module metadata
    pub fn get_metadata(&self) -> serde_json::Value {
        serde_json::json!({
            "module_type": "staking",
            "version": "1.0.0",
            "description": "x/staking Module for delegation and validation",
            "functions": [
                "create_validator",
                "edit_validator",
                "delegate",
                "undelegate",
                "redelegate",
                "get_validator",
                "get_all_validators",
                "get_bonded_validators",
                "get_delegation",
                "get_delegations",
                "get_validator_delegations",
                "get_unbonding_delegation",
                "get_unbonding_delegations",
                "get_pool",
                "get_params",
                "withdraw_delegator_reward",
                "slash_validator",
                "process_staking_operation",
                "validate_staking_operation"
            ],
            "total_validators": self.staking_module.get_all_validators().len()
        })
    }

    // =============================================================================
    // Helper Functions
    // =============================================================================

    /// Check if caller is router or owner
    fn is_router_or_owner(&self, caller: &AccountId) -> bool {
        caller == &self.owner || 
        self.router_contract.as_ref().map_or(false, |router| caller == router)
    }

    /// Assert that the caller is authorized (owner or router)
    fn assert_authorized_caller(&self) {
        let caller = env::predecessor_account_id();
        
        let is_owner = caller == self.owner;
        let is_router = self.router_contract.as_ref().map_or(false, |router| caller == *router);
        
        assert!(
            is_owner || is_router,
            "Unauthorized: only owner or router can call this function"
        );
    }

    /// Assert that the caller is the contract owner
    fn assert_owner(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.owner,
            "Only owner can perform this action"
        );
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        self.assert_owner();
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        env::log_str(&format!("Ownership transferred from {} to {}", old_owner, new_owner));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, VMContext};

    fn get_context(predecessor: AccountId) -> VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(predecessor)
            .build()
    }

    #[test]
    fn test_staking_contract_initialization() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = StakingContract::new(accounts(1), Some(accounts(2)));
        assert_eq!(contract.owner, accounts(1));
        assert_eq!(contract.router_contract, Some(accounts(2)));
    }

    #[test]
    fn test_authorized_caller_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = StakingContract::new(accounts(1), Some(accounts(2)));
        
        // Owner should be authorized
        contract.assert_authorized_caller();
        
        // Test router access
        let router_context = get_context(accounts(2));
        testing_env!(router_context);
        contract.assert_authorized_caller();
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_unauthorized_caller() {
        let context = get_context(accounts(3)); // Unauthorized account
        testing_env!(context);
        
        let contract = StakingContract::new(accounts(1), Some(accounts(2)));
        contract.assert_authorized_caller();
    }

    #[test]
    fn test_health_check() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = StakingContract::new(accounts(1), None);
        assert!(contract.health_check());
    }

    #[test]
    fn test_validation() {
        let context = get_context(accounts(1));
        testing_env!(context);
        
        let contract = StakingContract::new(accounts(1), None);
        
        // Test with non-existent validator
        let result = contract.validate_staking_operation("non-existent".to_string(), 1000);
        assert!(!result);
    }
}