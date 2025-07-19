package storage

import "bytes"

type MockStore struct {
	Data map[string][]byte
}

func NewMockStore() *MockStore {
	return &MockStore{
		Data: make(map[string][]byte),
	}
}

func (ms *MockStore) Get(key []byte) []byte {
	return ms.Data[string(key)]
}

func (ms *MockStore) Set(key []byte, value []byte) {
	ms.Data[string(key)] = value
}

func (ms *MockStore) Delete(key []byte) {
	delete(ms.Data, string(key))
}

func (ms *MockStore) IterPrefix(prefix []byte, callback func(key, value []byte) bool) {
	for k, v := range ms.Data {
		if bytes.HasPrefix([]byte(k), prefix) {
			if !callback([]byte(k), v) {
				break
			}
		}
	}
}