# Performance Tuning Guide

This guide provides comprehensive performance optimization strategies for the Proxima smart contract, covering contract-level optimizations, client-side improvements, and operational best practices for achieving optimal throughput and response times.

## Overview

Performance optimization in Proxima involves multiple layers:

- **Contract Optimization**: Efficient Rust code and smart contract patterns
- **Transaction Optimization**: Optimal transaction structure and batching
- **Client Optimization**: Efficient client implementations and connection management
- **Network Optimization**: Infrastructure and network configuration
- **Monitoring & Analytics**: Performance measurement and continuous optimization

## Contract-Level Optimizations

### 1. Memory Management

#### Efficient State Storage

```rust
// Inefficient: Storing entire transaction history
#[near_bindgen]
impl CosmosContract {
    pub fn store_transaction(&mut self, tx_hash: String, tx_data: Vec<u8>) {
        self.transaction_history.insert(&tx_hash, &tx_data); // Expensive storage
    }
}

// Efficient: Store only essential transaction metadata
#[derive(BorshSerialize, BorshDeserialize)]
pub struct TxMetadata {
    pub block_height: u64,
    pub gas_used: u64,
    pub status: u8, // 0 = success, error codes for failures
    pub timestamp: u64,
}

#[near_bindgen]
impl CosmosContract {
    pub fn store_transaction_metadata(&mut self, tx_hash: String, metadata: TxMetadata) {
        self.transaction_metadata.insert(&tx_hash, &metadata); // Minimal storage
    }
}
```

#### Optimize Collection Usage

```rust
use near_sdk::collections::{LookupMap, UnorderedMap, Vector};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct CosmosContract {
    // Use LookupMap for O(1) access without iteration needs
    pub accounts: LookupMap<String, CosmosAccount>,
    
    // Use UnorderedMap when you need to iterate occasionally
    pub validators: UnorderedMap<String, ValidatorInfo>,
    
    // Use Vector for ordered data with frequent appends
    pub recent_transactions: Vector<String>,
    
    // Avoid HashMap/BTreeMap for persistent state (not gas-efficient)
}

impl CosmosContract {
    // Batch operations to reduce storage calls
    pub fn batch_update_accounts(&mut self, updates: Vec<(String, CosmosAccount)>) {
        for (address, account) in updates {
            self.accounts.insert(&address, &account);
        }
    }
    
    // Use views for read-only operations
    pub fn get_account_batch(&self, addresses: Vec<String>) -> Vec<Option<CosmosAccount>> {
        addresses.iter()
            .map(|addr| self.accounts.get(addr))
            .collect()
    }
}
```

### 2. Computational Optimizations

#### Signature Verification Optimization

```rust
use near_sdk::env;

impl SignatureVerifier {
    // Cache signature verification results for repeated signatures
    pub fn verify_with_cache(&mut self, signature: &[u8], message: &[u8], public_key: &[u8]) -> Result<bool, String> {
        let cache_key = self.compute_cache_key(signature, message);
        
        if let Some(cached_result) = self.verification_cache.get(&cache_key) {
            return Ok(cached_result);
        }
        
        let result = self.verify_signature_internal(signature, message, public_key)?;
        
        // Cache result with TTL
        self.verification_cache.insert(&cache_key, &result);
        
        Ok(result)
    }
    
    // ✅ Batch signature verifications when possible
    pub fn verify_batch(&self, signatures: Vec<SignatureData>) -> Vec<bool> {
        signatures.par_iter() // Use parallel processing for independent verifications
            .map(|sig_data| {
                self.verify_signature_internal(&sig_data.signature, &sig_data.message, &sig_data.public_key)
                    .unwrap_or(false)
            })
            .collect()
    }
    
    // ✅ Early exit on invalid signatures in multi-signature scenarios
    pub fn verify_multisig(&self, signatures: &[SignatureData], threshold: usize) -> bool {
        let mut valid_count = 0;
        
        for sig_data in signatures {
            if self.verify_signature_internal(&sig_data.signature, &sig_data.message, &sig_data.public_key).unwrap_or(false) {
                valid_count += 1;
                if valid_count >= threshold {
                    return true; // Early exit once threshold is met
                }
            }
        }
        
        false
    }
}
```

#### Message Processing Optimization

