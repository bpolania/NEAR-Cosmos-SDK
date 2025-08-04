# Gas Estimation and Optimization Guide

This guide provides comprehensive information about gas estimation, optimization techniques, and best practices for minimizing transaction costs in Proxima while ensuring reliable execution.

## Overview

Gas represents computational resources consumed during transaction execution. Proper gas estimation and optimization are crucial for:

- **Cost efficiency**: Minimizing transaction fees for users
- **Reliability**: Ensuring transactions have sufficient gas to complete
- **Network health**: Preventing resource waste and DoS attacks
- **User experience**: Providing predictable and reasonable transaction costs

## Gas Fundamentals

### Gas Model

Proxima follows the Cosmos SDK gas model with NEAR-specific adaptations:

```
Total Fee = Gas Used × Gas Price
```

- **Gas Used**: Actual computational units consumed
- **Gas Price**: Price per gas unit (configurable in contract)
- **Gas Limit**: Maximum gas allowed for transaction (user-specified)

### Gas Units and Pricing

**Gas Price Units**: yoctoNEAR (10^-24 NEAR)
- 1 NEAR = 10^24 yoctoNEAR
- Typical gas price: 1-1000 yoctoNEAR per gas unit

**Example Calculations**:
```javascript
// Gas calculation examples
const gasUsed = 150000;
const gasPrice = 100; // yoctoNEAR per gas

const totalFeeYocto = gasUsed * gasPrice; // 15,000,000 yoctoNEAR
const totalFeeNEAR = totalFeeYocto / 1e24; // 0.000015 NEAR

console.log(`Transaction fee: ${totalFeeNEAR} NEAR`);
```

## Gas Consumption Patterns

### Message Type Gas Usage

| Message Type | Typical Gas Range | Factors Affecting Gas |
|-------------|------------------|---------------------|
| **MsgSend** | 80,000 - 120,000 | Number of recipients, denomination validation |
| **MsgMultiSend** | 150,000 - 300,000 | Number of inputs/outputs, validation complexity |
| **MsgDelegate** | 200,000 - 350,000 | Validator existence checks, delegation calculations |
| **MsgUndelegate** | 250,000 - 400,000 | Unbonding queue updates, reward calculations |
| **MsgRedelegate** | 300,000 - 500,000 | Source/dest validator checks, complex state updates |
| **MsgVote** | 100,000 - 200,000 | Proposal existence checks, vote weight calculations |
| **MsgSubmitProposal** | 400,000 - 800,000 | Proposal validation, deposit handling |
| **MsgDeposit** | 150,000 - 250,000 | Proposal state updates, deposit calculations |

### Multi-Message Transaction Gas

Gas consumption for multi-message transactions:

```javascript
// Gas estimation for multi-message transactions
function estimateMultiMessageGas(messages) {
  let baseGas = 50000; // Base transaction processing
  let totalGas = baseGas;
  
  messages.forEach(message => {
    switch (message['@type']) {
      case '/cosmos.bank.v1beta1.MsgSend':
        totalGas += 100000;
        break;
      case '/cosmos.staking.v1beta1.MsgDelegate':
        totalGas += 275000;
        break;
      case '/cosmos.gov.v1beta1.MsgVote':
        totalGas += 150000;
        break;
      default:
        totalGas += 200000; // Default estimate
    }
  });
  
  // Add 20% complexity overhead for multi-message coordination
  return Math.ceil(totalGas * 1.2);
}

// Example usage
const messages = [
  { '@type': '/cosmos.bank.v1beta1.MsgSend' },
  { '@type': '/cosmos.staking.v1beta1.MsgDelegate' },
  { '@type': '/cosmos.gov.v1beta1.MsgVote' }
];

const estimatedGas = estimateMultiMessageGas(messages);
console.log(`Estimated gas: ${estimatedGas}`); // ~642,000 gas
```

## Gas Estimation Strategies

### 1. Simulation-Based Estimation (Recommended)

Always use `simulate_tx()` before broadcasting for accurate gas estimation:

