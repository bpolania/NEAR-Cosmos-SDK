package bank

import (
	"github.com/vlmoon99/near-sdk-go/borsh"
)

type Balance struct {
	Amount uint64 `borsh:"amount"`
}

func (b *Balance) Serialize() []byte {
	data, _ := borsh.BorshSerialize(b)
	return data
}

func DeserializeBalance(data []byte) *Balance {
	if len(data) == 0 {
		return &Balance{Amount: 0}
	}
	
	var balance Balance
	borsh.BorshDeserialize(&balance, data)
	return &balance
}