```rust
impl MessageProcessor {
    // ✅ Use pattern matching for efficient message routing
    pub fn process_message(&mut self, message: &Any) -> Result<MessageResponse, ProcessingError> {
        match message.type_url.as_str() {
            "/cosmos.bank.v1beta1.MsgSend" => {
                self.process_bank_send(message)
            }
            "/cosmos.staking.v1beta1.MsgDelegate" => {
                self.process_staking_delegate(message)
            }
            "/cosmos.gov.v1beta1.MsgVote" => {
                self.process_governance_vote(message)
            }
            _ => Err(ProcessingError::UnsupportedMessageType(message.type_url.clone()))
        }
    }
    
    // ✅ Optimize common operations
    pub fn process_bank_send(&mut self, message: &Any) -> Result<MessageResponse, ProcessingError> {
        let msg: MsgSend = self.decode_message(message)?;
        
        // Fast path for single denomination transfers
        if msg.amount.len() == 1 {
            return self.process_single_send(&msg);
        }
        
        // Multi-denomination path
        self.process_multi_send(&msg)
    }
    
    // ✅ Pre-validate before expensive operations
    pub fn process_single_send(&mut self, msg: &MsgSend) -> Result<MessageResponse, ProcessingError> {
        let coin = &msg.amount[0];
        
        // Quick validation checks first
        if coin.amount.parse::<u128>().is_err() {
            return Err(ProcessingError::InvalidAmount);
        }
        
        if !self.is_valid_address(&msg.from_address) || !self.is_valid_address(&msg.to_address) {
            return Err(ProcessingError::InvalidAddress);
        }
        
        // Expensive balance check only after validation
        let sender_balance = self.get_account_balance(&msg.from_address, &coin.denom)?;
        let amount: u128 = coin.amount.parse().unwrap();
        
        if sender_balance < amount {
            return Err(ProcessingError::InsufficientFunds);
        }
        
        // Execute transfer
        self.execute_transfer(&msg.from_address, &msg.to_address, amount, &coin.denom)
    }
}
```

### 3. Gas Optimization

#### Reduce Storage Operations

```rust
impl CosmosContract {
    // ✅ Minimize storage writes by batching updates
    pub fn process_transaction_batch(&mut self, tx: CosmosTx) -> TxResponse {
        let mut account_updates = Vec::new();
        let mut balance_updates = Vec::new();
        
        // Process all messages and collect updates
        for message in tx.body.messages {
            match self.process_message_simulation(&message) {
                Ok((account_change, balance_change)) => {
                    account_updates.push(account_change);
                    balance_updates.push(balance_change);
                }
                Err(e) => return TxResponse::error(e, None),
            }
        }
        
        // Apply all updates in batch (single storage operation per account)
        self.apply_account_updates_batch(account_updates);
        self.apply_balance_updates_batch(balance_updates);
        
        TxResponse::success()
    }
    
    // ✅ Use lazy loading for expensive operations
    pub fn get_validator_info(&self, validator_address: &str) -> Option<ValidatorInfo> {
        // Check cache first
        if let Some(cached) = self.validator_cache.get(validator_address) {
            return Some(cached);
        }
        
        // Load from storage only if needed
        let validator = self.validators.get(validator_address)?;
        
        // Cache for future use
        self.validator_cache.insert(validator_address.to_string(), validator.clone());
        
        Some(validator)
    }
}
```

## Transaction-Level Optimizations

### 1. Message Batching Strategies

```javascript
class OptimalTransactionBatcher {
  constructor(maxGasPerTx = 2000000, maxMessagesPerTx = 20) {
    this.maxGasPerTx = maxGasPerTx;
    this.maxMessagesPerTx = maxMessagesPerTx;
    this.gasEstimates = new Map();
  }
  
  // ✅ Intelligent message batching based on gas efficiency
  createOptimalBatches(messages) {
    // Sort messages by gas efficiency (gas per operation value)
    const sortedMessages = messages.sort((a, b) => {
      return this.calculateEfficiency(a) - this.calculateEfficiency(b);
    });
    
    const batches = [];
    let currentBatch = [];
    let currentGas = 50000; // Base transaction gas
    
    for (const message of sortedMessages) {
      const messageGas = this.estimateMessageGas(message);
      
      // Check if we can add this message to current batch
      if (currentBatch.length < this.maxMessagesPerTx && 
          currentGas + messageGas <= this.maxGasPerTx) {
        currentBatch.push(message);
        currentGas += messageGas;
      } else {
        // Start new batch
        if (currentBatch.length > 0) {
          batches.push({
            messages: currentBatch,
            estimatedGas: currentGas
          });
        }
        currentBatch = [message];
        currentGas = 50000 + messageGas;
      }
    }
    
    // Add final batch
    if (currentBatch.length > 0) {
      batches.push({
        messages: currentBatch,
        estimatedGas: currentGas
      });
    }
    
    return batches;
  }
  
  calculateEfficiency(message) {
    const gas = this.estimateMessageGas(message);
    const value = this.estimateMessageValue(message);
    return gas / Math.max(value, 1); // Lower is better
  }
  
  estimateMessageValue(message) {
    switch (message['@type']) {
      case '/cosmos.bank.v1beta1.MsgSend':
        return parseInt(message.amount[0].amount);
      case '/cosmos.staking.v1beta1.MsgDelegate':
        return parseInt(message.amount.amount);
      default:
        return 1000000; // Default value for comparison
    }
  }
  
  // ✅ Optimize message order within transactions
  optimizeMessageOrder(messages) {
    // Group by type for better processing efficiency
    const grouped = {
      bank: [],
      staking: [],
      governance: [],
      other: []
    };
    
    messages.forEach(msg => {
      const type = msg['@type'];
      if (type.includes('bank')) {
        grouped.bank.push(msg);
      } else if (type.includes('staking')) {
        grouped.staking.push(msg);
      } else if (type.includes('gov')) {
        grouped.governance.push(msg);
      } else {
        grouped.other.push(msg);
      }
    });
    
    // Return in optimal processing order
    return [
      ...grouped.bank,      // Process transfers first (simpler)
      ...grouped.staking,   // Then staking operations
      ...grouped.governance, // Then governance
      ...grouped.other      // Finally other operations
    ];
  }
}
```

