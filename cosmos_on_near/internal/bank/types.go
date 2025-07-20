package bank

import (
	"encoding/binary"
)

type Balance struct {
	Amount uint64 `json:"amount"`
}

func (b *Balance) Serialize() []byte {
	buf := make([]byte, 8)
	binary.LittleEndian.PutUint64(buf, b.Amount)
	return buf
}

func DeserializeBalance(data []byte) *Balance {
	if len(data) == 0 {
		return &Balance{Amount: 0}
	}
	
	if len(data) < 8 {
		return &Balance{Amount: 0}
	}
	
	amount := binary.LittleEndian.Uint64(data[:8])
	return &Balance{Amount: amount}
}