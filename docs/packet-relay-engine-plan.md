# Core Packet Relay Engine Development Plan

## Overview
The packet relay engine is the heart of the IBC relayer, responsible for detecting, transmitting, and tracking packets between NEAR and Cosmos chains. This component orchestrates the entire cross-chain communication flow.

## Architecture Design

### 1. Core Components Structure
```rust
crates/ibc-relayer/src/relay/
├── engine.rs          // Main relay engine orchestrator
├── packet.rs          // Packet lifecycle management
├── processor.rs       // Packet processing logic
├── tracker.rs         // Enhanced packet tracking (existing)
├── proof.rs           // Proof generation and validation
└── strategy.rs        // Relay strategies (ordered/unordered)
```

### 2. Key Data Structures

**Packet Processing Pipeline:**
```rust
pub struct RelayEngine {
    chains: HashMap<String, Box<dyn Chain>>,
    packet_processor: PacketProcessor,
    packet_tracker: PacketTracker,
    event_receiver: mpsc::Receiver<RelayEvent>,
    config: RelayConfig,
    metrics: RelayMetrics,
}

pub struct PacketProcessor {
    proof_generator: ProofGenerator,
    tx_builder: TransactionBuilder,
    retry_policy: RetryPolicy,
}

pub enum PacketState {
    Detected,
    ProofGenerated,
    Submitted,
    Confirmed,
    Acknowledged,
    TimedOut,
    Failed(String),
}
```

## Implementation Steps

### Phase 1: Core Engine Structure (Day 1)

**1.1 Enhance Relay Engine (`engine.rs`)**
- Extend existing `RelayEngine` with packet processing loop
- Implement main relay algorithm with event-driven state machine
- Add bidirectional packet flow management
- Integrate with existing packet tracker

**1.2 Packet Lifecycle Management (`packet.rs`)**
- Create `PacketLifecycle` struct to manage packet states
- Implement state transitions with validation
- Add packet metadata tracking (timestamps, retries, errors)
- Build packet queue management for concurrent processing

**1.3 Packet Processing Logic (`processor.rs`)**
- Implement `process_send_packet()` for NEAR → Cosmos flow
- Implement `process_recv_packet()` for Cosmos → NEAR flow
- Add acknowledgment packet handling
- Build timeout packet processing

### Phase 2: Proof Generation & Validation (Day 2)

**2.1 Proof Generator (`proof.rs`)**
- Create abstraction for chain-specific proof generation
- Implement mock proof generation for development
- Add proof caching mechanism
- Build proof validation pipeline

**2.2 Transaction Builder**
- Create IBC transaction templates for both chains
- Implement gas estimation logic
- Add transaction signing abstraction
- Build batch transaction support

**2.3 Integration Points**
- Connect proof generator with chain implementations
- Wire up transaction submission to chain interfaces
- Add metrics collection for proof generation

### Phase 3: Relay Strategies & Optimization (Day 3)

**3.1 Relay Strategies (`strategy.rs`)**
- Implement ordered channel relay strategy
- Implement unordered channel relay strategy
- Add packet prioritization logic
- Build congestion control mechanisms

**3.2 Error Handling & Recovery**
- Implement retry policy with exponential backoff
- Add circuit breaker for failing chains
- Build dead letter queue for problematic packets
- Create recovery mechanisms for partial failures

**3.3 Performance Optimization**
- Implement concurrent packet processing
- Add packet batching for efficiency
- Build proof caching and reuse
- Optimize database queries

## Technical Implementation Details

### 1. Main Relay Loop
```rust
impl RelayEngine {
    pub async fn run(&mut self) -> Result<(), RelayError> {
        loop {
            tokio::select! {
                Some(event) = self.event_receiver.recv() => {
                    self.handle_relay_event(event).await?;
                }
                _ = self.process_pending_packets() => {
                    // Process queued packets
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => {
                    // Periodic tasks
                }
            }
        }
    }
    
    async fn handle_relay_event(&mut self, event: RelayEvent) -> Result<(), RelayError> {
        match event {
            RelayEvent::PacketDetected { chain_id, packet, .. } => {
                self.queue_packet_for_relay(chain_id, packet).await?;
            }
            RelayEvent::PacketConfirmed { packet_key } => {
                self.handle_packet_confirmation(packet_key).await?;
            }
            // Handle other events...
        }
        Ok(())
    }
}
```

### 2. Packet Processing Flow
```
1. Detect packet on source chain
2. Generate proof of packet commitment
3. Build IBC transaction for destination chain
4. Submit transaction with proof
5. Monitor for confirmation
6. Process acknowledgment (if applicable)
7. Handle timeouts and failures
```

### 3. Key Algorithms

**Packet Relay Algorithm:**
```
FOR each pending packet:
    IF packet.state == Detected:
        proof = generate_proof(packet)
        packet.state = ProofGenerated
    
    IF packet.state == ProofGenerated:
        tx = build_transaction(packet, proof)
        result = submit_transaction(tx)
        packet.state = Submitted
    
    IF packet.state == Submitted:
        confirmation = check_confirmation(packet)
        IF confirmed:
            packet.state = Confirmed
            process_acknowledgment(packet)
```

## Testing Strategy

### Unit Tests
- Packet state machine transitions
- Proof generation mocking
- Transaction building logic
- Error handling scenarios

### Integration Tests
- End-to-end packet relay simulation
- Multi-packet concurrent processing
- Failure recovery testing
- Performance benchmarks

## Success Criteria

### 1. Functional Requirements
- Successfully relay packets bidirectionally
- Handle all packet lifecycle states
- Process acknowledgments and timeouts
- Recover from transient failures

### 2. Performance Requirements
- Process 100+ packets/second
- <5 second average relay time
- Concurrent processing of multiple channels
- Efficient proof generation and caching

### 3. Reliability Requirements
- 99.9% packet delivery success rate
- Automatic retry with backoff
- Graceful degradation under load
- Comprehensive error reporting

## Dependencies & Prerequisites

### 1. Existing Components
- Chain trait implementations (NEAR, Cosmos)
- Event system for packet detection
- Configuration management
- Metrics collection

### 2. External Dependencies
- No new crate dependencies needed
- Existing async runtime (tokio)
- Current error handling framework

## Risk Mitigation

### 1. Technical Risks
- **Proof generation complexity**: Start with mock proofs
- **Chain integration issues**: Use extensive error handling
- **Performance bottlenecks**: Design for concurrent processing

### 2. Mitigation Strategies
- Incremental development with testing
- Extensive logging and monitoring
- Fallback mechanisms for failures
- Performance profiling from day one

## Timeline Summary

- **Day 1**: Core engine structure and packet lifecycle
- **Day 2**: Proof generation and transaction building
- **Day 3**: Relay strategies and optimization

This plan provides a structured approach to implementing the core packet relay engine over 3 days, with clear milestones and testing strategies to ensure reliable cross-chain packet transmission.