### 2. Parallel Transaction Processing

```javascript
class ParallelTransactionManager {
  constructor(client, maxConcurrency = 10) {
    this.client = client;
    this.maxConcurrency = maxConcurrency;
    this.activeRequests = 0;
    this.requestQueue = [];
  }
  
  // ✅ Process independent transactions in parallel
  async processTransactionsBatch(transactions) {
    // Group by sender to handle sequence dependencies
    const groupedBySender = this.groupBySender(transactions);
    
    const results = new Map();
    const promises = [];
    
    // Process each sender's transactions sequentially, but different senders in parallel
    for (const [sender, senderTxs] of groupedBySender) {
      const promise = this.processSenderTransactions(sender, senderTxs);
      promises.push(promise);
      
      // Limit concurrent senders to avoid overwhelming the network
      if (promises.length >= this.maxConcurrency) {
        const completed = await Promise.allSettled(promises.splice(0, this.maxConcurrency));
        this.processResults(completed, results);
      }
    }
    
    // Process remaining promises
    if (promises.length > 0) {
      const completed = await Promise.allSettled(promises);
      this.processResults(completed, results);
    }
    
    return results;
  }
  
  async processSenderTransactions(sender, transactions) {
    const results = [];
    let currentSequence = await this.getCurrentSequence(sender);
    
    for (const tx of transactions) {
      // Update sequence for this transaction
      tx.auth_info.signer_infos[0].sequence = currentSequence;
      
      try {
        const result = await this.sendTransaction(tx);
        results.push({ tx, result, success: result.code === 0 });
        
        if (result.code === 0) {
          currentSequence++; // Only increment on success
        }
      } catch (error) {
        results.push({ tx, error, success: false });
      }
    }
    
    return { sender, results };
  }
  
  // ✅ Implement request throttling to avoid overwhelming the RPC
  async sendTransaction(tx) {
    return new Promise((resolve, reject) => {
      const request = { tx, resolve, reject };
      
      if (this.activeRequests < this.maxConcurrency) {
        this.executeRequest(request);
      } else {
        this.requestQueue.push(request);
      }
    });
  }
  
  async executeRequest(request) {
    this.activeRequests++;
    
    try {
      const result = await this.client.broadcastTxSync(this.encodeTx(request.tx));
      request.resolve(result);
    } catch (error) {
      request.reject(error);
    } finally {
      this.activeRequests--;
      
      // Process next request in queue
      if (this.requestQueue.length > 0) {
        const nextRequest = this.requestQueue.shift();
        setImmediate(() => this.executeRequest(nextRequest));
      }
    }
  }
  
  groupBySender(transactions) {
    const grouped = new Map();
    
    for (const tx of transactions) {
      const sender = this.extractSender(tx);
      if (!grouped.has(sender)) {
        grouped.set(sender, []);
      }
      grouped.get(sender).push(tx);
    }
    
    return grouped;
  }
  
  extractSender(tx) {
    const firstMessage = tx.body.messages[0];
    return firstMessage.from_address || 
           firstMessage.delegator_address || 
           firstMessage.voter;
  }
  
  async getCurrentSequence(sender) {
    // Implementation depends on your account query method
    const account = await this.client.queryAccount(sender);
    return account.sequence;
  }
  
  encodeTx(tx) {
    return Buffer.from(JSON.stringify(tx)).toString('base64');
  }
  
  processResults(completed, results) {
    completed.forEach((result, index) => {
      if (result.status === 'fulfilled') {
        const { sender, results: senderResults } = result.value;
        results.set(sender, senderResults);
      }
    });
  }
}
```

## Client-Side Optimizations

### 1. Connection Management

