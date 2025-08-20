use std::sync::Arc;
use crate::cosmwasm::state::StateManager;
use crate::cosmwasm::types::StateChange;
use near_jsonrpc_client::JsonRpcClient;
use near_primitives::types::AccountId;

/// Create a mock StateManager for testing
fn create_test_state_manager() -> StateManager {
    let client = Arc::new(JsonRpcClient::connect("http://localhost:3030"));
    let account_id: AccountId = "test.near".parse().unwrap();
    StateManager::new(client, account_id)
}

#[tokio::test]
async fn test_state_manager_creation() {
    let manager = create_test_state_manager();
    
    // Check initial state
    let stats = manager.get_cache_stats().await;
    assert_eq!(stats.len(), 0);
}

#[tokio::test]
async fn test_set_and_get_from_cache() {
    let manager = create_test_state_manager();
    
    let key = vec![1, 2, 3];
    let value = vec![4, 5, 6];
    
    // Set a value
    manager.set("contract1", key.clone(), value.clone()).await;
    
    // Get should return from cache
    let result = manager.get("contract1", &key).await;
    assert_eq!(result, Some(value));
}

#[tokio::test]
async fn test_remove_from_cache() {
    let manager = create_test_state_manager();
    
    let key = vec![1, 2, 3];
    let value = vec![4, 5, 6];
    
    // Set then remove
    manager.set("contract1", key.clone(), value).await;
    manager.remove("contract1", key.clone()).await;
    
    // Should not be in cache
    let result = manager.get("contract1", &key).await;
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_pending_changes() {
    let manager = create_test_state_manager();
    
    // Add multiple changes
    manager.set("contract1", vec![1], vec![10]).await;
    manager.set("contract1", vec![2], vec![20]).await;
    manager.remove("contract1", vec![3]).await;
    
    // Get pending changes
    let changes = manager.get_pending_changes("contract1").await;
    assert_eq!(changes.len(), 3);
    
    // Check change types
    assert!(matches!(changes[0], StateChange::Set { .. }));
    assert!(matches!(changes[1], StateChange::Set { .. }));
    assert!(matches!(changes[2], StateChange::Remove { .. }));
    
    // Pending changes should be cleared after getting them
    let changes2 = manager.get_pending_changes("contract1").await;
    assert_eq!(changes2.len(), 0);
}

#[tokio::test]
async fn test_cache_isolation() {
    let manager = create_test_state_manager();
    
    // Set values for different contracts
    manager.set("contract1", vec![1], vec![10]).await;
    manager.set("contract2", vec![1], vec![20]).await;
    
    // Each contract should have its own value
    assert_eq!(manager.get("contract1", &[1]).await, Some(vec![10]));
    assert_eq!(manager.get("contract2", &[1]).await, Some(vec![20]));
}

#[tokio::test]
async fn test_clear_cache() {
    let manager = create_test_state_manager();
    
    // Set some values
    manager.set("contract1", vec![1], vec![10]).await;
    manager.set("contract1", vec![2], vec![20]).await;
    
    // Clear cache for contract1
    manager.clear_cache("contract1").await;
    
    // Cache should be empty (would try to read from NEAR)
    // In test environment without NEAR, this returns None
    let result = manager.get("contract1", &[1]).await;
    assert_eq!(result, None);
}

#[tokio::test]
async fn test_cache_stats() {
    let manager = create_test_state_manager();
    
    // Add data to multiple contracts
    manager.set("contract1", vec![1], vec![10]).await;
    manager.set("contract1", vec![2], vec![20]).await;
    manager.set("contract2", vec![1], vec![30]).await;
    
    // Check stats
    let stats = manager.get_cache_stats().await;
    assert_eq!(stats.len(), 2);
    
    // Check contract1 stats
    if let Some((count, _height)) = stats.get("contract1") {
        assert_eq!(*count, 2); // Two keys
    } else {
        panic!("contract1 not in stats");
    }
    
    // Check contract2 stats
    if let Some((count, _height)) = stats.get("contract2") {
        assert_eq!(*count, 1); // One key
    } else {
        panic!("contract2 not in stats");
    }
}

#[tokio::test]
async fn test_state_change_ordering() {
    let manager = create_test_state_manager();
    
    // Add changes in specific order
    manager.set("contract1", vec![1], vec![10]).await;
    manager.set("contract1", vec![1], vec![20]).await; // Overwrite
    manager.remove("contract1", vec![1]).await;
    manager.set("contract1", vec![1], vec![30]).await;
    
    // Get pending changes
    let changes = manager.get_pending_changes("contract1").await;
    assert_eq!(changes.len(), 4);
    
    // Verify order is preserved
    match &changes[3] {
        StateChange::Set { value, .. } => assert_eq!(value, &vec![30]),
        _ => panic!("Expected Set as last change"),
    }
}

#[tokio::test]
async fn test_concurrent_access() {
    let manager = Arc::new(create_test_state_manager());
    
    // Spawn multiple tasks that access the state concurrently
    let mut handles = vec![];
    
    for i in 0..10 {
        let manager_clone = manager.clone();
        let handle = tokio::spawn(async move {
            manager_clone.set(
                "contract1",
                vec![i],
                vec![i * 10],
            ).await;
        });
        handles.push(handle);
    }
    
    // Wait for all tasks
    for handle in handles {
        handle.await.unwrap();
    }
    
    // Check that all values were set
    for i in 0..10 {
        let result = manager.get("contract1", &[i]).await;
        assert_eq!(result, Some(vec![i * 10]));
    }
}