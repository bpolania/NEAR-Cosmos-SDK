// Cosmos event parsing

use super::{EventParser, IbcEvent, IbcEventType};
use crate::chains::ChainEvent;

/// Cosmos event parser for standard IBC events
pub struct CosmosEventParser;

impl EventParser for CosmosEventParser {
    fn parse_event(&self, raw_event: &ChainEvent) -> Option<IbcEvent> {
        // TODO: Parse Cosmos IBC events
        // Events will be in standard Cosmos SDK format
        
        match raw_event.event_type.as_str() {
            "send_packet" => Some(IbcEvent {
                event_type: IbcEventType::SendPacket,
                chain_id: "cosmoshub-testnet".to_string(),
                height: raw_event.height,
                tx_hash: raw_event.tx_hash.clone(),
                attributes: raw_event.attributes.iter().cloned().collect(),
            }),
            "recv_packet" => Some(IbcEvent {
                event_type: IbcEventType::RecvPacket,
                chain_id: "cosmoshub-testnet".to_string(),
                height: raw_event.height,
                tx_hash: raw_event.tx_hash.clone(),
                attributes: raw_event.attributes.iter().cloned().collect(),
            }),
            _ => None,
        }
    }
}