#!/bin/bash
set -e

echo "Creating IBC Tendermint client on NEAR contract..."

CONTRACT_ID="cosmos-sdk-demo.testnet"
SIGNER_ID="cuteharbor3573.testnet"

# Create a basic Tendermint header for the provider chain
# This is a minimal viable header - in production you'd get this from the actual chain
INITIAL_HEADER='{
  "signed_header": {
    "header": {
      "version": {"block": 11, "app": 0},
      "chain_id": "provider",
      "height": 1000,
      "time": 1722297600,
      "last_block_id": {
        "hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
        "parts": {
          "total": 1,
          "hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
        }
      },
      "last_commit_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "data_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "validators_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "next_validators_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "consensus_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "app_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "last_results_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "evidence_hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
      "proposer_address": "ABCDABCDABCDABCDABCDABCD"
    },
    "commit": {
      "height": 1000,
      "round": 0,
      "block_id": {
        "hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
        "parts": {
          "total": 1,
          "hash": "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
        }
      },
      "signatures": []
    }
  },
  "validator_set": {
    "validators": [
      {
        "address": "ABCDABCDABCDABCDABCDABCD",
        "pub_key": {
          "type": "tendermint/PubKeyEd25519",
          "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
        },
        "voting_power": 1000000000,
        "proposer_priority": 0
      }
    ],
    "proposer": {
      "address": "ABCDABCDABCDABCDABCDABCD",
      "pub_key": {
        "type": "tendermint/PubKeyEd25519",
        "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
      },
      "voting_power": 1000000000,
      "proposer_priority": 0
    },
    "total_voting_power": 1000000000
  },
  "trusted_height": {"revision_number": 0, "revision_height": 1000},
  "trusted_validators": {
    "validators": [
      {
        "address": "ABCDABCDABCDABCDABCDABCD",
        "pub_key": {
          "type": "tendermint/PubKeyEd25519",
          "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
        },
        "voting_power": 1000000000,
        "proposer_priority": 0
      }
    ],
    "proposer": {
      "address": "ABCDABCDABCDABCDABCDABCD",
      "pub_key": {
        "type": "tendermint/PubKeyEd25519",
        "value": "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
      },
      "voting_power": 1000000000,
      "proposer_priority": 0
    },
    "total_voting_power": 1000000000
  }
}'

echo "Creating IBC client with chain_id: provider..."

# Create the IBC client
RESULT=$(near call $CONTRACT_ID ibc_create_client "{
  \"chain_id\": \"provider\",
  \"trust_period\": 864000,
  \"unbonding_period\": 2592000,
  \"max_clock_drift\": 60,
  \"initial_header\": $INITIAL_HEADER
}" --accountId $SIGNER_ID --gas 300000000000000)

echo "Result: $RESULT"

# Check if client was created successfully
echo "Checking created client..."
CLIENT_STATE=$(near view $CONTRACT_ID ibc_get_client_state '{"client_id": "07-tendermint-0"}')
echo "Client state: $CLIENT_STATE"

if [ "$CLIENT_STATE" != "null" ]; then
    echo "✅ IBC client created successfully!"
    echo "Client ID: 07-tendermint-0"
    echo "Chain ID: provider"
else
    echo "❌ Failed to create IBC client"
    exit 1
fi

echo "Next steps:"
echo "1. Create IBC connection using: near call $CONTRACT_ID ibc_conn_open_init ..."
echo "2. Create IBC channel for token transfers"