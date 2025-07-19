package bank

import (
	"cosmos_on_near/internal/storage"
	"github.com/vlmoon99/near-sdk-go/env"
)

type BankModule struct {
	store *storage.ModuleStore
}

func NewBankModule(store storage.Store) *BankModule {
	return &BankModule{
		store: storage.NewModuleStore(store, "bank"),
	}
}

func (bm *BankModule) getBalance(account string) *Balance {
	key := []byte("balance:" + account)
	data := bm.store.Get(key)
	return DeserializeBalance(data)
}

func (bm *BankModule) setBalance(account string, balance *Balance) {
	key := []byte("balance:" + account)
	bm.store.Set(key, balance.Serialize())
}

func (bm *BankModule) Transfer(receiver string, amount uint64) {
	sender := env.PredecessorAccountId()
	
	senderBalance := bm.getBalance(sender)
	if senderBalance.Amount < amount {
		env.Panic("Insufficient balance")
	}
	
	receiverBalance := bm.getBalance(receiver)
	
	senderBalance.Amount -= amount
	receiverBalance.Amount += amount
	
	bm.setBalance(sender, senderBalance)
	bm.setBalance(receiver, receiverBalance)
	
	env.Log("Transfer: " + sender + " -> " + receiver + " amount: " + string(rune(amount)))
}

func (bm *BankModule) Mint(receiver string, amount uint64) {
	receiverBalance := bm.getBalance(receiver)
	receiverBalance.Amount += amount
	
	bm.setBalance(receiver, receiverBalance)
	
	env.Log("Mint: " + receiver + " amount: " + string(rune(amount)))
}

func (bm *BankModule) GetBalance(account string) uint64 {
	return bm.getBalance(account).Amount
}