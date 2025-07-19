package storage

import (
	"encoding/binary"
)

const BlockHeightKey = "block_height"

func (ms *ModuleStore) GetBlockHeight() uint64 {
	data := ms.Get([]byte(BlockHeightKey))
	if len(data) == 0 {
		return 0
	}
	return binary.LittleEndian.Uint64(data)
}

func (ms *ModuleStore) SetBlockHeight(height uint64) {
	data := make([]byte, 8)
	binary.LittleEndian.PutUint64(data, height)
	ms.Set([]byte(BlockHeightKey), data)
}

func (ms *ModuleStore) IncrementBlockHeight() uint64 {
	height := ms.GetBlockHeight() + 1
	ms.SetBlockHeight(height)
	return height
}