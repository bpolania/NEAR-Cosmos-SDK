#!/usr/bin/env node

// API Design Validation for Cosmos-on-NEAR
// This script simulates our contract calls to validate the API design

const fs = require('fs');

// Simulate our contract state
class CosmosContract {
    constructor() {
        this.balances = new Map();
        this.validators = new Map();
        this.proposals = new Map();
        this.blockHeight = 0;
        this.parameters = new Map();
        this.delegations = new Map();
        this.unbondingQueue = new Map();
        this.nextProposalId = 1;
    }

    // Bank Module
    transfer(sender, receiver, amount) {
        const senderBalance = this.balances.get(sender) || 0;
        if (senderBalance < amount) {
            throw new Error("Insufficient balance");
        }
        
        const receiverBalance = this.balances.get(receiver) || 0;
        this.balances.set(sender, senderBalance - amount);
        this.balances.set(receiver, receiverBalance + amount);
        
        console.log(`✅ Transfer: ${sender} -> ${receiver} amount: ${amount}`);
        return { success: true };
    }

    mint(receiver, amount) {
        const currentBalance = this.balances.get(receiver) || 0;
        this.balances.set(receiver, currentBalance + amount);
        
        console.log(`✅ Mint: ${receiver} amount: ${amount}`);
        return { success: true };
    }

    get_balance(account) {
        const balance = this.balances.get(account) || 0;
        console.log(`📊 Balance for ${account}: ${balance}`);
        return balance;
    }

    // Staking Module
    add_validator(address) {
        this.validators.set(address, {
            delegated_stake: 0,
            is_active: true
        });
        
        console.log(`✅ Added validator: ${address}`);
        return { success: true };
    }

    delegate(delegator, validator, amount) {
        if (!this.validators.has(validator)) {
            throw new Error("Validator not found");
        }
        
        // Transfer tokens to staking pool
        this.transfer(delegator, "staking_pool.testnet", amount);
        
        // Update delegation
        const delegationKey = `${delegator}:${validator}`;
        const currentDelegation = this.delegations.get(delegationKey) || 0;
        this.delegations.set(delegationKey, currentDelegation + amount);
        
        // Update validator stake
        const validatorInfo = this.validators.get(validator);
        validatorInfo.delegated_stake += amount;
        this.validators.set(validator, validatorInfo);
        
        console.log(`✅ Delegated: ${delegator} -> ${validator} amount: ${amount}`);
        return { success: true };
    }

    undelegate(delegator, validator, amount) {
        const delegationKey = `${delegator}:${validator}`;
        const currentDelegation = this.delegations.get(delegationKey) || 0;
        
        if (currentDelegation < amount) {
            throw new Error("Insufficient delegation");
        }
        
        // Update delegation
        this.delegations.set(delegationKey, currentDelegation - amount);
        
        // Update validator stake
        const validatorInfo = this.validators.get(validator);
        validatorInfo.delegated_stake -= amount;
        this.validators.set(validator, validatorInfo);
        
        // Add to unbonding queue (100 blocks from now)
        const unlockHeight = this.blockHeight + 100;
        const unbondingKey = `${unlockHeight}:${delegator}:${validator}`;
        this.unbondingQueue.set(unbondingKey, amount);
        
        console.log(`✅ Undelegated: ${delegator} from ${validator} amount: ${amount}, unlock at block ${unlockHeight}`);
        return { success: true };
    }

    // Governance Module
    submit_proposal(title, description, param_key, param_value) {
        const proposalId = this.nextProposalId++;
        const endBlock = this.blockHeight + 50; // 50 block voting period
        
        this.proposals.set(proposalId, {
            title,
            description,
            param_key,
            param_value,
            end_block: endBlock,
            yes_votes: 0,
            no_votes: 0,
            status: 'active'
        });
        
        console.log(`✅ Proposal submitted: ID ${proposalId} - ${title}, voting ends at block ${endBlock}`);
        return proposalId;
    }

    vote(proposal_id, option) {
        const proposal = this.proposals.get(proposal_id);
        if (!proposal) {
            throw new Error("Proposal not found");
        }
        
        if (this.blockHeight > proposal.end_block) {
            throw new Error("Voting period has ended");
        }
        
        if (option === 1) {
            proposal.yes_votes++;
        } else {
            proposal.no_votes++;
        }
        
        this.proposals.set(proposal_id, proposal);
        
        console.log(`✅ Vote cast on proposal ${proposal_id}: ${option === 1 ? 'YES' : 'NO'}`);
        return { success: true };
    }

    get_parameter(key) {
        const value = this.parameters.get(key);
        console.log(`📊 Parameter ${key}: ${value || 'not set'}`);
        return value;
    }

