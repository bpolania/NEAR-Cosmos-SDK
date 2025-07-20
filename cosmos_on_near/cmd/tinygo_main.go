package main

import (
	"cosmos_on_near/internal/bank"
	"cosmos_on_near/internal/governance"
	"cosmos_on_near/internal/near"
	"cosmos_on_near/internal/staking"
	"cosmos_on_near/internal/storage"
	"fmt"
)

type CosmosContract struct {
	store            storage.Store
	systemStore      *storage.ModuleStore
	bankModule       *bank.BankModule
	stakingModule    *staking.StakingModule
	governanceModule *governance.GovernanceModule
}

var contract *CosmosContract

func init() {
	store := storage.NewNearStore()
	systemStore := storage.NewModuleStore(store, "system")
	bankModule := bank.NewBankModule(store)
	stakingModule := staking.NewStakingModule(store, bankModule)
	governanceModule := governance.NewGovernanceModule(store)
	
	contract = &CosmosContract{
		store:            store,
		systemStore:      systemStore,
		bankModule:       bankModule,
		stakingModule:    stakingModule,
		governanceModule: governanceModule,
	}
}

// Modern NEAR contract entry points using //export pattern

//export transfer
func transfer() {
	// Parse JSON input for {sender, receiver, amount}
	var input struct {
		Sender   string `json:"sender"`
		Receiver string `json:"receiver"`
		Amount   uint64 `json:"amount"`
	}
	
	// TODO: Get input from NEAR runtime
	// For now, this is a placeholder structure
	contract.bankModule.Transfer(input.Sender, input.Receiver, input.Amount)
	
	result := fmt.Sprintf("Transferred %d from %s to %s", input.Amount, input.Sender, input.Receiver)
	near.ContractValueReturn([]byte(result))
}

//export mint
func mint() {
	var input struct {
		Receiver string `json:"receiver"`
		Amount   uint64 `json:"amount"`
	}
	
	contract.bankModule.Mint(input.Receiver, input.Amount)
	
	result := fmt.Sprintf("Minted %d to %s", input.Amount, input.Receiver)
	near.ContractValueReturn([]byte(result))
}

//export get_balance
func getBalance() {
	var input struct {
		Account string `json:"account"`
	}
	
	balance := contract.bankModule.GetBalance(input.Account)
	
	result := fmt.Sprintf("{\"balance\":%d}", balance)
	near.ContractValueReturn([]byte(result))
}

//export delegate
func delegate() {
	var input struct {
		Validator string `json:"validator"`
		Amount    uint64 `json:"amount"`
	}
	
	contract.stakingModule.Delegate(input.Validator, input.Amount)
	
	result := fmt.Sprintf("Delegated %d to %s", input.Amount, input.Validator)
	near.ContractValueReturn([]byte(result))
}

//export undelegate
func undelegate() {
	var input struct {
		Validator string `json:"validator"`
		Amount    uint64 `json:"amount"`
	}
	
	contract.stakingModule.Undelegate(input.Validator, input.Amount)
	
	result := fmt.Sprintf("Undelegated %d from %s", input.Amount, input.Validator)
	near.ContractValueReturn([]byte(result))
}

//export add_validator
func addValidator() {
	var input struct {
		Address string `json:"address"`
	}
	
	contract.stakingModule.AddValidator(input.Address)
	
	result := fmt.Sprintf("Added validator %s", input.Address)
	near.ContractValueReturn([]byte(result))
}

//export submit_proposal
func submitProposal() {
	var input struct {
		Title       string `json:"title"`
		Description string `json:"description"`
		ParamKey    string `json:"param_key"`
		ParamValue  string `json:"param_value"`
	}
	
	proposalID := contract.governanceModule.SubmitProposal(input.Title, input.Description, input.ParamKey, input.ParamValue)
	
	result := fmt.Sprintf("{\"proposal_id\":%d}", proposalID)
	near.ContractValueReturn([]byte(result))
}

//export vote
func vote() {
	var input struct {
		ProposalID uint64 `json:"proposal_id"`
		Option     uint8  `json:"option"`
	}
	
	contract.governanceModule.Vote(input.ProposalID, input.Option)
	
	result := fmt.Sprintf("Voted %d on proposal %d", input.Option, input.ProposalID)
	near.ContractValueReturn([]byte(result))
}

//export get_parameter
func getParameter() {
	var input struct {
		Key string `json:"key"`
	}
	
	value := contract.governanceModule.GetParameter(input.Key)
	
	result := fmt.Sprintf("{\"value\":\"%s\"}", value)
	near.ContractValueReturn([]byte(result))
}

//export process_block
func processBlock() {
	height := contract.systemStore.IncrementBlockHeight()
	
	contract.stakingModule.BeginBlock(height)
	contract.stakingModule.EndBlock(height)
	contract.governanceModule.EndBlock(height)
	
	result := fmt.Sprintf("Processed block %d", height)
	near.ContractValueReturn([]byte(result))
}

// Required main function (but won't be called in WASM)
func main() {
	// This is required for compilation but won't be used in WASM context
}