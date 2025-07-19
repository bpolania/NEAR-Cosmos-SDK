package governance

import (
	"github.com/vlmoon99/near-sdk-go/borsh"
)

type Proposal struct {
	ID          uint64 `borsh:"id"`
	Title       string `borsh:"title"`
	Description string `borsh:"description"`
	ParamKey    string `borsh:"param_key"`
	ParamValue  string `borsh:"param_value"`
	EndBlock    uint64 `borsh:"end_block"`
	YesVotes    uint64 `borsh:"yes_votes"`
	NoVotes     uint64 `borsh:"no_votes"`
	Status      uint8  `borsh:"status"` // 0: Active, 1: Passed, 2: Rejected
}

type Vote struct {
	ProposalID uint64 `borsh:"proposal_id"`
	Voter      string `borsh:"voter"`
	Option     uint8  `borsh:"option"` // 0: No, 1: Yes
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
	data, _ := borsh.BorshSerialize(p)
	return data
}

func DeserializeProposal(data []byte) *Proposal {
	if len(data) == 0 {
		return nil
	}
	
	var proposal Proposal
	borsh.BorshDeserialize(&proposal, data)
	return &proposal
}

func (v *Vote) Serialize() []byte {
	data, _ := borsh.BorshSerialize(v)
	return data
}

func DeserializeVote(data []byte) *Vote {
	if len(data) == 0 {
		return nil
	}
	
	var vote Vote
	borsh.BorshDeserialize(&vote, data)
	return &vote
}