```javascript
// JavaScript/TypeScript implementation
class GasEstimator {
  constructor(client) {
    this.client = client;
    this.gasBufferPercent = 20; // 20% safety buffer
  }
  
  async estimateGas(tx, includeBuffer = true) {
    try {
      // Encode transaction for simulation
      const txBytes = this.encodeTx(tx);
      
      // Simulate transaction
      const simulation = await this.client.simulateTx(txBytes);
      
      if (simulation.code !== 0) {
        throw new Error(`Simulation failed: ${simulation.raw_log}`);
      }
      
      const gasUsed = parseInt(simulation.gas_used);
      
      if (includeBuffer) {
        return Math.ceil(gasUsed * (1 + this.gasBufferPercent / 100));
      }
      
      return gasUsed;
    } catch (error) {
      console.error('Gas estimation failed:', error);
      // Fallback to conservative static estimate
      return this.getStaticEstimate(tx);
    }
  }
  
  async estimateWithRetry(tx, maxRetries = 3) {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        return await this.estimateGas(tx);
      } catch (error) {
        if (attempt === maxRetries) throw error;
        
        console.log(`Gas estimation attempt ${attempt} failed, retrying...`);
        await this.sleep(1000 * attempt);
      }
    }
  }
  
  getStaticEstimate(tx) {
    // Fallback static estimation based on message types
    let estimate = 50000; // Base transaction cost
    
    tx.body.messages.forEach(message => {
      const messageType = message['@type'];
      estimate += this.getMessageGasEstimate(messageType);
    });
    
    return Math.ceil(estimate * 1.5); // Conservative buffer
  }
  
  getMessageGasEstimate(messageType) {
    const estimates = {
      '/cosmos.bank.v1beta1.MsgSend': 100000,
      '/cosmos.bank.v1beta1.MsgMultiSend': 250000,
      '/cosmos.staking.v1beta1.MsgDelegate': 300000,
      '/cosmos.staking.v1beta1.MsgUndelegate': 350000,
      '/cosmos.staking.v1beta1.MsgRedelegate': 450000,
      '/cosmos.gov.v1beta1.MsgVote': 150000,
      '/cosmos.gov.v1beta1.MsgSubmitProposal': 600000,
      '/cosmos.gov.v1beta1.MsgDeposit': 200000
    };
    
    return estimates[messageType] || 250000; // Default estimate
  }
  
  encodeTx(tx) {
    return Buffer.from(JSON.stringify(tx)).toString('base64');
  }
  
  sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }
}

// Usage example
const gasEstimator = new GasEstimator(client);

async function sendTransactionWithOptimalGas(tx) {
  try {
    // Estimate gas with simulation
    const estimatedGas = await gasEstimator.estimateWithRetry(tx);
    
    // Update transaction with estimated gas
    tx.auth_info.fee.gas_limit = estimatedGas;
    
    console.log(`Using estimated gas limit: ${estimatedGas}`);
    
    // Broadcast transaction
    const result = await client.broadcastTxSync(encodeTx(tx));
    
    if (result.code === 0) {
      const actualGas = parseInt(result.gas_used);
      const efficiency = (actualGas / estimatedGas * 100).toFixed(1);
      console.log(`Transaction successful. Gas efficiency: ${efficiency}%`);
    }
    
    return result;
  } catch (error) {
    console.error('Transaction failed:', error);
    throw error;
  }
}
```

### 2. Historical Data-Based Estimation

Build gas estimation models from historical transaction data:

```javascript
class HistoricalGasEstimator {
  constructor() {
    this.gasHistory = new Map(); // messageType -> gas usage array
    this.loadHistoricalData();
  }
  
  recordGasUsage(messageType, gasUsed) {
    if (!this.gasHistory.has(messageType)) {
      this.gasHistory.set(messageType, []);
    }
    
    const history = this.gasHistory.get(messageType);
    history.push(gasUsed);
    
    // Keep only recent 1000 records
    if (history.length > 1000) {
      history.shift();
    }
  }
  
  estimateGasForMessage(messageType) {
    const history = this.gasHistory.get(messageType);
    if (!history || history.length === 0) {
      return this.getDefaultEstimate(messageType);
    }
    
    // Use 90th percentile for reliable estimation
    const sorted = [...history].sort((a, b) => a - b);
    const p90Index = Math.floor(sorted.length * 0.9);
    const p90Gas = sorted[p90Index];
    
    // Add 10% buffer to 90th percentile
    return Math.ceil(p90Gas * 1.1);
  }
  
  estimateGasForTransaction(messages) {
    let totalGas = 50000; // Base transaction cost
    
    messages.forEach(message => {
      const messageType = message['@type'];
      totalGas += this.estimateGasForMessage(messageType);
    });
    
    // Multi-message overhead (if more than one message)
    if (messages.length > 1) {
      totalGas = Math.ceil(totalGas * 1.15);
    }
    
    return totalGas;
  }
  
  getGasStatistics(messageType) {
    const history = this.gasHistory.get(messageType);
    if (!history || history.length === 0) {
      return null;
    }
    
    const sorted = [...history].sort((a, b) => a - b);
    return {
      count: history.length,
      min: sorted[0],
      max: sorted[sorted.length - 1],
      median: sorted[Math.floor(sorted.length / 2)],
      p90: sorted[Math.floor(sorted.length * 0.9)],
      p95: sorted[Math.floor(sorted.length * 0.95)],
      average: Math.round(history.reduce((sum, gas) => sum + gas, 0) / history.length)
    };
  }
  
  loadHistoricalData() {
    // Load from persistent storage
    // Implementation depends on storage solution
  }
  
  getDefaultEstimate(messageType) {
    // Fallback estimates when no historical data available
    const defaults = {
      '/cosmos.bank.v1beta1.MsgSend': 100000,
      '/cosmos.staking.v1beta1.MsgDelegate': 300000,
      // ... other message types
    };
    return defaults[messageType] || 200000;
  }
}
```