```javascript
class OptimizedProximaClient {
  constructor(config) {
    this.config = {
      rpcUrl: config.rpcUrl,
      contractId: config.contractId,
      maxConnections: config.maxConnections || 10,
      connectionTimeout: config.connectionTimeout || 30000,
      retryAttempts: config.retryAttempts || 3,
      retryDelay: config.retryDelay || 1000,
      ...config
    };
    
    this.connectionPool = [];
    this.activeConnections = 0;
    this.requestQueue = [];
    this.cache = new Map();
  }
  
  // ✅ Implement connection pooling
  async getConnection() {
    if (this.connectionPool.length > 0) {
      return this.connectionPool.pop();
    }
    
    if (this.activeConnections < this.config.maxConnections) {
      this.activeConnections++;
      return this.createConnection();
    }
    
    // Wait for available connection
    return new Promise((resolve) => {
      this.requestQueue.push(resolve);
    });
  }
  
  createConnection() {
    return {
      id: Math.random().toString(36),
      lastUsed: Date.now(),
      client: this.createHttpClient()
    };
  }
  
  createHttpClient() {
    // Configure HTTP client with optimal settings
    return axios.create({
      baseURL: this.config.rpcUrl,
      timeout: this.config.connectionTimeout,
      headers: {
        'Content-Type': 'application/json',
        'Accept': 'application/json',
        'Connection': 'keep-alive'
      },
      // Enable HTTP/2 and connection reuse
      httpAgent: new http.Agent({
        keepAlive: true,
        maxSockets: this.config.maxConnections
      }),
      httpsAgent: new https.Agent({
        keepAlive: true,
        maxSockets: this.config.maxConnections
      })
    });
  }
  
  releaseConnection(connection) {
    connection.lastUsed = Date.now();
    
    if (this.requestQueue.length > 0) {
      const waitingRequest = this.requestQueue.shift();
      waitingRequest(connection);
    } else {
      this.connectionPool.push(connection);
    }
  }
  
  // ✅ Implement intelligent caching
  async callWithCache(method, params, cacheTtl = 30000) {
    const cacheKey = `${method}:${JSON.stringify(params)}`;
    
    // Check cache first
    const cached = this.cache.get(cacheKey);
    if (cached && Date.now() - cached.timestamp < cacheTtl) {
      return cached.data;
    }
    
    // Make actual call
    const result = await this.call(method, params);
    
    // Cache result
    this.cache.set(cacheKey, {
      data: result,
      timestamp: Date.now()
    });
    
    // Clean old cache entries periodically
    if (this.cache.size > 1000) {
      this.cleanCache();
    }
    
    return result;
  }
  
  cleanCache() {
    const now = Date.now();
    for (const [key, entry] of this.cache.entries()) {
      if (now - entry.timestamp > 60000) { // Remove entries older than 1 minute
        this.cache.delete(key);
      }
    }
  }
  
  // ✅ Implement retry logic with exponential backoff
  async callWithRetry(method, params, retryCount = 0) {
    try {
      return await this.call(method, params);
    } catch (error) {
      if (retryCount < this.config.retryAttempts && this.isRetryableError(error)) {
        const delay = this.config.retryDelay * Math.pow(2, retryCount);
        console.log(`Retrying ${method} in ${delay}ms (attempt ${retryCount + 1}/${this.config.retryAttempts})`);
        
        await this.sleep(delay);
        return this.callWithRetry(method, params, retryCount + 1);
      }
      
      throw error;
    }
  }
  
  isRetryableError(error) {
    // Retry on network errors, timeouts, and server errors
    return error.code === 'ECONNRESET' ||
           error.code === 'ETIMEDOUT' ||
           error.code === 'ECONNREFUSED' ||
           (error.response && error.response.status >= 500);
  }
  
  async call(method, params) {
    const connection = await this.getConnection();
    
    try {
      const response = await connection.client.post('/', {
        jsonrpc: '2.0',
        id: 'dontcare',
        method: 'call_function',
        params: {
          account_id: this.config.contractId,
          method_name: method,
          args_base64: Buffer.from(JSON.stringify(params)).toString('base64'),
          finality: 'final'
        }
      });
      
      return JSON.parse(Buffer.from(response.data.result.result, 'base64').toString());
    } finally {
      this.releaseConnection(connection);
    }
  }
  
  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}
```

### 2. Request Optimization

```javascript
class RequestOptimizer {
  constructor(client) {
    this.client = client;
    this.requestBatcher = new RequestBatcher();
    this.responseCache = new LRUCache({ max: 1000, ttl: 30000 });
  }
  
  // ✅ Batch multiple requests into single RPC call when possible
  async batchRequests(requests) {
    const batchableRequests = requests.filter(req => this.isBatchable(req.method));
    const individualRequests = requests.filter(req => !this.isBatchable(req.method));
    
    const results = new Map();
    
    // Process batchable requests together
    if (batchableRequests.length > 0) {
      const batchResults = await this.processBatch(batchableRequests);
      batchResults.forEach((result, id) => results.set(id, result));
    }
    
    // Process individual requests in parallel
    if (individualRequests.length > 0) {
      const individualResults = await Promise.allSettled(
        individualRequests.map(req => this.client.call(req.method, req.params))
      );
      
      individualRequests.forEach((req, index) => {
        const result = individualResults[index];
        results.set(req.id, result.status === 'fulfilled' ? result.value : result.reason);
      });
    }
    
    return results;
  }
  
  isBatchable(method) {
    // Methods that can be batched together for efficiency
    const batchableMethods = [
      'get_account',
      'get_balance',
      'get_validator',
      'simulate_tx'
    ];
    return batchableMethods.includes(method);
  }
  
  // ✅ Implement request deduplication
  async deduplicatedCall(method, params) {
    const requestKey = `${method}:${JSON.stringify(params)}`;
    
    // Check if same request is already in flight
    if (this.requestBatcher.hasPendingRequest(requestKey)) {
      return this.requestBatcher.waitForRequest(requestKey);
    }
    
    // Check cache
    const cached = this.responseCache.get(requestKey);
    if (cached) {
      return cached;
    }
    
    // Execute request
    const promise = this.client.call(method, params);
    this.requestBatcher.addPendingRequest(requestKey, promise);
    
    try {
      const result = await promise;
      this.responseCache.set(requestKey, result);
      return result;
    } finally {
      this.requestBatcher.removePendingRequest(requestKey);
    }
  }
}

class RequestBatcher {
  constructor() {
    this.pendingRequests = new Map();
  }
  
  hasPendingRequest(key) {
    return this.pendingRequests.has(key);
  }
  
  async waitForRequest(key) {
    const existingPromise = this.pendingRequests.get(key);
    return existingPromise;
  }
  
  addPendingRequest(key, promise) {
    this.pendingRequests.set(key, promise);
  }
  
  removePendingRequest(key) {
    this.pendingRequests.delete(key);
  }
}
```

