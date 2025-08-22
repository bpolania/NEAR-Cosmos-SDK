# Configuration Management Guide

This guide covers configuration management for the Proxima smart contract, including runtime parameter updates, deployment configurations, and operational best practices.

## Overview

Proxima uses a flexible configuration system that allows runtime updates of critical transaction processing parameters without requiring contract redeployment. This enables dynamic adjustments for network conditions, security requirements, and operational needs.

## Configuration Structure

### TxProcessingConfig

The core configuration object manages all transaction processing parameters:

```rust
pub struct TxProcessingConfig {
    pub chain_id: String,           // Chain identifier for signature verification
    pub max_gas_per_tx: u64,        // Maximum gas limit per transaction
    pub gas_price: u128,            // Gas price in NEAR tokens (yoctoNEAR)
    pub verify_signatures: bool,    // Enable/disable signature verification
    pub check_sequences: bool       // Enable/disable sequence number validation
}
```

### Default Configuration

```json
{
  "chain_id": "proxima-testnet-1",
  "max_gas_per_tx": 2000000,
  "gas_price": 1,
  "verify_signatures": true,
  "check_sequences": true
}
```

## Configuration Management

### Retrieving Current Configuration

Use the `get_tx_config()` public method to retrieve current settings:

```javascript
// JavaScript/TypeScript
async function getCurrentConfig(client) {
  const response = await client.call('get_tx_config', {});
  return response;
}

// Example response
{
  "chain_id": "proxima-testnet-1",
  "max_gas_per_tx": 2000000,
  "gas_price": 1,
  "verify_signatures": true,
  "check_sequences": true
}
```

```go
// Go
func getCurrentConfig(client *ProximaClient) (*TxProcessingConfig, error) {
    result, err := client.Call("get_tx_config", map[string]interface{}{})
    if err != nil {
        return nil, err
    }
    
    var config TxProcessingConfig
    err = json.Unmarshal(result, &config)
    return &config, err
}
```

```python
# Python
def get_current_config(client):
    result = client.call("get_tx_config", {})
    return result
```

```rust
// Rust
async fn get_current_config(client: &ProximaClient) -> Result<TxProcessingConfig, Box<dyn std::error::Error>> {
    let response = client.call("get_tx_config", serde_json::Value::Null).await?;
    let config: TxProcessingConfig = serde_json::from_value(response)?;
    Ok(config)
}
```

### Updating Configuration

Use the `update_tx_config()` method to update configuration parameters at runtime:

```javascript
// JavaScript/TypeScript
async function updateConfig(client, newConfig) {
  const result = await client.call('update_tx_config', {
    config: newConfig
  });
  console.log('Configuration updated successfully');
}

// Example usage
const newConfig = {
  chain_id: "proxima-mainnet-1",
  max_gas_per_tx: 5000000,
  gas_price: 2,
  verify_signatures: true,
  check_sequences: true
};

await updateConfig(client, newConfig);
```

```go
// Go
func updateConfig(client *ProximaClient, config TxProcessingConfig) error {
    args := map[string]interface{}{
        "config": config,
    }
    
    _, err := client.Call("update_tx_config", args)
    if err != nil {
        return fmt.Errorf("failed to update config: %w", err)
    }
    
    fmt.Println("Configuration updated successfully")
    return nil
}
```

```python
# Python
def update_config(client, new_config):
    result = client.call("update_tx_config", {"config": new_config})
    print("Configuration updated successfully")
    return result
```

```rust
// Rust
async fn update_config(
    client: &ProximaClient, 
    config: TxProcessingConfig
) -> Result<(), Box<dyn std::error::Error>> {
    let args = serde_json::json!({ "config": config });
    client.call("update_tx_config", args).await?;
    println!("Configuration updated successfully");
    Ok(())
}
```

## Configuration Parameters

### Chain ID (`chain_id`)

**Purpose**: Unique identifier for the blockchain network used in signature verification and transaction validation.