### 3. Adaptive Gas Estimation

Implement adaptive estimation that learns from recent network conditions:

```javascript
class AdaptiveGasEstimator {
  constructor(client) {
    this.client = client;
    this.baseEstimates = new Map();
    this.recentAdjustments = [];
    this.adaptationFactor = 0.1; // How quickly to adapt to changes
  }
  
  async estimateAdaptiveGas(tx) {
    // Get base estimate from simulation or static calculation
    const baseEstimate = await this.getBaseEstimate(tx);
    
    // Apply network condition adjustments
    const networkAdjustment = this.calculateNetworkAdjustment();
    
    // Apply recent transaction success rate adjustments
    const successRateAdjustment = this.calculateSuccessRateAdjustment();
    
    // Combine adjustments
    const totalAdjustment = networkAdjustment + successRateAdjustment;
    const adjustedEstimate = Math.ceil(baseEstimate * (1 + totalAdjustment));
    
    console.log(`Base: ${baseEstimate}, Network adj: ${networkAdjustment.toFixed(2)}, Success adj: ${successRateAdjustment.toFixed(2)}, Final: ${adjustedEstimate}`);
    
    return adjustedEstimate;
  }
  
  calculateNetworkAdjustment() {
    // Estimate network congestion based on recent transaction patterns
    // Higher congestion = higher gas requirements
    
    const recentFailures = this.getRecentGasFailures();
    if (recentFailures > 0.1) { // More than 10% gas failures
      return 0.3; // Increase gas by 30%
    } else if (recentFailures > 0.05) { // More than 5% gas failures
      return 0.15; // Increase gas by 15%
    }
    
    return 0; // No adjustment needed
  }
  
  calculateSuccessRateAdjustment() {
    // Adjust based on recent transaction success rates
    const recentTransactions = this.recentAdjustments.slice(-50); // Last 50 transactions
    if (recentTransactions.length < 10) return 0;
    
    const successRate = recentTransactions.filter(tx => tx.success).length / recentTransactions.length;
    
    if (successRate < 0.9) {
      return 0.25; // Low success rate, increase gas significantly
    } else if (successRate < 0.95) {
      return 0.1; // Moderate success rate, small increase
    } else if (successRate > 0.98) {
      return -0.05; // Very high success rate, can reduce slightly
    }
    
    return 0;
  }
  
  recordTransactionResult(estimatedGas, actualGas, success) {
    this.recentAdjustments.push({
      timestamp: Date.now(),
      estimatedGas,
      actualGas,
      success,
      efficiency: actualGas / estimatedGas
    });
    
    // Keep only recent records
    if (this.recentAdjustments.length > 200) {
      this.recentAdjustments.shift();
    }
  }
  
  getRecentGasFailures() {
    const recent = this.recentAdjustments.slice(-100); // Last 100 transactions
    const gasFailures = recent.filter(tx => !tx.success && tx.estimatedGas < tx.actualGas);
    return gasFailures.length / recent.length;
  }
  
  async getBaseEstimate(tx) {
    try {
      const simulation = await this.client.simulateTx(this.encodeTx(tx));
      return parseInt(simulation.gas_used);
    } catch (error) {
      return this.getStaticEstimate(tx);
    }
  }
  
  getStaticEstimate(tx) {
    // Static estimation fallback
    let estimate = 50000;
    tx.body.messages.forEach(msg => {
      estimate += this.getMessageEstimate(msg['@type']);
    });
    return estimate;
  }
  
  getMessageEstimate(messageType) {
    const estimates = {
      '/cosmos.bank.v1beta1.MsgSend': 100000,
      '/cosmos.staking.v1beta1.MsgDelegate': 300000,
      // ... other estimates
    };
    return estimates[messageType] || 200000;
  }
  
  encodeTx(tx) {
    return Buffer.from(JSON.stringify(tx)).toString('base64');
  }
}
```

