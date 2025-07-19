package staking

import (
	"github.com/vlmoon99/near-sdk-go/borsh"
)

type Validator struct {
	Address       string `borsh:"address"`
	DelegatedStake uint64 `borsh:"delegated_stake"`
	IsActive      bool   `borsh:"is_active"`
}

type Delegation struct {
	Delegator string `borsh:"delegator"`
	Validator string `borsh:"validator"`
	Amount    uint64 `borsh:"amount"`
}

type UnbondingEntry struct {
	Delegator    string `borsh:"delegator"`
	Validator    string `borsh:"validator"`
	Amount       uint64 `borsh:"amount"`
	UnlockHeight uint64 `borsh:"unlock_height"`
}

func (v *Validator) Serialize() []byte {
	data, _ := borsh.BorshSerialize(v)
	return data
}

func DeserializeValidator(data []byte) *Validator {
	if len(data) == 0 {
		return nil
	}
	
	var validator Validator
	borsh.BorshDeserialize(&validator, data)
	return &validator
}

func (d *Delegation) Serialize() []byte {
	data, _ := borsh.BorshSerialize(d)
	return data
}

func DeserializeDelegation(data []byte) *Delegation {
	if len(data) == 0 {
		return nil
	}
	
	var delegation Delegation
	borsh.BorshDeserialize(&delegation, data)
	return &delegation
}

func (u *UnbondingEntry) Serialize() []byte {
	data, _ := borsh.BorshSerialize(u)
	return data
}

func DeserializeUnbondingEntry(data []byte) *UnbondingEntry {
	if len(data) == 0 {
		return nil
	}
	
	var entry UnbondingEntry
	borsh.BorshDeserialize(&entry, data)
	return &entry
}