    // Block Processing
    process_block() {
        this.blockHeight++;
        
        // Process unbonding queue
        for (const [key, amount] of this.unbondingQueue.entries()) {
            const [unlockHeight, delegator, validator] = key.split(':');
            if (parseInt(unlockHeight) <= this.blockHeight) {
                this.transfer("staking_pool.testnet", delegator, amount);
                this.unbondingQueue.delete(key);
                console.log(`🔓 Released unbonding: ${delegator} amount: ${amount}`);
            }
        }
        
        // Process proposals
        for (const [proposalId, proposal] of this.proposals.entries()) {
            if (proposal.status === 'active' && this.blockHeight > proposal.end_block) {
                const totalVotes = proposal.yes_votes + proposal.no_votes;
                if (totalVotes > 0 && (proposal.yes_votes / totalVotes) >= 0.5) {
                    proposal.status = 'passed';
                    this.parameters.set(proposal.param_key, proposal.param_value);
                    console.log(`🗳️ Proposal ${proposalId} PASSED: ${proposal.param_key} = ${proposal.param_value}`);
                } else {
                    proposal.status = 'rejected';
                    console.log(`❌ Proposal ${proposalId} REJECTED`);
                }
                this.proposals.set(proposalId, proposal);
            }
        }
        
        // Distribute rewards (5% of total staked)
        let totalStaked = 0;
        for (const [validator, info] of this.validators.entries()) {
            totalStaked += info.delegated_stake;
        }
        
        if (totalStaked > 0) {
            const rewards = Math.floor(totalStaked * 0.05);
            this.mint("staking_pool.testnet", rewards);
            console.log(`💰 Distributed rewards: ${rewards} (5% of ${totalStaked} staked)`);
        }
        
        console.log(`⏰ Processed block ${this.blockHeight}`);
        return { block_height: this.blockHeight };
    }

    get_block_height() {
        return this.blockHeight;
    }
}

// Test Suite
async function runAPITests() {
    console.log("🧪 Starting Cosmos-on-NEAR API Validation Tests\n");
    
    const contract = new CosmosContract();
    
    try {
        // Test 1: Bank Module
        console.log("=== 🏦 Testing Bank Module ===");
        contract.mint("alice.testnet", 1000);
        contract.mint("bob.testnet", 500);
        contract.get_balance("alice.testnet");
        contract.transfer("alice.testnet", "bob.testnet", 300);
        contract.get_balance("alice.testnet"); // Should be 700
        contract.get_balance("bob.testnet");   // Should be 800
        console.log("");

        // Test 2: Staking Module
        console.log("=== 🥩 Testing Staking Module ===");
        contract.add_validator("validator1.testnet");
        contract.add_validator("validator2.testnet");
        contract.mint("alice.testnet", 300); // Restore Alice's balance for delegation
        contract.delegate("alice.testnet", "validator1.testnet", 200);
        contract.undelegate("alice.testnet", "validator1.testnet", 50);
        console.log("");

        // Test 3: Governance Module
        console.log("=== 🗳️ Testing Governance Module ===");
        const proposalId = contract.submit_proposal(
            "Update Reward Rate",
            "Increase staking rewards to 10%",
            "reward_rate",
            "10"
        );
        contract.vote(proposalId, 1); // Vote YES
        contract.get_parameter("reward_rate"); // Should be empty
        console.log("");

        // Test 4: Block Processing
        console.log("=== ⏰ Testing Block Processing ===");
        
        // Process enough blocks to complete proposal voting
        console.log("Processing 55 blocks to complete voting period and unbonding...");
        for (let i = 0; i < 55; i++) {
            const result = contract.process_block();
            if (i % 10 === 0) {
                console.log(`Block ${result.block_height} processed`);
            }
        }
        
        contract.get_parameter("reward_rate"); // Should now be "10"
        console.log("");

        // Test 5: Complex Scenario
        console.log("=== 🎯 Testing Complex Scenario ===");
        contract.mint("charlie.testnet", 2000);
        contract.delegate("charlie.testnet", "validator2.testnet", 1000);
        
        const proposal2 = contract.submit_proposal(
            "Add New Validator Requirement",
            "Require 1000 tokens minimum stake",
            "min_validator_stake",
            "1000"
        );
        contract.vote(proposal2, 0); // Vote NO
        
        // Process a few more blocks
        for (let i = 0; i < 60; i++) {
            contract.process_block();
        }
        
        contract.get_parameter("min_validator_stake"); // Should be empty (proposal rejected)
        console.log("");

        console.log("🎉 All API tests completed successfully!");
        
        // Summary
        console.log("\n=== 📊 Final State Summary ===");
        console.log(`Block Height: ${contract.get_block_height()}`);
        console.log("Balances:");
        console.log(`  alice.testnet: ${contract.get_balance("alice.testnet")}`);
        console.log(`  bob.testnet: ${contract.get_balance("bob.testnet")}`);
        console.log(`  charlie.testnet: ${contract.get_balance("charlie.testnet")}`);
        console.log(`  staking_pool.testnet: ${contract.get_balance("staking_pool.testnet")}`);
        
        console.log("\nParameters:");
        contract.get_parameter("reward_rate");
        contract.get_parameter("min_validator_stake");
        
    } catch (error) {
        console.error("❌ Test failed:", error.message);
        process.exit(1);
    }
}

// Run the tests
if (require.main === module) {
    runAPITests();
}

module.exports = { CosmosContract };