## Infrastructure Optimizations

### 1. Load Balancing and Redundancy

```javascript
class LoadBalancedProximaClient {
  constructor(endpoints, options = {}) {
    this.endpoints = endpoints.map(endpoint => ({
      url: endpoint,
      weight: 1,
      active: true,
      responseTime: 0,
      errorCount: 0,
      lastHealthCheck: 0
    }));
    
    this.options = {
      healthCheckInterval: options.healthCheckInterval || 30000,
      maxRetries: options.maxRetries || 3,
      timeout: options.timeout || 10000,
      ...options
    };
    
    this.currentEndpointIndex = 0;
    this.startHealthChecks();
  }
  
  // ✅ Implement intelligent endpoint selection
  selectBestEndpoint() {
    const activeEndpoints = this.endpoints.filter(ep => ep.active);
    
    if (activeEndpoints.length === 0) {
      throw new Error('No active endpoints available');
    }
    
    // Weighted round-robin based on response time and error rate
    return activeEndpoints.reduce((best, current) => {
      const bestScore = this.calculateEndpointScore(best);
      const currentScore = this.calculateEndpointScore(current);
      return currentScore > bestScore ? current : best;
    });
  }
  
  calculateEndpointScore(endpoint) {
    const responseTimeFactor = Math.max(0, 1 - (endpoint.responseTime / 5000)); // Normalize to 5s max
    const errorRateFactor = Math.max(0, 1 - (endpoint.errorCount / 10)); // Normalize to 10 errors
    const healthFactor = endpoint.active ? 1 : 0;
    
    return (responseTimeFactor * 0.4 + errorRateFactor * 0.4 + healthFactor * 0.2);
  }
  
  // ✅ Implement automatic failover
  async callWithFailover(method, params) {
    let lastError;
    const maxAttempts = Math.min(this.options.maxRetries, this.endpoints.length);
    
    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      const endpoint = this.selectBestEndpoint();
      const startTime = Date.now();
      
      try {
        const client = new ProximaClient({
          rpcUrl: endpoint.url,
          timeout: this.options.timeout
        });
        
        const result = await client.call(method, params);
        
        // Update endpoint metrics on success
        endpoint.responseTime = Date.now() - startTime;
        endpoint.errorCount = Math.max(0, endpoint.errorCount - 1); // Gradually reduce error count
        
        return result;
      } catch (error) {
        lastError = error;
        endpoint.errorCount++;
        
        // Mark endpoint as inactive if too many errors
        if (endpoint.errorCount > 5) {
          endpoint.active = false;
          console.warn(`Marking endpoint ${endpoint.url} as inactive due to errors`);
        }
        
        console.log(`Attempt ${attempt + 1} failed for ${endpoint.url}:`, error.message);
      }
    }
    
    throw new Error(`All endpoints failed. Last error: ${lastError.message}`);
  }
  
  startHealthChecks() {
    setInterval(async () => {
      await this.performHealthChecks();
    }, this.options.healthCheckInterval);
  }
  
  async performHealthChecks() {
    const healthPromises = this.endpoints.map(async (endpoint) => {
      try {
        const client = new ProximaClient({ 
          rpcUrl: endpoint.url, 
          timeout: 5000 
        });
        
        const startTime = Date.now();
        await client.call('get_tx_config', {});
        
        // Endpoint is healthy
        endpoint.active = true;
        endpoint.responseTime = Date.now() - startTime;
        endpoint.errorCount = Math.max(0, endpoint.errorCount - 1);
        endpoint.lastHealthCheck = Date.now();
        
      } catch (error) {
        endpoint.errorCount++;
        if (endpoint.errorCount > 3) {
          endpoint.active = false;
        }
        console.warn(`Health check failed for ${endpoint.url}:`, error.message);
      }
    });
    
    await Promise.allSettled(healthPromises);
    
    const activeCount = this.endpoints.filter(ep => ep.active).length;
    console.log(`Health check completed. ${activeCount}/${this.endpoints.length} endpoints active`);
  }
}
```

### 2. Caching Strategies

