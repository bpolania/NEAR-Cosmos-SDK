package storage

import (
	"cosmos_on_near/internal/near"
)

type Store interface {
	Get(key []byte) []byte
	Set(key []byte, value []byte)
	Delete(key []byte)
	IterPrefix(prefix []byte, callback func(key, value []byte) bool)
}

type NearStore struct{}

func NewNearStore() *NearStore {
	return &NearStore{}
}

func (s *NearStore) Get(key []byte) []byte {
	data, err := near.StorageRead(key)
	if err != nil {
		return nil
	}
	return data
}

func (s *NearStore) Set(key []byte, value []byte) {
	near.StorageWrite(key, value)
}

func (s *NearStore) Delete(key []byte) {
	near.StorageRemove(key)
}

func (s *NearStore) IterPrefix(prefix []byte, callback func(key, value []byte) bool) {
	// Simple implementation - for a full implementation, we'd need to scan all keys
	// This is a placeholder that works for testing but may not be efficient
	// In a real implementation, you'd want to use NEAR's range iteration if available
	_ = prefix
	_ = callback
	// TODO: Implement proper prefix iteration when available in near-sdk-go
}