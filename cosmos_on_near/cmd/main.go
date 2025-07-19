package main

import (
	"cosmos_on_near/internal/bank"
	"cosmos_on_near/internal/governance"
	"cosmos_on_near/internal/staking"
	"cosmos_on_near/internal/storage"
	"github.com/vlmoon99/near-sdk-go/contract"
)

type CosmosContract struct {
	store            storage.Store
	systemStore      *storage.ModuleStore
	bankModule       *bank.BankModule
	stakingModule    *staking.StakingModule
	governanceModule *governance.GovernanceModule
}

func NewCosmosContract() *CosmosContract {
	store := storage.NewNearStore()
	systemStore := storage.NewModuleStore(store, "system")
	bankModule := bank.NewBankModule(store)
	stakingModule := staking.NewStakingModule(store, bankModule)
	governanceModule := governance.NewGovernanceModule(store)
	
	return &CosmosContract{
		store:            store,
		systemStore:      systemStore,
		bankModule:       bankModule,
		stakingModule:    stakingModule,
		governanceModule: governanceModule,
	}
}

// Bank module functions
func (c *CosmosContract) Transfer(receiver string, amount uint64) {
	c.bankModule.Transfer(receiver, amount)
}

func (c *CosmosContract) Mint(receiver string, amount uint64) {
	c.bankModule.Mint(receiver, amount)
}

func (c *CosmosContract) GetBalance(account string) uint64 {
	return c.bankModule.GetBalance(account)
}

// Staking module functions
func (c *CosmosContract) Delegate(validator string, amount uint64) {
	c.stakingModule.Delegate(validator, amount)
}

func (c *CosmosContract) Undelegate(validator string, amount uint64) {
	c.stakingModule.Undelegate(validator, amount)
}

func (c *CosmosContract) AddValidator(address string) {
	c.stakingModule.AddValidator(address)
}

// Governance module functions
func (c *CosmosContract) SubmitProposal(title, description, paramKey, paramValue string) uint64 {
	return c.governanceModule.SubmitProposal(title, description, paramKey, paramValue)
}

func (c *CosmosContract) Vote(proposalID uint64, option uint8) {
	c.governanceModule.Vote(proposalID, option)
}

func (c *CosmosContract) GetParameter(key string) string {
	return c.governanceModule.GetParameter(key)
}

// Block processing (called by cron)
func (c *CosmosContract) ProcessBlock() {
	height := c.systemStore.IncrementBlockHeight()
	
	c.stakingModule.BeginBlock(height)
	c.stakingModule.EndBlock(height)
	c.governanceModule.EndBlock(height)
}

func main() {
	contractObj := NewCosmosContract()

	// Bank functions
	contract.RegisterFunction("transfer", contractObj.Transfer)
	contract.RegisterFunction("mint", contractObj.Mint)
	contract.RegisterFunction("get_balance", contractObj.GetBalance)
	
	// Staking functions
	contract.RegisterFunction("delegate", contractObj.Delegate)
	contract.RegisterFunction("undelegate", contractObj.Undelegate)
	contract.RegisterFunction("add_validator", contractObj.AddValidator)
	
	// Governance functions
	contract.RegisterFunction("submit_proposal", contractObj.SubmitProposal)
	contract.RegisterFunction("vote", contractObj.Vote)
	contract.RegisterFunction("get_parameter", contractObj.GetParameter)
	
	// Block processing
	contract.RegisterFunction("process_block", contractObj.ProcessBlock)
}