```javascript
class MultilevelCache {
  constructor(options = {}) {
    this.memoryCache = new LRUCache({ 
      max: options.memoryCacheSize || 1000, 
      ttl: options.memoryCacheTtl || 30000 
    });
    
    this.redisClient = options.redisClient; // Optional Redis client
    this.cacheStats = {
      memoryHits: 0,
      redisHits: 0,
      misses: 0,
      sets: 0
    };
  }
  
  // ✅ Implement multilevel caching (memory -> Redis -> source)
  async get(key) {
    // Level 1: Memory cache
    let result = this.memoryCache.get(key);
    if (result !== undefined) {
      this.cacheStats.memoryHits++;
      return result;
    }
    
    // Level 2: Redis cache (if available)
    if (this.redisClient) {
      try {
        const redisResult = await this.redisClient.get(key);
        if (redisResult !== null) {
          result = JSON.parse(redisResult);
          this.memoryCache.set(key, result); // Promote to memory cache
          this.cacheStats.redisHits++;
          return result;
        }
      } catch (error) {
        console.warn('Redis cache error:', error.message);
      }
    }
    
    this.cacheStats.misses++;
    return undefined;
  }
  
  async set(key, value, options = {}) {
    const ttl = options.ttl || 30000;
    
    // Set in memory cache
    this.memoryCache.set(key, value, { ttl });
    
    // Set in Redis cache (if available)
    if (this.redisClient) {
      try {
        await this.redisClient.setex(key, Math.floor(ttl / 1000), JSON.stringify(value));
      } catch (error) {
        console.warn('Redis cache set error:', error.message);
      }
    }
    
    this.cacheStats.sets++;
  }
  
  // ✅ Implement cache warming for frequently accessed data
  async warmCache(warmingStrategies) {
    console.log('Starting cache warming...');
    
    const warmingPromises = warmingStrategies.map(async (strategy) => {
      try {
        await strategy.warm(this);
        console.log(`Cache warming completed for ${strategy.name}`);
      } catch (error) {
        console.error(`Cache warming failed for ${strategy.name}:`, error);
      }
    });
    
    await Promise.allSettled(warmingPromises);
    console.log('Cache warming completed');
  }
  
  getCacheStats() {
    const total = this.cacheStats.memoryHits + this.cacheStats.redisHits + this.cacheStats.misses;
    return {
      ...this.cacheStats,
      memoryHitRate: total > 0 ? (this.cacheStats.memoryHits / total) : 0,
      redisHitRate: total > 0 ? (this.cacheStats.redisHits / total) : 0,
      overallHitRate: total > 0 ? ((this.cacheStats.memoryHits + this.cacheStats.redisHits) / total) : 0
    };
  }
}

// Example cache warming strategies
const cacheWarmingStrategies = [
  {
    name: 'Popular Validators',
    async warm(cache) {
      const popularValidators = await getPopularValidators();
      for (const validator of popularValidators) {
        const key = `validator:${validator.address}`;
        await cache.set(key, validator, { ttl: 300000 }); // 5 minutes
      }
    }
  },
  {
    name: 'Active Accounts',
    async warm(cache) {
      const activeAccounts = await getActiveAccounts();
      for (const account of activeAccounts) {
        const key = `account:${account.address}`;
        await cache.set(key, account, { ttl: 60000 }); // 1 minute
      }
    }
  }
];
```

## Performance Monitoring

### 1. Comprehensive Metrics Collection

