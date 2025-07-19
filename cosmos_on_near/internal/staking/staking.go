package staking

import (
	"cosmos_on_near/internal/bank"
	"cosmos_on_near/internal/storage"
	"encoding/binary"
	"github.com/vlmoon99/near-sdk-go/env"
	"strconv"
)

const (
	UnbondingPeriod    = 100 // blocks
	RewardPercentage   = 5   // 5% per reward cycle
)

type StakingModule struct {
	store      *storage.ModuleStore
	bankModule *bank.BankModule
}

func NewStakingModule(store storage.Store, bankModule *bank.BankModule) *StakingModule {
	return &StakingModule{
		store:      storage.NewModuleStore(store, "staking"),
		bankModule: bankModule,
	}
}

func (sm *StakingModule) getValidator(address string) *Validator {
	key := []byte("validator:" + address)
	data := sm.store.Get(key)
	return DeserializeValidator(data)
}

func (sm *StakingModule) setValidator(validator *Validator) {
	key := []byte("validator:" + validator.Address)
	sm.store.Set(key, validator.Serialize())
}

func (sm *StakingModule) getDelegation(delegator, validator string) *Delegation {
	key := []byte("delegation:" + delegator + ":" + validator)
	data := sm.store.Get(key)
	return DeserializeDelegation(data)
}

func (sm *StakingModule) setDelegation(delegation *Delegation) {
	key := []byte("delegation:" + delegation.Delegator + ":" + delegation.Validator)
	sm.store.Set(key, delegation.Serialize())
}

func (sm *StakingModule) addUnbondingEntry(entry *UnbondingEntry) {
	key := []byte("unbonding:" + strconv.FormatUint(entry.UnlockHeight, 10) + ":" + entry.Delegator + ":" + entry.Validator)
	sm.store.Set(key, entry.Serialize())
}

func (sm *StakingModule) Delegate(validatorAddr string, amount uint64) {
	delegator := env.PredecessorAccountId()
	
	validator := sm.getValidator(validatorAddr)
	if validator == nil {
		env.Panic("Validator not found")
	}
	
	if !validator.IsActive {
		env.Panic("Validator is not active")
	}
	
	sm.bankModule.Transfer("staking_pool", amount)
	
	delegation := sm.getDelegation(delegator, validatorAddr)
	if delegation == nil {
		delegation = &Delegation{
			Delegator: delegator,
			Validator: validatorAddr,
			Amount:    0,
		}
	}
	
	delegation.Amount += amount
	validator.DelegatedStake += amount
	
	sm.setDelegation(delegation)
	sm.setValidator(validator)
	
	env.Log("Delegated: " + delegator + " -> " + validatorAddr + " amount: " + strconv.FormatUint(amount, 10))
}

func (sm *StakingModule) Undelegate(validatorAddr string, amount uint64) {
	delegator := env.PredecessorAccountId()
	currentHeight := sm.store.GetBlockHeight()
	
	delegation := sm.getDelegation(delegator, validatorAddr)
	if delegation == nil || delegation.Amount < amount {
		env.Panic("Insufficient delegation")
	}
	
	validator := sm.getValidator(validatorAddr)
	if validator == nil {
		env.Panic("Validator not found")
	}
	
	delegation.Amount -= amount
	validator.DelegatedStake -= amount
	
	sm.setDelegation(delegation)
	sm.setValidator(validator)
	
	unbondingEntry := &UnbondingEntry{
		Delegator:    delegator,
		Validator:    validatorAddr,
		Amount:       amount,
		UnlockHeight: currentHeight + UnbondingPeriod,
	}
	
	sm.addUnbondingEntry(unbondingEntry)
	
	env.Log("Undelegated: " + delegator + " from " + validatorAddr + " amount: " + strconv.FormatUint(amount, 10))
}

func (sm *StakingModule) AddValidator(address string) {
	validator := &Validator{
		Address:        address,
		DelegatedStake: 0,
		IsActive:       true,
	}
	
	sm.setValidator(validator)
	env.Log("Added validator: " + address)
}

func (sm *StakingModule) BeginBlock(height uint64) {
	env.Log("BeginBlock: " + strconv.FormatUint(height, 10))
}

func (sm *StakingModule) EndBlock(height uint64) {
	sm.processUnbonding(height)
	sm.distributeRewards()
	env.Log("EndBlock: " + strconv.FormatUint(height, 10))
}

func (sm *StakingModule) processUnbonding(currentHeight uint64) {
	prefix := []byte("unbonding:")
	
	sm.store.IterPrefix(prefix, func(key, value []byte) bool {
		entry := DeserializeUnbondingEntry(value)
		if entry != nil && entry.UnlockHeight <= currentHeight {
			sm.bankModule.Transfer(entry.Delegator, entry.Amount)
			sm.store.Delete(key)
			env.Log("Released unbonding: " + entry.Delegator + " amount: " + strconv.FormatUint(entry.Amount, 10))
		}
		return true
	})
}

func (sm *StakingModule) distributeRewards() {
	prefix := []byte("validator:")
	
	sm.store.IterPrefix(prefix, func(key, value []byte) bool {
		validator := DeserializeValidator(value)
		if validator != nil && validator.IsActive && validator.DelegatedStake > 0 {
			rewardAmount := validator.DelegatedStake * RewardPercentage / 100
			sm.bankModule.Mint("staking_pool", rewardAmount)
			env.Log("Distributed rewards to validator: " + validator.Address + " amount: " + strconv.FormatUint(rewardAmount, 10))
		}
		return true
	})
}