## Optimization Techniques

### 1. Transaction Structure Optimization

#### Minimize Message Complexity

```javascript
// ❌ Inefficient: Multiple separate transfers
const inefficientTx = {
  body: {
    messages: [
      {
        '@type': '/cosmos.bank.v1beta1.MsgSend',
        from_address: 'cosmos1sender...',
        to_address: 'cosmos1recipient1...',
        amount: [{ denom: 'unear', amount: '1000000' }]
      },
      {
        '@type': '/cosmos.bank.v1beta1.MsgSend',
        from_address: 'cosmos1sender...',
        to_address: 'cosmos1recipient2...',
        amount: [{ denom: 'unear', amount: '2000000' }]
      }
    ]
  }
};

// ✅ Efficient: Single multi-send message
const efficientTx = {
  body: {
    messages: [{
      '@type': '/cosmos.bank.v1beta1.MsgMultiSend',
      inputs: [{
        address: 'cosmos1sender...',
        coins: [{ denom: 'unear', amount: '3000000' }]
      }],
      outputs: [
        {
          address: 'cosmos1recipient1...',
          coins: [{ denom: 'unear', amount: '1000000' }]
        },
        {
          address: 'cosmos1recipient2...',
          coins: [{ denom: 'unear', amount: '2000000' }]
        }
      ]
    }]
  }
};
```

#### Optimize Message Ordering

```javascript
// ✅ Optimize message order for efficiency
function optimizeMessageOrder(messages) {
  // Sort messages by estimated gas consumption (ascending)
  return messages.sort((a, b) => {
    const gasA = getMessageGasEstimate(a['@type']);
    const gasB = getMessageGasEstimate(b['@type']);
    return gasA - gasB;
  });
}

// Group related messages together
function groupRelatedMessages(messages) {
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
  
  // Return in optimal order
  return [
    ...grouped.bank,
    ...grouped.staking,
    ...grouped.governance,
    ...grouped.other
  ];
}
```

### 2. Smart Gas Limit Selection

```javascript
class SmartGasLimitSelector {
  constructor() {
    this.gasLimitStrategies = {
      conservative: 1.5,  // 50% buffer
      balanced: 1.2,      // 20% buffer
      aggressive: 1.1,    // 10% buffer
      minimal: 1.05       // 5% buffer
    };
  }
  
  selectOptimalGasLimit(simulatedGas, transactionContext) {
    const {
      priority,           // 'high', 'normal', 'low'
      networkCongestion,  // 'high', 'normal', 'low'
      userProfile,        // 'cost_sensitive', 'balanced', 'speed_focused'
      retryAttempt        // number of retry attempts
    } = transactionContext;
    
    let strategy = 'balanced'; // default
    
    // Adjust strategy based on context
    if (priority === 'high' || networkCongestion === 'high') {
      strategy = 'conservative';
    } else if (userProfile === 'cost_sensitive' && networkCongestion === 'low') {
      strategy = 'aggressive';
    } else if (retryAttempt > 0) {
      // Previous attempt failed, be more conservative
      strategy = retryAttempt > 2 ? 'conservative' : 'balanced';
    }
    
    const multiplier = this.gasLimitStrategies[strategy];
    const gasLimit = Math.ceil(simulatedGas * multiplier);
    
    console.log(`Selected ${strategy} strategy (${multiplier}x): ${gasLimit} gas`);
    return gasLimit;
  }
  
  adjustForMessageComplexity(baseGasLimit, messages) {
    let complexity = 0;
    
    messages.forEach(msg => {
      const type = msg['@type'];
      
      // Add complexity based on message type
      if (type.includes('MsgMultiSend')) {
        const outputs = msg.outputs?.length || 1;
        complexity += outputs * 0.1; // Each additional output adds complexity
      } else if (type.includes('MsgRedelegate')) {
        complexity += 0.3; // Redelegation is inherently complex
      } else if (type.includes('MsgSubmitProposal')) {
        complexity += 0.5; // Proposals are very complex
      }
    });
    
    const complexityMultiplier = 1 + Math.min(complexity, 0.5); // Cap at 50% increase
    return Math.ceil(baseGasLimit * complexityMultiplier);
  }
}

// Usage
const gasSelector = new SmartGasLimitSelector();

async function sendTransactionWithSmartGas(tx, context) {
  const simulatedGas = await estimateGas(tx);
  const smartGasLimit = gasSelector.selectOptimalGasLimit(simulatedGas, context);
  const adjustedGasLimit = gasSelector.adjustForMessageComplexity(smartGasLimit, tx.body.messages);
  
  tx.auth_info.fee.gas_limit = adjustedGasLimit;
  
  return await broadcastTransaction(tx);
}
```