```javascript
class PerformanceMonitor {
  constructor() {
    this.metrics = {
      transactions: {
        total: 0,
        successful: 0,
        failed: 0,
        avgResponseTime: 0,
        responseTimeHistory: []
      },
      gas: {
        totalUsed: 0,
        avgPerTransaction: 0,
        efficiency: 0
      },
      network: {
        totalRequests: 0,
        avgLatency: 0,
        errorRate: 0
      },
      cache: {
        hitRate: 0,
        size: 0,
        evictions: 0
      }
    };
    
    this.startTime = Date.now();
    this.measurementInterval = null;
  }
  
  startMonitoring(intervalMs = 60000) {
    this.measurementInterval = setInterval(() => {
      this.collectMetrics();
      this.reportMetrics();
    }, intervalMs);
    
    console.log('Performance monitoring started');
  }
  
  stopMonitoring() {
    if (this.measurementInterval) {
      clearInterval(this.measurementInterval);
      this.measurementInterval = null;
    }
    console.log('Performance monitoring stopped');
  }
  
  recordTransaction(startTime, endTime, result, gasUsed) {
    const responseTime = endTime - startTime;
    
    this.metrics.transactions.total++;
    if (result.code === 0) {
      this.metrics.transactions.successful++;
    } else {
      this.metrics.transactions.failed++;
    }
    
    // Update response time metrics
    this.metrics.transactions.responseTimeHistory.push(responseTime);
    if (this.metrics.transactions.responseTimeHistory.length > 1000) {
      this.metrics.transactions.responseTimeHistory.shift();
    }
    
    this.metrics.transactions.avgResponseTime = this.calculateAverage(
      this.metrics.transactions.responseTimeHistory
    );
    
    // Update gas metrics
    if (gasUsed) {
      this.metrics.gas.totalUsed += gasUsed;
      this.metrics.gas.avgPerTransaction = this.metrics.gas.totalUsed / this.metrics.transactions.total;
    }
  }
  
  recordNetworkRequest(latency, isError = false) {
    this.metrics.network.totalRequests++;
    
    if (isError) {
      this.metrics.network.errorRate = 
        (this.metrics.network.errorRate * (this.metrics.network.totalRequests - 1) + 1) / 
        this.metrics.network.totalRequests;
    } else {
      this.metrics.network.avgLatency = 
        (this.metrics.network.avgLatency * (this.metrics.network.totalRequests - 1) + latency) / 
        this.metrics.network.totalRequests;
    }
  }
  
  updateCacheMetrics(cacheStats) {
    this.metrics.cache = {
      hitRate: cacheStats.overallHitRate,
      size: cacheStats.size || 0,
      evictions: cacheStats.evictions || 0
    };
  }
  
  collectMetrics() {
    // Collect system metrics
    const memoryUsage = process.memoryUsage();
    const uptime = Date.now() - this.startTime;
    
    return {
      timestamp: Date.now(),
      uptime: uptime,
      memory: {
        rss: memoryUsage.rss,
        heapUsed: memoryUsage.heapUsed,
        heapTotal: memoryUsage.heapTotal,
        external: memoryUsage.external
      },
      performance: this.metrics
    };
  }
  
  reportMetrics() {
    const metrics = this.collectMetrics();
    
    console.log('=== Performance Report ===');
    console.log(`Uptime: ${Math.floor(metrics.uptime / 1000)}s`);
    console.log(`Transactions: ${metrics.performance.transactions.total} (${metrics.performance.transactions.successful} success, ${metrics.performance.transactions.failed} failed)`);
    console.log(`Avg Response Time: ${metrics.performance.transactions.avgResponseTime.toFixed(2)}ms`);
    console.log(`Success Rate: ${((metrics.performance.transactions.successful / metrics.performance.transactions.total) * 100).toFixed(1)}%`);
    console.log(`Avg Gas/Tx: ${metrics.performance.gas.avgPerTransaction.toFixed(0)}`);
    console.log(`Network Error Rate: ${(metrics.performance.network.errorRate * 100).toFixed(2)}%`);
    console.log(`Cache Hit Rate: ${(metrics.performance.cache.hitRate * 100).toFixed(1)}%`);
    console.log(`Memory Usage: ${Math.round(metrics.memory.heapUsed / 1024 / 1024)}MB`);
    console.log('========================');
  }
  
  calculateAverage(array) {
    if (array.length === 0) return 0;
    return array.reduce((sum, val) => sum + val, 0) / array.length;
  }
  
  getPerformanceInsights() {
    const insights = [];
    
    // Analyze response times
    if (this.metrics.transactions.avgResponseTime > 1000) {
      insights.push({
        type: 'WARNING',
        category: 'RESPONSE_TIME',
        message: `Average response time is high: ${this.metrics.transactions.avgResponseTime.toFixed(2)}ms`,
        suggestion: 'Consider optimizing transaction processing or network configuration'
      });
    }
    
    // Analyze success rate
    const successRate = this.metrics.transactions.successful / this.metrics.transactions.total;
    if (successRate < 0.95) {
      insights.push({
        type: 'CRITICAL',
        category: 'SUCCESS_RATE',
        message: `Low success rate: ${(successRate * 100).toFixed(1)}%`,
        suggestion: 'Investigate transaction failures and improve error handling'
      });
    }
    
    // Analyze cache performance
    if (this.metrics.cache.hitRate < 0.7) {
      insights.push({
        type: 'INFO',
        category: 'CACHE_PERFORMANCE',
        message: `Low cache hit rate: ${(this.metrics.cache.hitRate * 100).toFixed(1)}%`,
        suggestion: 'Review caching strategy and TTL settings'
      });
    }
    
    return insights;
  }
}
```

### 2. Automated Performance Optimization

