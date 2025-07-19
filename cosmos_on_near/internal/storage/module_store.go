package storage

import (
	"bytes"
)

type ModuleStore struct {
	store  Store
	prefix []byte
}

func NewModuleStore(store Store, moduleName string) *ModuleStore {
	prefix := append([]byte(moduleName), '|')
	return &ModuleStore{
		store:  store,
		prefix: prefix,
	}
}

func (ms *ModuleStore) prefixKey(key []byte) []byte {
	prefixedKey := make([]byte, len(ms.prefix)+len(key))
	copy(prefixedKey, ms.prefix)
	copy(prefixedKey[len(ms.prefix):], key)
	return prefixedKey
}

func (ms *ModuleStore) unprefixKey(prefixedKey []byte) []byte {
	if len(prefixedKey) <= len(ms.prefix) {
		return []byte{}
	}
	return prefixedKey[len(ms.prefix):]
}

func (ms *ModuleStore) Get(key []byte) []byte {
	return ms.store.Get(ms.prefixKey(key))
}

func (ms *ModuleStore) Set(key []byte, value []byte) {
	ms.store.Set(ms.prefixKey(key), value)
}

func (ms *ModuleStore) Delete(key []byte) {
	ms.store.Delete(ms.prefixKey(key))
}

func (ms *ModuleStore) IterPrefix(prefix []byte, callback func(key, value []byte) bool) {
	prefixedPrefix := ms.prefixKey(prefix)
	
	ms.store.IterPrefix(prefixedPrefix, func(key, value []byte) bool {
		if !bytes.HasPrefix(key, ms.prefix) {
			return false
		}
		
		unprefixedKey := ms.unprefixKey(key)
		return callback(unprefixedKey, value)
	})
}