**Impact**: 
- **Critical for security**: Prevents cross-chain signature replay attacks
- **Must match client expectations**: Incorrect chain ID causes all signature verifications to fail
- **Network identification**: Distinguishes between testnet, mainnet, and development environments

**Valid Values**:
- `"proxima-testnet-1"` - Testnet environment
- `"proxima-mainnet-1"` - Mainnet environment  
- `"proxima-devnet-1"` - Development environment
- Custom values for private networks

**Update Considerations**:
- **Breaking change**: Updating chain ID invalidates all pending transactions
- **Coordinate with clients**: All clients must update to new chain ID simultaneously
- **Plan maintenance window**: Brief downtime may be required for coordination

**Example Update**:
```json
{
  "chain_id": "proxima-mainnet-1",
  "max_gas_per_tx": 2000000,
  "gas_price": 1,
  "verify_signatures": true,
  "check_sequences": true
}
```

### Maximum Gas Per Transaction (`max_gas_per_tx`)

**Purpose**: Upper limit on gas consumption per individual transaction to prevent resource exhaustion attacks.

**Impact**:
- **DoS protection**: Prevents single transactions from consuming excessive resources
- **Network stability**: Ensures fair resource allocation across all users
- **Transaction rejection**: Transactions exceeding limit are rejected with error code 12

**Recommended Values**:
- **Simple transfers**: 200,000 - 500,000 gas
- **Staking operations**: 300,000 - 800,000 gas
- **Multi-message transactions**: 800,000 - 2,000,000 gas  
- **Complex operations**: 2,000,000 - 10,000,000 gas

**Dynamic Adjustment Strategy**:
```javascript
// Monitor average gas usage and adjust limits
async function adjustGasLimits(client, historicalData) {
  const avgGasUsage = calculateAverageGasUsage(historicalData);
  const p95GasUsage = calculatePercentile(historicalData, 95);
  
  // Set limit to allow 95% of transactions with 50% buffer
  const newLimit = Math.ceil(p95GasUsage * 1.5);
  
  const config = await client.call('get_tx_config', {});
  config.max_gas_per_tx = newLimit;
  
  await client.call('update_tx_config', { config });
  console.log(`Updated gas limit to ${newLimit} based on usage patterns`);
}
```

### Gas Price (`gas_price`)

**Purpose**: Base price per unit of gas in yoctoNEAR (10^-24 NEAR) for fee calculation.

**Impact**:
- **Fee calculation**: `total_fee = gas_used × gas_price`
- **Economic incentives**: Higher prices can prioritize transactions during congestion
- **Cost management**: Affects transaction costs for all users

**Units**: yoctoNEAR (1 NEAR = 10^24 yoctoNEAR)

**Example Values**:
- `1` - 1 yoctoNEAR per gas (very low cost)
- `1000` - 1000 yoctoNEAR per gas (moderate cost)
- `1000000` - 0.001 NEAR per million gas units

**Dynamic Pricing Strategy**:
```javascript
// Implement congestion-based pricing
async function adjustGasPrice(client, networkMetrics) {
  const currentConfig = await client.call('get_tx_config', {});
  let newGasPrice = currentConfig.gas_price;
  
  if (networkMetrics.txPoolSize > 1000) {
    // High congestion - increase price by 50%
    newGasPrice = Math.ceil(currentConfig.gas_price * 1.5);
  } else if (networkMetrics.txPoolSize < 100) {
    // Low congestion - decrease price gradually
    newGasPrice = Math.max(1, Math.floor(currentConfig.gas_price * 0.9));
  }
  
  if (newGasPrice !== currentConfig.gas_price) {
    currentConfig.gas_price = newGasPrice;
    await client.call('update_tx_config', { config: currentConfig });
    console.log(`Adjusted gas price to ${newGasPrice} based on network congestion`);
  }
}
```

### Signature Verification (`verify_signatures`)

**Purpose**: Controls whether cryptographic signature validation is performed on transactions.

