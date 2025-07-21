use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, AccountId};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Proposal {
    pub id: u64,
    pub proposer: AccountId,
    pub title: String,
    pub description: String,
    pub param_key: String,
    pub param_value: String,
    pub start_height: u64,
    pub end_height: u64,
    pub yes_votes: u32,
    pub no_votes: u32,
    pub status: ProposalStatus,
}

#[derive(BorshDeserialize, BorshSerialize, PartialEq, Debug)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Vote {
    pub proposal_id: u64,
    pub voter: AccountId,
    pub option: u8, // 0 = No, 1 = Yes
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct GovernanceModule {
    proposals: UnorderedMap<u64, Proposal>,
    votes: UnorderedMap<String, Vote>, // key: "proposal_id:voter"
    parameters: UnorderedMap<String, String>,
    next_proposal_id: u64,
}

impl GovernanceModule {
    pub fn new() -> Self {
        let mut module = Self {
            proposals: UnorderedMap::new(b"p".to_vec()),
            votes: UnorderedMap::new(b"vo".to_vec()),
            parameters: UnorderedMap::new(b"pa".to_vec()),
            next_proposal_id: 1,
        };
        
        // Initialize default parameters
        module.parameters.insert(&"reward_rate".to_string(), &"5".to_string());
        module.parameters.insert(&"min_validator_stake".to_string(), &"100".to_string());
        module.parameters.insert(&"voting_period".to_string(), &"50".to_string());
        
        module
    }

    pub fn submit_proposal(
        &mut self,
        proposer: &AccountId,
        title: String,
        description: String,
        param_key: String,
        param_value: String,
        current_height: u64,
    ) -> u64 {
        let voting_period: u64 = self.parameters.get(&"voting_period".to_string())
            .unwrap_or("50".to_string())
            .parse()
            .unwrap_or(50);

        let proposal = Proposal {
            id: self.next_proposal_id,
            proposer: proposer.clone(),
            title,
            description,
            param_key,
            param_value,
            start_height: current_height,
            end_height: current_height + voting_period,
            yes_votes: 0,
            no_votes: 0,
            status: ProposalStatus::Active,
        };

        self.proposals.insert(&self.next_proposal_id, &proposal);
        
        env::log_str(&format!("Governance: Submitted proposal {} by {}", 
            self.next_proposal_id, proposer));
        
        let proposal_id = self.next_proposal_id;
        self.next_proposal_id += 1;
        proposal_id
    }

    pub fn vote(&mut self, voter: &AccountId, proposal_id: u64, option: u8) {
        let mut proposal = self.proposals.get(&proposal_id)
            .expect("Proposal not found");
        
        assert_eq!(proposal.status, ProposalStatus::Active, "Proposal not active");
        
        let vote_key = format!("{}:{}", proposal_id, voter);
        
        // Check if voter already voted
        if self.votes.get(&vote_key).is_some() {
            env::panic_str("Already voted on this proposal");
        }
        
        // Record vote
        let vote = Vote {
            proposal_id,
            voter: voter.clone(),
            option,
        };
        self.votes.insert(&vote_key, &vote);
        
        // Update proposal vote counts
        if option == 1 {
            proposal.yes_votes += 1;
        } else {
            proposal.no_votes += 1;
        }
        
        self.proposals.insert(&proposal_id, &proposal);
        
        env::log_str(&format!("Governance: Vote {} on proposal {} by {}", 
            option, proposal_id, voter));
    }

    pub fn get_parameter(&self, key: &String) -> String {
        self.parameters.get(key).unwrap_or("".to_string())
    }

    pub fn end_block(&mut self, current_height: u64) {
        let mut proposals_to_update = Vec::new();
        
        for (proposal_id, proposal) in self.proposals.iter() {
            if proposal.status == ProposalStatus::Active && current_height >= proposal.end_height {
                proposals_to_update.push((proposal_id, proposal));
            }
        }
        
        for (proposal_id, mut proposal) in proposals_to_update {
            let total_votes = proposal.yes_votes + proposal.no_votes;
            let quorum_threshold = 2; // 50% quorum (simplified)
            
            if total_votes >= quorum_threshold && proposal.yes_votes > proposal.no_votes {
                // Proposal passed
                proposal.status = ProposalStatus::Passed;
                
                // Apply parameter change
                self.parameters.insert(&proposal.param_key, &proposal.param_value);
                
                env::log_str(&format!("Governance: Proposal {} PASSED - {} = {}", 
                    proposal_id, proposal.param_key, proposal.param_value));
            } else {
                // Proposal rejected
                proposal.status = ProposalStatus::Rejected;
                
                env::log_str(&format!("Governance: Proposal {} REJECTED", proposal_id));
            }
            
            self.proposals.insert(&proposal_id, &proposal);
        }
    }
}