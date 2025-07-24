// Event monitoring and parsing

pub mod near_events;
pub mod cosmos_events;

use serde::{Deserialize, Serialize};

/// Generic IBC event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IbcEvent {
    pub event_type: IbcEventType,
    pub chain_id: String,
    pub height: u64,
    pub tx_hash: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
}

/// IBC event types we care about for relaying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IbcEventType {
    SendPacket,
    RecvPacket,
    WriteAcknowledgement,
    AcknowledgePacket,
    TimeoutPacket,
    CreateClient,
    UpdateClient,
    ConnectionOpenInit,
    ConnectionOpenTry,
    ConnectionOpenAck,
    ConnectionOpenConfirm,
    ChannelOpenInit,
    ChannelOpenTry,
    ChannelOpenAck,
    ChannelOpenConfirm,
    ChannelCloseInit,
    ChannelCloseConfirm,
}

/// Event parser trait
pub trait EventParser {
    /// Parse raw chain event into IBC event
    fn parse_event(&self, raw_event: &crate::chains::ChainEvent) -> Option<IbcEvent>;
}