**Impact**:
- **Security**: Disabling verification removes authentication layer
- **Performance**: Verification adds computational overhead
- **Testing**: Can be disabled for testing environments

**Values**:
- `true` - **Production setting**: Full signature verification enabled
- `false` - **Testing only**: Signatures ignored (**INSECURE**)

**Use Cases**:
```javascript
// Production configuration
const productionConfig = {
  verify_signatures: true,  // Always enable for production
  check_sequences: true,
  // ... other settings
};

// Testing configuration (development only)
const testingConfig = {
  verify_signatures: false, // TESTING ONLY - NEVER in production
  check_sequences: false,   // May also disable for easier testing
  // ... other settings
};
```

**Security Warning**: 
> **NEVER disable signature verification in production environments**. This completely removes transaction authentication and allows unauthorized transaction execution.

### Sequence Number Validation (`check_sequences`)

**Purpose**: Controls whether account sequence number validation is enforced to prevent replay attacks.

**Impact**:
- **Replay protection**: Prevents transaction replay attacks
- **Order enforcement**: Ensures transactions execute in correct order
- **User experience**: Strict ordering may complicate concurrent transaction handling

**Values**:
- `true` - **Recommended**: Full sequence validation (prevents replay attacks)
- `false` - **Special cases**: Disabled validation (**reduces security**)

**Sequence Management Strategies**:
```javascript
// Strict sequence management (recommended)
class SequenceManager {
  constructor() {
    this.accountSequences = new Map();
  }
  
  async getNextSequence(address) {
    if (!this.accountSequences.has(address)) {
      // Query current sequence from network
      const account = await this.queryAccount(address);
      this.accountSequences.set(address, account.sequence);
    }
    
    const currentSequence = this.accountSequences.get(address);
    this.accountSequences.set(address, currentSequence + 1);
    return currentSequence;
  }
  
  handleFailedTransaction(address, sequence) {
    // Reset sequence on failure for retry
    this.accountSequences.set(address, sequence);
  }
}

// Relaxed sequence management (when check_sequences = false)
class RelaxedSequenceManager {
  async getNextSequence(address) {
    // Always query fresh sequence from network
    const account = await this.queryAccount(address);
    return account.sequence;
  }
}
```

## Configuration Deployment Strategies

### Environment-Specific Configurations

#### Development Environment
```json
{
  "chain_id": "proxima-devnet-1",
  "max_gas_per_tx": 10000000,
  "gas_price": 1,
  "verify_signatures": false,
  "check_sequences": false
}
```
- **Higher gas limits** for complex testing
- **Minimal gas prices** for cost-free testing
- **Relaxed validation** for easier development

#### Testnet Environment
```json
{
  "chain_id": "proxima-testnet-1", 
  "max_gas_per_tx": 5000000,
  "gas_price": 100,
  "verify_signatures": true,
  "check_sequences": true
}
```
- **Production-like security** with full validation
- **Higher gas limits** than mainnet for testing edge cases
- **Low but non-zero prices** to simulate real costs

#### Mainnet Environment
```json
{
  "chain__id": "proxima-mainnet-1",
  "max_gas_per_tx": 2000000,
  "gas_price": 1000,
  "verify_signatures": true,
  "check_sequences": true
}
```
- **Conservative gas limits** for stability
- **Market-appropriate pricing**
- **Full security validation**

### Gradual Configuration Updates

```javascript
// Example: Gradually increase gas price during high congestion
class GradualConfigUpdater {
  constructor(client, updateIntervalMs = 60000) {
    this.client = client;
    this.updateInterval = updateIntervalMs;
    this.isUpdating = false;
  }
  
  async startGradualGasPriceIncrease(targetPrice, steps = 10) {
    if (this.isUpdating) return;
    this.isUpdating = true;
    
    try {
      const currentConfig = await this.client.call('get_tx_config', {});
      const currentPrice = currentConfig.gas_price;
      const priceStep = Math.ceil((targetPrice - currentPrice) / steps);
      
      for (let i = 0; i < steps; i++) {
        const newPrice = Math.min(targetPrice, currentPrice + (priceStep * (i + 1)));
        currentConfig.gas_price = newPrice;
        
        await this.client.call('update_tx_config', { config: currentConfig });
        console.log(`Updated gas price to ${newPrice} (step ${i + 1}/${steps})`);
        
        if (newPrice >= targetPrice) break;
        await this.sleep(this.updateInterval);
      }
    } finally {
      this.isUpdating = false;
    }
  }
  
  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}
```