```javascript
class AutoPerformanceOptimizer {
  constructor(client, performanceMonitor) {
    this.client = client;
    this.monitor = performanceMonitor;
    this.optimizations = new Map();
    this.config = {
      responseTimeThreshold: 1000,
      successRateThreshold: 0.95,
      gasEfficiencyThreshold: 0.8
    };
  }
  
  async optimizeBasedOnMetrics() {
    const insights = this.monitor.getPerformanceInsights();
    
    for (const insight of insights) {
      await this.applyOptimization(insight);
    }
  }
  
  async applyOptimization(insight) {
    switch (insight.category) {
      case 'RESPONSE_TIME':
        await this.optimizeResponseTime();
        break;
      case 'SUCCESS_RATE':
        await this.optimizeSuccessRate();
        break;
      case 'CACHE_PERFORMANCE':
        await this.optimizeCaching();
        break;
    }
  }
  
  async optimizeResponseTime() {
    // Increase connection pool size
    if (!this.optimizations.has('connection_pool_increased')) {
      console.log('Optimizing: Increasing connection pool size');
      this.client.config.maxConnections = Math.min(
        this.client.config.maxConnections * 1.5, 
        50
      );
      this.optimizations.set('connection_pool_increased', Date.now());
    }
    
    // Reduce timeout for faster failover
    if (!this.optimizations.has('timeout_reduced')) {
      console.log('Optimizing: Reducing request timeout for faster failover');
      this.client.config.connectionTimeout = Math.max(
        this.client.config.connectionTimeout * 0.8,
        5000
      );
      this.optimizations.set('timeout_reduced', Date.now());
    }
  }
  
  async optimizeSuccessRate() {
    // Increase retry attempts
    if (!this.optimizations.has('retries_increased')) {
      console.log('Optimizing: Increasing retry attempts');
      this.client.config.retryAttempts = Math.min(
        this.client.config.retryAttempts + 1,
        5
      );
      this.optimizations.set('retries_increased', Date.now());
    }
    
    // Increase gas buffer for transactions
    if (!this.optimizations.has('gas_buffer_increased')) {
      console.log('Optimizing: Increasing gas buffer');
      // This would be applied to gas estimation logic
      this.optimizations.set('gas_buffer_increased', Date.now());
    }
  }
  
  async optimizeCaching() {
    // Increase cache size
    if (!this.optimizations.has('cache_size_increased')) {
      console.log('Optimizing: Increasing cache size');
      // This would be applied to cache configuration
      this.optimizations.set('cache_size_increased', Date.now());
    }
    
    // Extend cache TTL for stable data
    if (!this.optimizations.has('cache_ttl_extended')) {
      console.log('Optimizing: Extending cache TTL for stable data');
      this.optimizations.set('cache_ttl_extended', Date.now());
    }
  }
  
  // Revert optimizations if they don't improve performance
  async evaluateOptimizations() {
    const currentMetrics = this.monitor.collectMetrics();
    
    for (const [optimization, timestamp] of this.optimizations) {
      if (Date.now() - timestamp > 300000) { // 5 minutes
        const shouldRevert = await this.shouldRevertOptimization(optimization, currentMetrics);
        if (shouldRevert) {
          await this.revertOptimization(optimization);
        }
      }
    }
  }
  
  async shouldRevertOptimization(optimization, currentMetrics) {
    // Simple heuristic: if performance hasn't improved, revert
    const currentResponseTime = currentMetrics.performance.transactions.avgResponseTime;
    const currentSuccessRate = currentMetrics.performance.transactions.successful / 
                              currentMetrics.performance.transactions.total;
    
    return currentResponseTime > this.config.responseTimeThreshold || 
           currentSuccessRate < this.config.successRateThreshold;
  }
  
  async revertOptimization(optimization) {
    console.log(`Reverting optimization: ${optimization}`);
    this.optimizations.delete(optimization);
    
    // Revert the specific optimization
    switch (optimization) {
      case 'connection_pool_increased':
        this.client.config.maxConnections = Math.max(this.client.config.maxConnections / 1.5, 5);
        break;
      case 'timeout_reduced':
        this.client.config.connectionTimeout = Math.min(this.client.config.connectionTimeout / 0.8, 60000);
        break;
      // ... other reversions
    }
  }
}
```

## Best Practices Summary

### Contract-Level Best Practices

1. **Minimize storage operations**: Batch updates and use efficient data structures
2. **Optimize computations**: Cache expensive calculations and use early returns
3. **Efficient gas usage**: Profile gas consumption and optimize hot paths
4. **Memory management**: Use appropriate NEAR SDK collections for different use cases
5. **Error handling**: Fast-fail on validation errors before expensive operations

### Transaction-Level Best Practices

1. **Batch operations**: Combine related messages into single transactions
2. **Optimize message order**: Process simpler operations first
3. **Parallel processing**: Process independent transactions concurrently
4. **Smart sequencing**: Handle account sequences efficiently for concurrent transactions
5. **Gas optimization**: Use simulation for accurate gas estimation

### Client-Level Best Practices

1. **Connection pooling**: Reuse connections and implement proper connection management
2. **Request batching**: Batch multiple API calls when possible
3. **Intelligent caching**: Implement multilevel caching with appropriate TTLs
4. **Retry logic**: Implement exponential backoff for retryable errors
5. **Load balancing**: Use multiple endpoints with health checks and failover

### Infrastructure Best Practices

1. **Redundancy**: Deploy multiple RPC endpoints for high availability
2. **Monitoring**: Implement comprehensive performance monitoring and alerting
3. **Auto-scaling**: Scale infrastructure based on load patterns
4. **Geographic distribution**: Use geographically distributed endpoints for global users
5. **Regular optimization**: Continuously monitor and optimize based on metrics

### Monitoring Best Practices

1. **Comprehensive metrics**: Track transaction success rates, response times, and gas usage
2. **Real-time alerting**: Set up alerts for performance degradation
3. **Performance baselines**: Establish performance baselines and track improvements
4. **Regular reporting**: Generate regular performance reports for analysis
5. **Automated optimization**: Implement automated performance optimization where possible

This performance tuning guide provides a comprehensive framework for optimizing Proxima performance across all layers of the system, from smart contract code to client implementations and infrastructure management.