### 3. Batch Transaction Optimization

```javascript
class BatchTransactionOptimizer {
  constructor(maxGasPerTx = 2000000) {
    this.maxGasPerTx = maxGasPerTx;
  }
  
  optimizeBatch(transactions) {
    // Group transactions by sender to optimize sequence handling
    const groupedBySender = this.groupBySender(transactions);
    
    const optimizedBatches = [];
    
    for (const [sender, senderTxs] of groupedBySender) {
      const batches = this.createOptimalBatches(senderTxs);
      optimizedBatches.push(...batches);
    }
    
    return optimizedBatches;
  }
  
  createOptimalBatches(transactions) {
    const batches = [];
    let currentBatch = [];
    let currentBatchGas = 50000; // Base transaction gas
    
    for (const tx of transactions) {
      const txGas = this.estimateTransactionGas(tx);
      
      // Check if adding this transaction would exceed gas limit
      if (currentBatchGas + txGas > this.maxGasPerTx && currentBatch.length > 0) {
        // Start new batch
        batches.push(this.createBatchTransaction(currentBatch));
        currentBatch = [tx];
        currentBatchGas = 50000 + txGas;
      } else {
        // Add to current batch
        currentBatch.push(tx);
        currentBatchGas += txGas;
      }
    }
    
    // Add final batch if not empty
    if (currentBatch.length > 0) {
      batches.push(this.createBatchTransaction(currentBatch));
    }
    
    return batches;
  }
  
  createBatchTransaction(transactions) {
    // Combine all messages from transactions into a single batch transaction
    const allMessages = [];
    let maxSequence = 0;
    let senderAddress = '';
    
    transactions.forEach(tx => {
      allMessages.push(...tx.body.messages);
      
      // Track sequence numbers
      const sequence = tx.auth_info.signer_infos[0].sequence;
      if (sequence > maxSequence) {
        maxSequence = sequence;
      }
      
      // Get sender address from first transaction
      if (!senderAddress) {
        senderAddress = tx.body.messages[0].from_address || 
                       tx.body.messages[0].delegator_address ||
                       tx.body.messages[0].voter;
      }
    });
    
    return {
      body: {
        messages: allMessages,
        memo: `Batch of ${transactions.length} transactions`,
        timeout_height: 0,
        extension_options: [],
        non_critical_extension_options: []
      },
      auth_info: {
        signer_infos: [{
          public_key: null,
          mode_info: { mode: 'Direct', multi: null },
          sequence: maxSequence
        }],
        fee: {
          amount: [{ denom: 'unear', amount: '2000' }], // Adjusted for batch
          gas_limit: this.estimateBatchGas(allMessages),
          payer: '',
          granter: ''
        }
      },
      signatures: []
    };
  }
  
  estimateBatchGas(messages) {
    let totalGas = 50000; // Base transaction cost
    
    messages.forEach(msg => {
      totalGas += this.getMessageGasEstimate(msg['@type']);
    });
    
    // Add batch processing overhead
    const batchOverhead = Math.ceil(messages.length * 0.05 * totalGas);
    return Math.min(totalGas + batchOverhead, this.maxGasPerTx);
  }
  
  groupBySender(transactions) {
    const grouped = new Map();
    
    transactions.forEach(tx => {
      const sender = this.extractSenderAddress(tx);
      if (!grouped.has(sender)) {
        grouped.set(sender, []);
      }
      grouped.get(sender).push(tx);
    });
    
    return grouped;
  }
  
  extractSenderAddress(tx) {
    const firstMessage = tx.body.messages[0];
    return firstMessage.from_address || 
           firstMessage.delegator_address ||
           firstMessage.voter ||
           'unknown';
  }
  
  estimateTransactionGas(tx) {
    let gas = 50000; // Base cost
    tx.body.messages.forEach(msg => {
      gas += this.getMessageGasEstimate(msg['@type']);
    });
    return gas;
  }
  
  getMessageGasEstimate(messageType) {
    const estimates = {
      '/cosmos.bank.v1beta1.MsgSend': 100000,
      '/cosmos.staking.v1beta1.MsgDelegate': 300000,
      // ... other estimates
    };
    return estimates[messageType] || 200000;
  }
}
```

## Gas Monitoring and Analytics

### 1. Gas Usage Analytics

