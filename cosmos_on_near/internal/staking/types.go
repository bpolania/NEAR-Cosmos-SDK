package staking

import (
	"encoding/binary"
)

type Validator struct {
	Address       string `json:"address"`
	DelegatedStake uint64 `json:"delegated_stake"`
	IsActive      bool   `json:"is_active"`
}

type Delegation struct {
	Delegator string `json:"delegator"`
	Validator string `json:"validator"`
	Amount    uint64 `json:"amount"`
}

type UnbondingEntry struct {
	Delegator    string `json:"delegator"`
	Validator    string `json:"validator"`
	Amount       uint64 `json:"amount"`
	UnlockHeight uint64 `json:"unlock_height"`
}

func (v *Validator) Serialize() []byte {
	// Simple binary serialization - delegated_stake (8 bytes) + is_active (1 byte)
	// Strings are omitted in this simple implementation
	data := make([]byte, 9)
	binary.LittleEndian.PutUint64(data[0:8], v.DelegatedStake)
	if v.IsActive {
		data[8] = 1
	} else {
		data[8] = 0
	}
	return data
}

func DeserializeValidator(data []byte) *Validator {
	if len(data) < 9 {
		return nil
	}
	
	validator := &Validator{
		DelegatedStake: binary.LittleEndian.Uint64(data[0:8]),
		IsActive:       data[8] == 1,
	}
	return validator
}

func (d *Delegation) Serialize() []byte {
	// Simple binary serialization - amount (8 bytes)
	// Strings are omitted in this simple implementation
	data := make([]byte, 8)
	binary.LittleEndian.PutUint64(data[0:8], d.Amount)
	return data
}

func DeserializeDelegation(data []byte) *Delegation {
	if len(data) < 8 {
		return nil
	}
	
	delegation := &Delegation{
		Amount: binary.LittleEndian.Uint64(data[0:8]),
	}
	return delegation
}

func (u *UnbondingEntry) Serialize() []byte {
	// Simple binary serialization - amount (8 bytes) + unlock_height (8 bytes)
	// Strings are omitted in this simple implementation
	data := make([]byte, 16)
	binary.LittleEndian.PutUint64(data[0:8], u.Amount)
	binary.LittleEndian.PutUint64(data[8:16], u.UnlockHeight)
	return data
}

func DeserializeUnbondingEntry(data []byte) *UnbondingEntry {
	if len(data) < 16 {
		return nil
	}
	
	entry := &UnbondingEntry{
		Amount:       binary.LittleEndian.Uint64(data[0:8]),
		UnlockHeight: binary.LittleEndian.Uint64(data[8:16]),
	}
	return entry
}