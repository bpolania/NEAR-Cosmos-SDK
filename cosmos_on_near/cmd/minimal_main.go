package main

import (
	"cosmos_on_near/internal/bank"
	"cosmos_on_near/internal/storage"
	"github.com/vlmoon99/near-sdk-go/contract"
)

var bankModule *bank.BankModule

func handleInput(input *contract.ContractInput) error {
	// Simple contract handler - just initialize if not done
	if bankModule == nil {
		store := storage.NewNearStore()
		bankModule = bank.NewBankModule(store)
	}
	
	// For now, just mint 100 tokens to test account
	bankModule.Mint("test.testnet", 100)
	
	return nil
}

func main() {
	contract.HandleClientJSONInput(handleInput)
}