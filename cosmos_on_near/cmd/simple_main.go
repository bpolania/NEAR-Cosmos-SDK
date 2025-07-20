package main

import (
	"cosmos_on_near/internal/bank"
	"cosmos_on_near/internal/storage"
	"github.com/vlmoon99/near-sdk-go/contract"
)

type SimpleContract struct {
	store      storage.Store
	bankModule *bank.BankModule
}

func NewSimpleContract() *SimpleContract {
	store := storage.NewNearStore()
	bankModule := bank.NewBankModule(store)
	
	return &SimpleContract{
		store:      store,
		bankModule: bankModule,
	}
}

func (c *SimpleContract) Transfer(sender string, receiver string, amount uint64) {
	c.bankModule.Transfer(sender, receiver, amount)
}

func (c *SimpleContract) Mint(receiver string, amount uint64) {
	c.bankModule.Mint(receiver, amount)
}

func (c *SimpleContract) GetBalance(account string) uint64 {
	return c.bankModule.GetBalance(account)
}

func main() {
	contractObj := NewSimpleContract()

	contract.RegisterFunction("transfer", contractObj.Transfer)
	contract.RegisterFunction("mint", contractObj.Mint)
	contract.RegisterFunction("get_balance", contractObj.GetBalance)
}