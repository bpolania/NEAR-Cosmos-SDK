package storage

import (
	"bytes"
	"testing"
)

func TestModuleStoreIsolated(t *testing.T) {
	mockStore := NewMockStore()
	moduleStore := NewModuleStore(mockStore, "test")
	
	key := []byte("mykey")
	value := []byte("myvalue")
	
	moduleStore.Set(key, value)
	
	retrieved := moduleStore.Get(key)
	if !bytes.Equal(retrieved, value) {
		t.Errorf("Expected %s, got %s", value, retrieved)
	}
	
	prefixedKey := append([]byte("test|"), key...)
	if !bytes.Equal(mockStore.Get(prefixedKey), value) {
		t.Error("Value not stored with correct prefix")
	}
}

func TestModuleStorePrefix(t *testing.T) {
	mockStore := NewMockStore()
	moduleStore := NewModuleStore(mockStore, "test")
	
	moduleStore.Set([]byte("key1"), []byte("value1"))
	moduleStore.Set([]byte("key2"), []byte("value2"))
	moduleStore.Set([]byte("other"), []byte("other_value"))
	
	var results []string
	moduleStore.IterPrefix([]byte("key"), func(key, value []byte) bool {
		results = append(results, string(key)+":"+string(value))
		return true
	})
	
	if len(results) != 2 {
		t.Errorf("Expected 2 results, got %d", len(results))
	}
}

func TestBlockHeightStorage(t *testing.T) {
	mockStore := NewMockStore()
	moduleStore := NewModuleStore(mockStore, "system")
	
	height := moduleStore.GetBlockHeight()
	if height != 0 {
		t.Errorf("Expected initial height 0, got %d", height)
	}
	
	moduleStore.SetBlockHeight(100)
	height = moduleStore.GetBlockHeight()
	if height != 100 {
		t.Errorf("Expected height 100, got %d", height)
	}
	
	newHeight := moduleStore.IncrementBlockHeight()
	if newHeight != 101 {
		t.Errorf("Expected incremented height 101, got %d", newHeight)
	}
}