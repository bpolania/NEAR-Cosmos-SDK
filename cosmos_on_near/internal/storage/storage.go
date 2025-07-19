package storage

import (
	"bytes"
	"github.com/vlmoon99/near-sdk-go/env"
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
	return env.StorageRead(key)
}

func (s *NearStore) Set(key []byte, value []byte) {
	env.StorageWrite(key, value)
}

func (s *NearStore) Delete(key []byte) {
	env.StorageRemove(key)
}

func (s *NearStore) IterPrefix(prefix []byte, callback func(key, value []byte) bool) {
	prefixLen := len(prefix)
	
	iterator := env.StorageIter(prefix, append(prefix, 0xFF))
	
	for iterator.Valid() {
		key := iterator.Key()
		if len(key) < prefixLen || !bytes.HasPrefix(key, prefix) {
			break
		}
		
		value := iterator.Value()
		if !callback(key, value) {
			break
		}
		iterator.Next()
	}
}