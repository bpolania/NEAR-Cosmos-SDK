package storage

import (
	"bytes"
	"testing"
)

func TestModuleStore(t *testing.T) {
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

func TestBlockHeight(t *testing.T) {
	mockStore := NewMockStore()
	moduleStore := NewModuleStore(mockStore, "system")
	
	height := moduleStore.GetBlockHeight()
	if height != 0 {
		t.Errorf("Expected initial height 0, got %d", height)
	}
	
	newHeight := moduleStore.IncrementBlockHeight()
	if newHeight != 1 {
		t.Errorf("Expected incremented height 1, got %d", newHeight)
	}
	
	height = moduleStore.GetBlockHeight()
	if height != 1 {
		t.Errorf("Expected stored height 1, got %d", height)
	}
}