```javascript
class GasAnalytics {
  constructor() {
    this.transactions = [];
    this.analytics = {
      totalTransactions: 0,
      totalGasUsed: 0,
      totalFeesSpent: 0,
      averageGasUsage: 0,
      gasEfficiency: 0,
      messageTypeStats: new Map()
    };
  }
  
  recordTransaction(tx, result) {
    const record = {
      timestamp: Date.now(),
      messageTypes: tx.body.messages.map(msg => msg['@type']),
      gasLimit: tx.auth_info.fee.gas_limit,
      gasUsed: parseInt(result.gas_used),
      gasPrice: this.extractGasPrice(tx),
      success: result.code === 0,
      efficiency: parseInt(result.gas_used) / tx.auth_info.fee.gas_limit
    };
    
    this.transactions.push(record);
    this.updateAnalytics(record);
    
    // Keep only recent transactions for memory management
    if (this.transactions.length > 10000) {
      this.transactions.shift();
    }
  }
  
  updateAnalytics(record) {
    this.analytics.totalTransactions++;
    this.analytics.totalGasUsed += record.gasUsed;
    this.analytics.totalFeesSpent += record.gasUsed * record.gasPrice;
    this.analytics.averageGasUsage = this.analytics.totalGasUsed / this.analytics.totalTransactions;
    this.analytics.gasEfficiency = this.calculateAverageEfficiency();
    
    // Update message type statistics
    record.messageTypes.forEach(type => {
      if (!this.analytics.messageTypeStats.has(type)) {
        this.analytics.messageTypeStats.set(type, {
          count: 0,
          totalGas: 0,
          averageGas: 0,
          minGas: Infinity,
          maxGas: 0
        });
      }
      
      const stats = this.analytics.messageTypeStats.get(type);
      stats.count++;
      stats.totalGas += record.gasUsed;
      stats.averageGas = stats.totalGas / stats.count;
      stats.minGas = Math.min(stats.minGas, record.gasUsed);
      stats.maxGas = Math.max(stats.maxGas, record.gasUsed);
    });
  }
  
  calculateAverageEfficiency() {
    if (this.transactions.length === 0) return 0;
    
    const totalEfficiency = this.transactions.reduce((sum, tx) => sum + tx.efficiency, 0);
    return totalEfficiency / this.transactions.length;
  }
  
  generateOptimizationReport() {
    const report = {
      summary: this.analytics,
      recommendations: [],
      inefficiencies: []
    };
    
    // Analyze gas efficiency
    const lowEfficiencyTxs = this.transactions.filter(tx => tx.efficiency < 0.7);
    if (lowEfficiencyTxs.length > this.transactions.length * 0.2) {
      report.recommendations.push({
        type: 'GAS_ESTIMATION',
        message: `${lowEfficiencyTxs.length} transactions (${(lowEfficiencyTxs.length / this.transactions.length * 100).toFixed(1)}%) had low gas efficiency (<70%)`,
        suggestion: 'Improve gas estimation accuracy or reduce gas buffers'
      });
    }
    
    // Analyze message type efficiency
    for (const [type, stats] of this.analytics.messageTypeStats) {
      if (stats.count >= 10) { // Only analyze types with sufficient data
        const efficiency = stats.totalGas / (stats.count * stats.maxGas);
        if (efficiency < 0.6) {
          report.inefficiencies.push({
            messageType: type,
            averageGas: stats.averageGas,
            maxGas: stats.maxGas,
            efficiency: efficiency,
            suggestion: `Consider optimizing ${type} transactions`
          });
        }
      }
    }
    
    return report;
  }
  
  extractGasPrice(tx) {
    const feeAmount = tx.auth_info.fee.amount[0];
    const gasLimit = tx.auth_info.fee.gas_limit;
    return parseInt(feeAmount.amount) / gasLimit;
  }
  
  getGasInsights(timeRangeHours = 24) {
    const cutoff = Date.now() - (timeRangeHours * 60 * 60 * 1000);
    const recentTxs = this.transactions.filter(tx => tx.timestamp >= cutoff);
    
    if (recentTxs.length === 0) return null;
    
    const gasUsages = recentTxs.map(tx => tx.gasUsed).sort((a, b) => a - b);
    
    return {
      timeRange: `${timeRangeHours} hours`,
      transactionCount: recentTxs.length,
      gasStatistics: {
        min: gasUsages[0],
        max: gasUsages[gasUsages.length - 1],
        median: gasUsages[Math.floor(gasUsages.length / 2)],
        p90: gasUsages[Math.floor(gasUsages.length * 0.9)],
        p95: gasUsages[Math.floor(gasUsages.length * 0.95)],
        average: Math.round(gasUsages.reduce((sum, gas) => sum + gas, 0) / gasUsages.length)
      },
      successRate: recentTxs.filter(tx => tx.success).length / recentTxs.length,
      averageEfficiency: recentTxs.reduce((sum, tx) => sum + tx.efficiency, 0) / recentTxs.length
    };
  }
}

// Usage
const gasAnalytics = new GasAnalytics();

// Record transactions
async function sendTrackedTransaction(tx) {
  const result = await client.broadcastTxSync(encodeTx(tx));
  gasAnalytics.recordTransaction(tx, result);
  
  // Periodically generate reports
  if (gasAnalytics.analytics.totalTransactions % 100 === 0) {
    const report = gasAnalytics.generateOptimizationReport();
    console.log('Gas Optimization Report:', report);
  }
  
  return result;
}
```

