package governance

import (
	"cosmos_on_near/internal/near"
	"cosmos_on_near/internal/storage"
	"encoding/binary"
	"strconv"
)

const (
	VotingPeriod = 50 // blocks
)

type GovernanceModule struct {
	store *storage.ModuleStore
}

func NewGovernanceModule(store storage.Store) *GovernanceModule {
	return &GovernanceModule{
		store: storage.NewModuleStore(store, "governance"),
	}
}

func (gm *GovernanceModule) getNextProposalID() uint64 {
	data := gm.store.Get([]byte("next_proposal_id"))
	if len(data) == 0 {
		return 1
	}
	return binary.LittleEndian.Uint64(data)
}

func (gm *GovernanceModule) setNextProposalID(id uint64) {
	data := make([]byte, 8)
	binary.LittleEndian.PutUint64(data, id)
	gm.store.Set([]byte("next_proposal_id"), data)
}

func (gm *GovernanceModule) getProposal(id uint64) *Proposal {
	key := []byte("proposal:" + strconv.FormatUint(id, 10))
	data := gm.store.Get(key)
	return DeserializeProposal(data)
}

func (gm *GovernanceModule) setProposal(proposal *Proposal) {
	key := []byte("proposal:" + strconv.FormatUint(proposal.ID, 10))
	gm.store.Set(key, proposal.Serialize())
}

func (gm *GovernanceModule) getVote(proposalID uint64, voter string) *Vote {
	key := []byte("vote:" + strconv.FormatUint(proposalID, 10) + ":" + voter)
	data := gm.store.Get(key)
	return DeserializeVote(data)
}

func (gm *GovernanceModule) setVote(vote *Vote) {
	key := []byte("vote:" + strconv.FormatUint(vote.ProposalID, 10) + ":" + vote.Voter)
	gm.store.Set(key, vote.Serialize())
}

func (gm *GovernanceModule) getParameter(key string) string {
	data := gm.store.Get([]byte("param:" + key))
	return string(data)
}

func (gm *GovernanceModule) setParameter(key, value string) {
	gm.store.Set([]byte("param:"+key), []byte(value))
}

func (gm *GovernanceModule) SubmitProposal(title, description, paramKey, paramValue string) uint64 {
	proposer := "system" // TODO: Get from NEAR context
	currentHeight := gm.store.GetBlockHeight()
	
	proposalID := gm.getNextProposalID()
	gm.setNextProposalID(proposalID + 1)
	
	proposal := &Proposal{
		ID:          proposalID,
		Title:       title,
		Description: description,
		ParamKey:    paramKey,
		ParamValue:  paramValue,
		EndBlock:    currentHeight + VotingPeriod,
		YesVotes:    0,
		NoVotes:     0,
		Status:      ProposalStatusActive,
	}
	
	gm.setProposal(proposal)
	
	near.LogString("Proposal submitted: " + proposer + " ID: " + strconv.FormatUint(proposalID, 10))
	return proposalID
}

func (gm *GovernanceModule) Vote(proposalID uint64, option uint8) {
	voter := "system" // TODO: Get from NEAR context
	
	proposal := gm.getProposal(proposalID)
	if proposal == nil {
		return // TODO: Handle error properly
	}
	
	if proposal.Status != ProposalStatusActive {
		return // TODO: Handle error properly
	}
	
	currentHeight := gm.store.GetBlockHeight()
	if currentHeight > proposal.EndBlock {
		return // TODO: Handle error properly
	}
	
	existingVote := gm.getVote(proposalID, voter)
	if existingVote != nil {
		return // TODO: Handle error properly
	}
	
	vote := &Vote{
		ProposalID: proposalID,
		Voter:      voter,
		Option:     option,
	}
	
	gm.setVote(vote)
	
	if option == VoteYes {
		proposal.YesVotes++
	} else {
		proposal.NoVotes++
	}
	
	gm.setProposal(proposal)
	
	near.LogString("Vote cast: " + voter + " on proposal " + strconv.FormatUint(proposalID, 10))
}

func (gm *GovernanceModule) EndBlock(height uint64) {
	prefix := []byte("proposal:")
	
	gm.store.IterPrefix(prefix, func(key, value []byte) bool {
		proposal := DeserializeProposal(value)
		if proposal != nil && proposal.Status == ProposalStatusActive && height > proposal.EndBlock {
			gm.tallyProposal(proposal)
		}
		return true
	})
}

func (gm *GovernanceModule) tallyProposal(proposal *Proposal) {
	totalVotes := proposal.YesVotes + proposal.NoVotes
	
	if totalVotes > 0 && proposal.YesVotes*100/totalVotes >= QuorumThreshold {
		proposal.Status = ProposalStatusPassed
		gm.setParameter(proposal.ParamKey, proposal.ParamValue)
		near.LogString("Proposal passed: " + strconv.FormatUint(proposal.ID, 10) + " param: " + proposal.ParamKey + "=" + proposal.ParamValue)
	} else {
		proposal.Status = ProposalStatusRejected
		near.LogString("Proposal rejected: " + strconv.FormatUint(proposal.ID, 10))
	}
	
	gm.setProposal(proposal)
}

func (gm *GovernanceModule) GetParameter(key string) string {
	return gm.getParameter(key)
}