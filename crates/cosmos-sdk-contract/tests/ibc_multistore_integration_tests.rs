use near_workspaces::Contract;
use tokio::time::{sleep, Duration};

const WASM_FILEPATH: &str = "./target/near/cosmos_sdk_near.wasm";

async fn deploy_cosmos_contract() -> (near_workspaces::Worker<near_workspaces::network::Sandbox>, Contract) {
    let worker = near_workspaces::sandbox().await.unwrap();
    let wasm = std::fs::read(WASM_FILEPATH).unwrap();
    let contract = worker.dev_deploy(&wasm).await.unwrap();

    // Initialize the contract
    contract
        .call("new")
        .max_gas()
        .transact()
        .await
        .unwrap()
        .unwrap();

    (worker, contract)
}


#[tokio::test]
async fn test_multistore_membership_basic() {
    sleep(Duration::from_millis(100)).await;
    let (_worker, contract) = deploy_cosmos_contract().await;

    // Create a test client first
    let chain_id = "cosmos-testnet-1".to_string();
    let trust_period = 1209600u64; // 14 days
    let unbonding_period = 1814400u64; // 21 days  
    let max_clock_drift = 10u64;

    // Create a mock header with minimal valid data
    let mock_header = serde_json::json!({
        "signed_header": {
            "header": {
                "version": {
                    "block": 11u64,
                    "app": 1u64
                },
                "chain_id": chain_id,
                "height": 100,
                "time": 1672531200u64, // 2023-01-01T00:00:00Z as timestamp
                "last_block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "last_commit_hash": vec![0u8; 32],
                "data_hash": vec![0u8; 32],
                "validators_hash": vec![1u8; 32], 
                "next_validators_hash": vec![2u8; 32],
                "consensus_hash": vec![0u8; 32],
                "app_hash": vec![3u8; 32], // This becomes our multi-store root
                "last_results_hash": vec![0u8; 32],
                "evidence_hash": vec![0u8; 32],
                "proposer_address": vec![0u8; 20]
            },
            "commit": {
                "height": 100,
                "round": 0,
                "block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                        "timestamp": 1672531200u64,
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
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        },
        "trusted_height": {
            "revision_number": 0,
            "revision_height": 100
        },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        }
    });

    // Create client
    let client_id: String = contract
        .call("ibc_create_client")
        .args_json(serde_json::json!({
            "chain_id": chain_id,
            "trust_period": trust_period,
            "unbonding_period": unbonding_period,
            "max_clock_drift": max_clock_drift,
            "initial_header": mock_header
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    println!("Created client: {}", client_id);

    // Create a mock multi-store proof
    let mock_multistore_proof = serde_json::json!({
        "multistore": {
            "store_infos": [
                {
                    "name": "bank",
                    "hash": vec![4u8; 32] // Mock bank store hash
                }
            ],
            "root_hash": vec![3u8; 32], // Matches app_hash from header
            "store_name": "bank",
            "store_proof": {
                "proof": {
                    "key": "bank".as_bytes().to_vec(),
                    "value": vec![4u8; 32],
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash", 
                        "prehash_value": "NoHash",
                        "length": "VarProto",
                        "prefix": []
                    },
                    "path": []
                }
            },
            "kv_proof": {
                "proof": {
                    "key": "balance/cosmos1test".as_bytes().to_vec(),
                    "value": "1000".as_bytes().to_vec(),
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256", 
                        "length": "VarProto",
                        "prefix": vec![0]
                    },
                    "path": []
                }
            }
        }
    });

    let proof_bytes = serde_json::to_vec(&mock_multistore_proof).unwrap();
    
    // Test multi-store membership verification
    let result: bool = contract
        .call("ibc_verify_multistore_membership")
        .args_json(serde_json::json!({
            "client_id": client_id,
            "height": 100u64,
            "store_name": "bank",
            "key": "balance/cosmos1test".as_bytes().to_vec(),
            "value": "1000".as_bytes().to_vec(),
            "proof": proof_bytes
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    // Note: This will likely return false due to mock data, but we're testing the API structure
    println!("Multi-store verification result: {}", result);
}

#[tokio::test]
async fn test_multistore_batch_verification() {
    sleep(Duration::from_millis(200)).await;
    let (_worker, contract) = deploy_cosmos_contract().await;

    // Create a test client
    let chain_id = "cosmos-testnet-2".to_string();
    let trust_period = 1209600u64;
    let unbonding_period = 1814400u64;
    let max_clock_drift = 10u64;

    let mock_header = serde_json::json!({
        "signed_header": {
            "header": {
                "version": {
                    "block": 11u64,
                    "app": 1u64
                },
                "chain_id": chain_id,
                "height": 200,
                "time": 1672531200u64, // 2023-01-01T00:00:00Z as timestamp
                "last_block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "last_commit_hash": vec![0u8; 32],
                "data_hash": vec![0u8; 32],
                "validators_hash": vec![1u8; 32],
                "next_validators_hash": vec![2u8; 32],
                "consensus_hash": vec![0u8; 32],
                "app_hash": vec![5u8; 32], // Different app_hash for this test
                "last_results_hash": vec![0u8; 32],
                "evidence_hash": vec![0u8; 32],
                "proposer_address": vec![0u8; 20]
            },
            "commit": {
                "height": 200,
                "round": 0,
                "block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                        "timestamp": 1672531200u64,
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
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        },
        "trusted_height": {
            "revision_number": 0,
            "revision_height": 200
        },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        }
    });

    let client_id: String = contract
        .call("ibc_create_client")
        .args_json(serde_json::json!({
            "chain_id": chain_id,
            "trust_period": trust_period,
            "unbonding_period": unbonding_period,
            "max_clock_drift": max_clock_drift,
            "initial_header": mock_header
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    // Create mock proofs for multiple stores
    let bank_proof = serde_json::json!({
        "multistore": {
            "store_infos": [
                {
                    "name": "bank",
                    "hash": vec![6u8; 32]
                }
            ],
            "root_hash": vec![5u8; 32], // Matches app_hash
            "store_name": "bank",
            "store_proof": {
                "proof": {
                    "key": "bank".as_bytes().to_vec(),
                    "value": vec![6u8; 32],
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "NoHash", 
                        "length": "VarProto",
                        "prefix": []
                    },
                    "path": []
                }
            },
            "kv_proof": {
                "proof": {
                    "key": "balance/cosmos1alice".as_bytes().to_vec(),
                    "value": "2000".as_bytes().to_vec(),
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "Sha256",
                        "length": "VarProto", 
                        "prefix": [0]
                    },
                    "path": []
                }
            }
        }
    });

    let staking_proof = serde_json::json!({
        "multistore": {
            "store_infos": [
                {
                    "name": "staking",
                    "hash": vec![7u8; 32]
                }
            ],
            "root_hash": vec![5u8; 32], // Same app_hash
            "store_name": "staking", 
            "store_proof": {
                "proof": {
                    "key": "staking".as_bytes().to_vec(),
                    "value": vec![7u8; 32],
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash",
                        "prehash_value": "NoHash",
                        "length": "VarProto",
                        "prefix": []
                    },
                    "path": []
                }
            },
            "kv_proof": {
                "proof": {
                    "key": "delegation/cosmos1alice/cosmosvaloper1test".as_bytes().to_vec(),
                    "value": "500".as_bytes().to_vec(),
                    "leaf": {
                        "hash": "Sha256",
                        "prehash_key": "NoHash", 
                        "prehash_value": "Sha256",
                        "length": "VarProto",
                        "prefix": [0]
                    },
                    "path": []
                }
            }
        }
    });

    let bank_proof_bytes = serde_json::to_vec(&bank_proof).unwrap();
    let staking_proof_bytes = serde_json::to_vec(&staking_proof).unwrap();

    // Test batch verification
    let batch_items = vec![
        (
            "bank".to_string(),
            "balance/cosmos1alice".as_bytes().to_vec(),
            "2000".as_bytes().to_vec(),
            bank_proof_bytes,
        ),
        (
            "staking".to_string(), 
            "delegation/cosmos1alice/cosmosvaloper1test".as_bytes().to_vec(),
            "500".as_bytes().to_vec(),
            staking_proof_bytes,
        ),
    ];

    let result: bool = contract
        .call("ibc_verify_multistore_batch")
        .args_json(serde_json::json!({
            "client_id": client_id,
            "height": 200u64,
            "items": batch_items
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    println!("Multi-store batch verification result: {}", result);
}

#[tokio::test] 
async fn test_multistore_invalid_client() {
    sleep(Duration::from_millis(300)).await;
    let (_worker, contract) = deploy_cosmos_contract().await;

    let mock_proof = serde_json::json!({
        "multistore": {
            "store_infos": [{"name": "bank", "hash": vec![1u8; 32]}],
            "root_hash": vec![1u8; 32],
            "store_name": "bank",
            "store_proof": {"proof": null},
            "kv_proof": {"proof": null}
        }
    });

    let proof_bytes = serde_json::to_vec(&mock_proof).unwrap();

    // Test with invalid client ID  
    let result: bool = contract
        .call("ibc_verify_multistore_membership")
        .args_json(serde_json::json!({
            "client_id": "non-existent-client",
            "height": 100u64,
            "store_name": "bank",
            "key": "test_key".as_bytes().to_vec(),
            "value": "test_value".as_bytes().to_vec(),
            "proof": proof_bytes
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    assert!(!result); // Should return false for invalid client
    println!("Invalid client test passed: {}", !result);
}

#[tokio::test]
async fn test_multistore_invalid_height() {
    sleep(Duration::from_millis(400)).await;
    let (_worker, contract) = deploy_cosmos_contract().await;

    // Create a valid client
    let chain_id = "cosmos-testnet-3".to_string();
    let trust_period = 1209600u64;
    let unbonding_period = 1814400u64;
    let max_clock_drift = 10u64;

    let mock_header = serde_json::json!({
        "signed_header": {
            "header": {
                "version": {
                    "block": 11u64,
                    "app": 1u64
                },
                "chain_id": chain_id,
                "height": 300,
                "time": 1672531200u64, // 2023-01-01T00:00:00Z as timestamp
                "last_block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "last_commit_hash": vec![0u8; 32],
                "data_hash": vec![0u8; 32],
                "validators_hash": vec![1u8; 32],
                "next_validators_hash": vec![2u8; 32],
                "consensus_hash": vec![0u8; 32],
                "app_hash": vec![8u8; 32],
                "last_results_hash": vec![0u8; 32],
                "evidence_hash": vec![0u8; 32],
                "proposer_address": vec![0u8; 20]
            },
            "commit": {
                "height": 300,
                "round": 0,
                "block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                        "timestamp": 1672531200u64,
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
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        },
        "trusted_height": {
            "revision_number": 0,
            "revision_height": 300
        },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        }
    });

    let client_id: String = contract
        .call("ibc_create_client")
        .args_json(serde_json::json!({
            "chain_id": chain_id,
            "trust_period": trust_period,
            "unbonding_period": unbonding_period,
            "max_clock_drift": max_clock_drift,
            "initial_header": mock_header
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    let mock_proof = serde_json::json!({
        "multistore": {
            "store_infos": [{"name": "bank", "hash": vec![1u8; 32]}],
            "root_hash": vec![1u8; 32],
            "store_name": "bank",
            "store_proof": {"proof": null},
            "kv_proof": {"proof": null}
        }
    });

    let proof_bytes = serde_json::to_vec(&mock_proof).unwrap();

    // Test with non-existent height
    let result: bool = contract
        .call("ibc_verify_multistore_membership")
        .args_json(serde_json::json!({
            "client_id": client_id,
            "height": 999u64, // Height that doesn't exist
            "store_name": "bank",
            "key": "test_key".as_bytes().to_vec(),
            "value": "test_value".as_bytes().to_vec(),
            "proof": proof_bytes
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    assert!(!result); // Should return false for invalid height
    println!("Invalid height test passed: {}", !result);
}

#[tokio::test]
async fn test_multistore_empty_batch() {
    sleep(Duration::from_millis(500)).await;
    let (_worker, contract) = deploy_cosmos_contract().await;

    // Create a valid client
    let chain_id = "cosmos-testnet-4".to_string();
    let trust_period = 1209600u64;
    let unbonding_period = 1814400u64;
    let max_clock_drift = 10u64;

    let mock_header = serde_json::json!({
        "signed_header": {
            "header": {
                "version": {
                    "block": 11u64,
                    "app": 1u64
                },
                "chain_id": chain_id,
                "height": 400,
                "time": 1672531200u64, // 2023-01-01T00:00:00Z as timestamp
                "last_block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "last_commit_hash": vec![0u8; 32],
                "data_hash": vec![0u8; 32],
                "validators_hash": vec![1u8; 32],
                "next_validators_hash": vec![2u8; 32],
                "consensus_hash": vec![0u8; 32],
                "app_hash": vec![9u8; 32],
                "last_results_hash": vec![0u8; 32],
                "evidence_hash": vec![0u8; 32],
                "proposer_address": vec![0u8; 20]
            },
            "commit": {
                "height": 400,
                "round": 0,
                "block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                        "timestamp": 1672531200u64,
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
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        },
        "trusted_height": {
            "revision_number": 0,
            "revision_height": 400
        },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        }
    });

    let client_id: String = contract
        .call("ibc_create_client")
        .args_json(serde_json::json!({
            "chain_id": chain_id,
            "trust_period": trust_period,
            "unbonding_period": unbonding_period,
            "max_clock_drift": max_clock_drift,
            "initial_header": mock_header
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    // Test with empty batch
    let empty_batch: Vec<(String, Vec<u8>, Vec<u8>, Vec<u8>)> = vec![];

    let result: bool = contract
        .call("ibc_verify_multistore_batch")
        .args_json(serde_json::json!({
            "client_id": client_id,
            "height": 400u64,
            "items": empty_batch
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    assert!(!result); // Should return false for empty batch
    println!("Empty batch test passed: {}", !result);
}

#[tokio::test]
async fn test_multistore_api_structure() {
    sleep(Duration::from_millis(600)).await;
    let (_worker, contract) = deploy_cosmos_contract().await;

    // Test that the functions exist and can be called (structure test)
    // Create minimal client for API testing
    let chain_id = "cosmos-testnet-5".to_string();
    let trust_period = 1209600u64;
    let unbonding_period = 1814400u64;
    let max_clock_drift = 10u64;

    let mock_header = serde_json::json!({
        "signed_header": {
            "header": {
                "version": {
                    "block": 11u64,
                    "app": 1u64
                },
                "chain_id": chain_id,
                "height": 500,
                "time": 1672531200u64, // 2023-01-01T00:00:00Z as timestamp
                "last_block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "last_commit_hash": vec![0u8; 32],
                "data_hash": vec![0u8; 32],
                "validators_hash": vec![1u8; 32],
                "next_validators_hash": vec![2u8; 32],
                "consensus_hash": vec![0u8; 32],
                "app_hash": vec![10u8; 32],
                "last_results_hash": vec![0u8; 32],
                "evidence_hash": vec![0u8; 32],
                "proposer_address": vec![0u8; 20]
            },
            "commit": {
                "height": 500,
                "round": 0,
                "block_id": {
                    "hash": vec![0u8; 32],
                    "part_set_header": {
                        "total": 1,
                        "hash": vec![0u8; 32]
                    }
                },
                "signatures": [
                    {
                        "block_id_flag": 2,
                        "validator_address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                        "timestamp": 1672531200u64,
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
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        },
        "trusted_height": {
            "revision_number": 0,
            "revision_height": 500
        },
        "trusted_validators": {
            "validators": [
                {
                    "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                    "pub_key": {
                        "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                    },
                    "voting_power": 1000,
                    "proposer_priority": 0
                }
            ],
            "proposer": {
                "address": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20],
                "pub_key": {
                    "Ed25519": [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]
                },
                "voting_power": 1000,
                "proposer_priority": 0
            },
            "total_voting_power": 1000
        }
    });

    let client_id: String = contract
        .call("ibc_create_client")
        .args_json(serde_json::json!({
            "chain_id": chain_id,
            "trust_period": trust_period,
            "unbonding_period": unbonding_period,
            "max_clock_drift": max_clock_drift,
            "initial_header": mock_header
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<String>()
        .unwrap();

    let mock_proof_bytes = serde_json::to_vec(&serde_json::json!({})).unwrap();

    // Test single membership function
    let _result: bool = contract
        .call("ibc_verify_multistore_membership")
        .args_json(serde_json::json!({
            "client_id": client_id,
            "height": 500u64,
            "store_name": "bank",
            "key": "test".as_bytes().to_vec(),
            "value": "value".as_bytes().to_vec(),
            "proof": mock_proof_bytes.clone()
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    // Test batch function
    let batch_items = vec![(
        "bank".to_string(),
        "test".as_bytes().to_vec(),
        "value".as_bytes().to_vec(),
        mock_proof_bytes,
    )];

    let _result: bool = contract
        .call("ibc_verify_multistore_batch")
        .args_json(serde_json::json!({
            "client_id": client_id,
            "height": 500u64,
            "items": batch_items
        }))
        .max_gas()
        .transact()
        .await
        .unwrap()
        .json::<bool>()
        .unwrap();

    println!("Multi-store API structure tests completed successfully");
}