## Monitoring and Alerting

### Configuration Change Monitoring

```javascript
class ConfigurationMonitor {
  constructor(client, checkIntervalMs = 30000) {
    this.client = client;
    this.checkInterval = checkIntervalMs;
    this.lastKnownConfig = null;
    this.alertCallbacks = [];
  }
  
  addAlertCallback(callback) {
    this.alertCallbacks.push(callback);
  }
  
  async startMonitoring() {
    setInterval(async () => {
      try {
        const currentConfig = await this.client.call('get_tx_config', {});
        
        if (this.lastKnownConfig && this.configChanged(currentConfig)) {
          this.alertCallbacks.forEach(callback => {
            callback('CONFIG_CHANGED', {
              old: this.lastKnownConfig,
              new: currentConfig,
              changes: this.getChanges(this.lastKnownConfig, currentConfig)
            });
          });
        }
        
        this.lastKnownConfig = currentConfig;
      } catch (error) {
        this.alertCallbacks.forEach(callback => {
          callback('CONFIG_QUERY_ERROR', { error: error.message });
        });
      }
    }, this.checkInterval);
  }
  
  configChanged(newConfig) {
    return JSON.stringify(this.lastKnownConfig) !== JSON.stringify(newConfig);
  }
  
  getChanges(oldConfig, newConfig) {
    const changes = {};
    for (const key in newConfig) {
      if (oldConfig[key] !== newConfig[key]) {
        changes[key] = { old: oldConfig[key], new: newConfig[key] };
      }
    }
    return changes;
  }
}

// Usage
const monitor = new ConfigurationMonitor(client);
monitor.addAlertCallback((event, data) => {
  if (event === 'CONFIG_CHANGED') {
    console.log('Configuration changed:', data.changes);
    // Send alert to monitoring system
    sendAlert(`Proxima config changed: ${JSON.stringify(data.changes)}`);
  }
});
monitor.startMonitoring();
```

### Performance Impact Monitoring

```javascript
class PerformanceMonitor {
  constructor(client) {
    this.client = client;
    this.metrics = {
      avgGasUsed: 0,
      txSuccessRate: 0,
      avgResponseTime: 0
    };
  }
  
  async analyzeConfigurationImpact() {
    const config = await this.client.call('get_tx_config', {});
    const recentMetrics = await this.collectRecentMetrics();
    
    const recommendations = [];
    
    // Gas limit analysis
    if (recentMetrics.avgGasUsed > config.max_gas_per_tx * 0.8) {
      recommendations.push({
        type: 'GAS_LIMIT_LOW',
        message: `Average gas usage (${recentMetrics.avgGasUsed}) is approaching limit (${config.max_gas_per_tx})`,
        suggestion: `Consider increasing max_gas_per_tx to ${Math.ceil(recentMetrics.avgGasUsed * 1.5)}`
      });
    }
    
    // Success rate analysis
    if (recentMetrics.txSuccessRate < 0.95) {
      recommendations.push({
        type: 'LOW_SUCCESS_RATE',
        message: `Transaction success rate is ${(recentMetrics.txSuccessRate * 100).toFixed(1)}%`,
        suggestion: 'Review error patterns and consider adjusting gas limits or pricing'
      });
    }
    
    return recommendations;
  }
  
  async collectRecentMetrics() {
    // Mock implementation - replace with actual metrics collection
    return {
      avgGasUsed: 150000,
      txSuccessRate: 0.98,
      avgResponseTime: 250
    };
  }
}
```

