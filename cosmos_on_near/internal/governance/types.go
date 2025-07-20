package governance

import (
	"encoding/binary"
)

type Proposal struct {
	ID          uint64 `json:"id"`
	Title       string `json:"title"`
	Description string `json:"description"`
	ParamKey    string `json:"param_key"`
	ParamValue  string `json:"param_value"`
	EndBlock    uint64 `json:"end_block"`
	YesVotes    uint64 `json:"yes_votes"`
	NoVotes     uint64 `json:"no_votes"`
	Status      uint8  `json:"status"` // 0: Active, 1: Passed, 2: Rejected
}

type Vote struct {
	ProposalID uint64 `json:"proposal_id"`
	Voter      string `json:"voter"`
	Option     uint8  `json:"option"` // 0: No, 1: Yes
}

const (
	ProposalStatusActive   = 0
	ProposalStatusPassed   = 1
	ProposalStatusRejected = 2
	
	VoteNo  = 0
	VoteYes = 1
	
	QuorumThreshold = 50 // 50% of total voting power
)

func (p *Proposal) Serialize() []byte {
	// Simple binary serialization - in production, use JSON or protobuf
	data := make([]byte, 8*6+1) // 6 uint64s + 1 uint8, strings handled separately
	binary.LittleEndian.PutUint64(data[0:8], p.ID)
	binary.LittleEndian.PutUint64(data[8:16], p.EndBlock)
	binary.LittleEndian.PutUint64(data[16:24], p.YesVotes)
	binary.LittleEndian.PutUint64(data[24:32], p.NoVotes)
	data[32] = p.Status
	// Note: Strings are omitted in this simple serialization
	return data
}

func DeserializeProposal(data []byte) *Proposal {
	if len(data) < 33 {
		return nil
	}
	
	proposal := &Proposal{
		ID:       binary.LittleEndian.Uint64(data[0:8]),
		EndBlock: binary.LittleEndian.Uint64(data[8:16]),
		YesVotes: binary.LittleEndian.Uint64(data[16:24]),
		NoVotes:  binary.LittleEndian.Uint64(data[24:32]),
		Status:   data[32],
	}
	return proposal
}

func (v *Vote) Serialize() []byte {
	data := make([]byte, 9) // uint64 + uint8
	binary.LittleEndian.PutUint64(data[0:8], v.ProposalID)
	data[8] = v.Option
	return data
}

func DeserializeVote(data []byte) *Vote {
	if len(data) < 9 {
		return nil
	}
	
	vote := &Vote{
		ProposalID: binary.LittleEndian.Uint64(data[0:8]),
		Option:     data[8],
	}
	return vote
}