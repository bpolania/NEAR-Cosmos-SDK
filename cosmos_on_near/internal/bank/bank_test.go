package bank

import (
	"cosmos_on_near/internal/storage"
	"testing"
)

func TestBalanceSerializationRoundtrip(t *testing.T) {
	balance := &Balance{Amount: 12345}
	
	serialized := balance.Serialize()
	deserialized := DeserializeBalance(serialized)
	
	if deserialized.Amount != balance.Amount {
		t.Errorf("Expected %d, got %d", balance.Amount, deserialized.Amount)
	}
}

func TestEmptyBalanceDeserialization(t *testing.T) {
	deserialized := DeserializeBalance([]byte{})
	
	if deserialized.Amount != 0 {
		t.Errorf("Expected 0 for empty data, got %d", deserialized.Amount)
	}
}

func TestBankModuleBalance(t *testing.T) {
	mockStore := storage.NewMockStore()
	bankModule := NewBankModule(mockStore)
	
	account := "test.near"
	balance := bankModule.GetBalance(account)
	
	if balance != 0 {
		t.Errorf("Expected initial balance 0, got %d", balance)
	}
	
	bankModule.Mint(account, 100)
	balance = bankModule.GetBalance(account)
	
	if balance != 100 {
		t.Errorf("Expected balance 100 after mint, got %d", balance)
	}
}