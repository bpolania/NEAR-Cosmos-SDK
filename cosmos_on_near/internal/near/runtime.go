package near

import "unsafe"

// Modern NEAR WebAssembly runtime bindings using //export pattern
// This replaces near-sdk-go with TinyGo 0.34+ compatible approach

// NEAR host environment functions (imported from WASM host)
// These will be provided by the NEAR runtime

//export storage_read
func storageReadHost(keyPtr, keyLen uint32) uint64

//export storage_write  
func storageWriteHost(keyPtr, keyLen uint32, valuePtr, valueLen uint32) uint64

//export storage_remove
func storageRemoveHost(keyPtr, keyLen uint32) uint64

//export current_account_id
func currentAccountIdHost() uint64

//export register_len
func registerLenHost(registerID uint64) uint64

//export read_register
func readRegisterHost(registerID uint64, ptr uint32)

//export value_return
func valueReturnHost(valuePtr, valueLen uint32)

//export log_utf8
func logUtf8Host(messagePtr, messageLen uint32)

// Go wrapper functions that provide a clean API
func StorageRead(key []byte) ([]byte, error) {
	if len(key) == 0 {
		return nil, nil
	}
	
	keyPtr := uint32(uintptr(unsafe.Pointer(&key[0])))
	registerID := storageReadHost(keyPtr, uint32(len(key)))
	
	if registerID == 0 {
		return nil, nil // Key not found
	}
	
	valueLen := registerLenHost(registerID)
	if valueLen == 0 {
		return []byte{}, nil
	}
	
	value := make([]byte, valueLen)
	readRegisterHost(registerID, uint32(uintptr(unsafe.Pointer(&value[0]))))
	return value, nil
}

func StorageWrite(key, value []byte) error {
	if len(key) == 0 {
		return nil
	}
	
	keyPtr := uint32(uintptr(unsafe.Pointer(&key[0])))
	valuePtr := uint32(uintptr(unsafe.Pointer(&value[0])))
	
	storageWriteHost(keyPtr, uint32(len(key)), valuePtr, uint32(len(value)))
	return nil
}

func StorageRemove(key []byte) error {
	if len(key) == 0 {
		return nil
	}
	
	keyPtr := uint32(uintptr(unsafe.Pointer(&key[0])))
	storageRemoveHost(keyPtr, uint32(len(key)))
	return nil
}

func LogString(message string) {
	if len(message) == 0 {
		return
	}
	messageBytes := []byte(message)
	messagePtr := uint32(uintptr(unsafe.Pointer(&messageBytes[0])))
	logUtf8Host(messagePtr, uint32(len(messageBytes)))
}

func ContractValueReturn(value []byte) {
	if len(value) == 0 {
		valueReturnHost(0, 0)
		return
	}
	valuePtr := uint32(uintptr(unsafe.Pointer(&value[0])))
	valueReturnHost(valuePtr, uint32(len(value)))
}