### 2. Real-time Gas Monitoring

```javascript
class RealTimeGasMonitor {
  constructor(alertThresholds = {}) {
    this.thresholds = {
      highGasUsage: 1500000,      // Alert if gas usage > 1.5M
      lowEfficiency: 0.6,         // Alert if efficiency < 60%
      highFailureRate: 0.1,       // Alert if >10% transactions fail
      ...alertThresholds
    };
    
    this.recentTransactions = [];
    this.alerts = [];
    this.isMonitoring = false;
  }
  
  startMonitoring(checkIntervalMs = 30000) {
    this.isMonitoring = true;
    
    setInterval(() => {
      if (this.isMonitoring) {
        this.performChecks();
      }
    }, checkIntervalMs);
    
    console.log('Real-time gas monitoring started');
  }
  
  stopMonitoring() {
    this.isMonitoring = false;
    console.log('Real-time gas monitoring stopped');
  }
  
  recordTransaction(tx, result) {
    const record = {
      timestamp: Date.now(),
      gasLimit: tx.auth_info.fee.gas_limit,
      gasUsed: parseInt(result.gas_used),
      success: result.code === 0,
      efficiency: parseInt(result.gas_used) / tx.auth_info.fee.gas_limit,
      messageCount: tx.body.messages.length
    };
    
    this.recentTransactions.push(record);
    
    // Keep only last hour of transactions
    const oneHourAgo = Date.now() - (60 * 60 * 1000);
    this.recentTransactions = this.recentTransactions.filter(
      tx => tx.timestamp >= oneHourAgo
    );
    
    // Immediate checks for this transaction
    this.checkTransaction(record);
  }
  
  checkTransaction(record) {
    // Check for unusually high gas usage
    if (record.gasUsed > this.thresholds.highGasUsage) {
      this.createAlert('HIGH_GAS_USAGE', {
        gasUsed: record.gasUsed,
        threshold: this.thresholds.highGasUsage,
        message: `Transaction used ${record.gasUsed} gas (threshold: ${this.thresholds.highGasUsage})`
      });
    }
    
    // Check for low efficiency
    if (record.efficiency < this.thresholds.lowEfficiency) {
      this.createAlert('LOW_EFFICIENCY', {
        efficiency: record.efficiency,
        threshold: this.thresholds.lowEfficiency,
        gasUsed: record.gasUsed,
        gasLimit: record.gasLimit,
        message: `Transaction efficiency ${(record.efficiency * 100).toFixed(1)}% (threshold: ${(this.thresholds.lowEfficiency * 100)}%)`
      });
    }
  }
  
  performChecks() {
    if (this.recentTransactions.length < 10) return; // Need minimum data
    
    // Check failure rate
    const failures = this.recentTransactions.filter(tx => !tx.success).length;
    const failureRate = failures / this.recentTransactions.length;
    
    if (failureRate > this.thresholds.highFailureRate) {
      this.createAlert('HIGH_FAILURE_RATE', {
        failureRate: failureRate,
        threshold: this.thresholds.highFailureRate,
        failureCount: failures,
        totalTransactions: this.recentTransactions.length,
        message: `High failure rate: ${(failureRate * 100).toFixed(1)}% (${failures}/${this.recentTransactions.length})`
      });
    }
    
    // Check for gas usage trends
    this.checkGasTrends();
  }
  
  checkGasTrends() {
    const recent = this.recentTransactions.slice(-20); // Last 20 transactions
    const older = this.recentTransactions.slice(-40, -20); // Previous 20 transactions
    
    if (recent.length < 10 || older.length < 10) return;
    
    const recentAvgGas = recent.reduce((sum, tx) => sum + tx.gasUsed, 0) / recent.length;
    const olderAvgGas = older.reduce((sum, tx) => sum + tx.gasUsed, 0) / older.length;
    
    const increaseRatio = recentAvgGas / olderAvgGas;
    
    if (increaseRatio > 1.5) { // 50% increase in gas usage
      this.createAlert('GAS_USAGE_SPIKE', {
        recentAverage: Math.round(recentAvgGas),
        previousAverage: Math.round(olderAvgGas),
        increaseRatio: increaseRatio,
        message: `Gas usage spike detected: ${Math.round(recentAvgGas)} vs ${Math.round(olderAvgGas)} (${((increaseRatio - 1) * 100).toFixed(1)}% increase)`
      });
    }
  }
  
  createAlert(type, data) {
    const alert = {
      type,
      timestamp: Date.now(),
      data,
      id: `${type}_${Date.now()}`
    };
    
    this.alerts.push(alert);
    
    // Keep only recent alerts
    if (this.alerts.length > 1000) {
      this.alerts.shift();
    }
    
    // Trigger alert callback
    this.onAlert(alert);
  }
  
  onAlert(alert) {
    console.warn(`⚠️  GAS ALERT [${alert.type}]:`, alert.data.message);
    
    // In production, you might send this to a monitoring service
    // sendToMonitoringService(alert);
  }
  
  getRecentAlerts(hours = 1) {
    const cutoff = Date.now() - (hours * 60 * 60 * 1000);
    return this.alerts.filter(alert => alert.timestamp >= cutoff);
  }
  
  getGasMetrics() {
    if (this.recentTransactions.length === 0) return null;
    
    const gasUsages = this.recentTransactions.map(tx => tx.gasUsed);
    const efficiencies = this.recentTransactions.map(tx => tx.efficiency);
    const successRate = this.recentTransactions.filter(tx => tx.success).length / this.recentTransactions.length;
    
    return {
      transactionCount: this.recentTransactions.length,
      gasUsage: {
        average: Math.round(gasUsages.reduce((sum, gas) => sum + gas, 0) / gasUsages.length),
        min: Math.min(...gasUsages),
        max: Math.max(...gasUsages)
      },
      efficiency: {
        average: efficiencies.reduce((sum, eff) => sum + eff, 0) / efficiencies.length,
        min: Math.min(...efficiencies),
        max: Math.max(...efficiencies)
      },
      successRate: successRate,
      alertCount: this.getRecentAlerts().length
    };
  }
}

// Usage
const gasMonitor = new RealTimeGasMonitor({
  highGasUsage: 1200000,
  lowEfficiency: 0.65,
  highFailureRate: 0.08
});

gasMonitor.startMonitoring();

// Integrate with transaction sending
async function sendMonitoredTransaction(tx) {
  const result = await client.broadcastTxSync(encodeTx(tx));
  gasMonitor.recordTransaction(tx, result);
  return result;
}
```

