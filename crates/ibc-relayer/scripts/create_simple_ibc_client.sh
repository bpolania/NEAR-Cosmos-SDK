#!/bin/bash
set -e

echo "Creating simple IBC Tendermint client on NEAR contract..."

CONTRACT_ID="cosmos-sdk-demo.testnet"
SIGNER_ID="cuteharbor3573.testnet"

# Create a simple working header based on test format
HEADER='{
  "signed_header": {
    "header": {
      "version": {"block": 11, "app": 0},
      "chain_id": "provider",
      "height": 1000,
      "time": 1722297600,
      "last_block_id": {
        "hash": [18, 52, 86, 120, 144, 171, 205, 239],
        "part_set_header": {
          "total": 1,
          "hash": [171, 205, 239, 18, 52, 86, 120, 144]
        }
      },
      "last_commit_hash": [17, 17, 17, 17, 17, 17, 17, 17],
      "data_hash": [34, 34, 34, 34, 34, 34, 34, 34],
      "validators_hash": [51, 51, 51, 51, 51, 51, 51, 51],
      "next_validators_hash": [68, 68, 68, 68, 68, 68, 68, 68],
      "consensus_hash": [85, 85, 85, 85, 85, 85, 85, 85],
      "app_hash": [102, 102, 102, 102, 102, 102, 102, 102],
      "last_results_hash": [119, 119, 119, 119, 119, 119, 119, 119],
      "evidence_hash": [136, 136, 136, 136, 136, 136, 136, 136],
      "proposer_address": [153, 153, 153, 153, 153, 153, 153, 153]
    },
    "commit": {
      "height": 1000,
      "round": 0,
      "block_id": {
        "hash": [18, 52, 86, 120, 144, 171, 205, 239],
        "part_set_header": {
          "total": 1,
          "hash": [171, 205, 239, 18, 52, 86, 120, 144]
        }
      },
      "signatures": [
        {
          "block_id_flag": 2,
          "validator_address": [1, 2, 3, 4, 5, 6, 7, 8],
          "timestamp": 1722297600,
          "signature": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64]
        }
      ]
    }
  },
  "validator_set": {
    "validators": [
      {
        "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        "pub_key": {
          "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
        },
        "voting_power": 1000000000,
        "proposer_priority": 0
      }
    ],
    "proposer": {
      "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
      "pub_key": {
        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
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
        "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
        "pub_key": {
          "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
        },
        "voting_power": 1000000000,
        "proposer_priority": 0
      }
    ],
    "proposer": {
      "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
      "pub_key": {
        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
      },
      "voting_power": 1000000000,
      "proposer_priority": 0
    },
    "total_voting_power": 1000000000
  }
}'

echo "Creating IBC client with provider chain..."

# Create the IBC client
RESULT=$(near call $CONTRACT_ID ibc_create_client "{
  \"chain_id\": \"provider\",
  \"trust_period\": 864000,
  \"unbonding_period\": 2592000,
  \"max_clock_drift\": 60,
  \"initial_header\": $HEADER
}" --accountId $SIGNER_ID --gas 300000000000000)

if [[ "$RESULT" == *"client-0"* ]]; then
    echo "✅ IBC client created successfully!"
    echo "Client ID: client-0"
    echo "Client type: 07-tendermint-0"
    
    # Verify the client was created
    echo "Verifying client state..."
    CLIENT_STATE=$(near view $CONTRACT_ID ibc_get_client_state '{"client_id": "client-0"}')
    echo "Client state: $CLIENT_STATE"
else
    echo "❌ Failed to create IBC client"
    echo "Result: $RESULT"
fi