## Operational Procedures

### Configuration Update Checklist

1. **Pre-Update Validation**
   - [ ] Review current configuration and usage patterns
   - [ ] Validate new configuration values
   - [ ] Test configuration in development environment
   - [ ] Assess impact on existing clients and applications

2. **Impact Assessment**
   - [ ] Identify affected users and applications
   - [ ] Estimate transaction cost changes (if gas price modified)
   - [ ] Check for potential transaction failures (if limits reduced)
   - [ ] Plan rollback strategy if needed

3. **Communication**
   - [ ] Notify users of upcoming changes (if breaking)
   - [ ] Update documentation and integration guides
   - [ ] Coordinate with dependent applications
   - [ ] Schedule maintenance window if needed

4. **Update Execution**
   - [ ] Apply configuration changes during low-traffic period
   - [ ] Monitor system metrics immediately after update
   - [ ] Verify configuration applied correctly
   - [ ] Test critical functionality with new configuration

5. **Post-Update Monitoring**
   - [ ] Monitor transaction success rates
   - [ ] Track gas usage patterns
   - [ ] Watch for error rate increases
   - [ ] Collect user feedback and system metrics

### Emergency Configuration Procedures

#### Emergency Gas Limit Increase (DoS Protection)
```javascript
// Emergency procedure to handle sudden gas usage spike
async function emergencyGasLimitIncrease(client, newLimit) {
  console.log(`EMERGENCY: Increasing gas limit to ${newLimit}`);
  
  const currentConfig = await client.call('get_tx_config', {});
  currentConfig.max_gas_per_tx = newLimit;
  
  await client.call('update_tx_config', { config: currentConfig });
  
  // Immediate verification
  const updatedConfig = await client.call('get_tx_config', {});
  if (updatedConfig.max_gas_per_tx !== newLimit) {
    throw new Error('Emergency gas limit update failed!');
  }
  
  console.log(`EMERGENCY: Gas limit successfully updated to ${newLimit}`);
  
  // Send alerts
  sendEmergencyAlert(`Gas limit increased to ${newLimit} due to emergency`);
}
```

#### Emergency Price Adjustment (Congestion Control)
```javascript
async function emergencyPriceIncrease(client, multiplier = 2) {
  console.log(`EMERGENCY: Increasing gas price by ${multiplier}x`);
  
  const currentConfig = await client.call('get_tx_config', {});
  const newPrice = currentConfig.gas_price * multiplier;
  currentConfig.gas_price = newPrice;
  
  await client.call('update_tx_config', { config: currentConfig });
  
  console.log(`EMERGENCY: Gas price increased to ${newPrice}`);
  sendEmergencyAlert(`Gas price increased ${multiplier}x to ${newPrice} for congestion control`);
}
```

## Best Practices

### Configuration Management
1. **Version Control**: Track all configuration changes in version control
2. **Environment Consistency**: Maintain similar configurations across environments where possible
3. **Gradual Changes**: Implement large changes gradually to assess impact
4. **Monitoring**: Continuously monitor metrics after configuration changes
5. **Documentation**: Document all configuration changes with rationale

### Security Considerations
1. **Never disable security features** in production (`verify_signatures`, `check_sequences`)
2. **Use appropriate gas limits** to prevent DoS attacks
3. **Monitor for abuse** of relaxed settings in development environments
4. **Implement access controls** for configuration updates in production
5. **Audit configuration changes** for security implications

### Performance Optimization
1. **Right-size gas limits** based on actual usage patterns
2. **Monitor gas price impact** on transaction volume
3. **Adjust limits proactively** during high-traffic periods
4. **Use metrics-driven decisions** rather than arbitrary values
5. **Test configuration changes** thoroughly before production deployment

This configuration guide provides comprehensive coverage of all aspects of managing Proxima's runtime parameters, from basic updates to advanced operational procedures.