## Best Practices Summary

### Gas Estimation Best Practices

1. **Always simulate first**: Use `simulate_tx()` for accurate gas estimation
2. **Use appropriate buffers**: 10-20% buffer for most transactions
3. **Consider network conditions**: Increase buffers during high congestion
4. **Monitor and adjust**: Track gas efficiency and adjust estimation models
5. **Implement fallbacks**: Have static estimates for when simulation fails

### Transaction Optimization Best Practices

1. **Batch related operations**: Combine multiple messages when possible
2. **Optimize message order**: Process simpler messages first
3. **Use appropriate message types**: Choose most efficient message type for the operation
4. **Minimize transaction size**: Reduce unnecessary data in transactions
5. **Plan message complexity**: Consider computational requirements of each message

### Monitoring and Analytics Best Practices

1. **Track gas efficiency**: Monitor actual vs estimated gas usage
2. **Set up alerts**: Monitor for unusual gas patterns or high failure rates
3. **Analyze trends**: Look for patterns in gas usage over time
4. **Regular optimization**: Periodically review and optimize gas strategies
5. **User feedback**: Consider user experience and cost sensitivity

### Cost Management Best Practices

1. **Understand pricing**: Monitor gas price changes and network conditions
2. **Offer options**: Provide different speed/cost options for users
3. **Educate users**: Help users understand gas costs and optimization
4. **Batch operations**: Encourage batching for cost efficiency
5. **Dynamic pricing**: Adjust strategies based on network congestion

This comprehensive gas optimization guide provides the tools and strategies needed to minimize transaction costs while ensuring reliable execution in the Proxima ecosystem.