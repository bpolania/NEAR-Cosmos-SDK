package test

import (
	"cosmos_on_near/internal/bank"
	"testing"
)

func TestBalanceSerialization(t *testing.T) {
	balance := &bank.Balance{Amount: 12345}
	
	serialized := balance.Serialize()
	if len(serialized) == 0 {
		t.Error("Serialization failed - empty result")
	}
	
	deserialized := bank.DeserializeBalance(serialized)
	
	if deserialized.Amount != balance.Amount {
		t.Errorf("Expected %d, got %d", balance.Amount, deserialized.Amount)
	}
}

func TestEmptyBalanceDeserialization(t *testing.T) {
	deserialized := bank.DeserializeBalance([]byte{})
	
	if deserialized.Amount != 0 {
		t.Errorf("Expected 0 for empty data, got %d", deserialized.Amount)
	}
}