// NEAR event parsing

use super::{EventParser, IbcEvent, IbcEventType};
use crate::chains::ChainEvent;

/// NEAR event parser for our Cosmos SDK contract
pub struct NearEventParser;

impl EventParser for NearEventParser {
    fn parse_event(&self, raw_event: &ChainEvent) -> Option<IbcEvent> {
        // TODO: Parse NEAR events from our contract
        // Events will be in the format emitted by our contract
        
        match raw_event.event_type.as_str() {
            "ibc_send_packet" => Some(IbcEvent {
                event_type: IbcEventType::SendPacket,
                chain_id: "near-testnet".to_string(),
                height: raw_event.height,
                tx_hash: raw_event.tx_hash.clone(),
                attributes: raw_event.attributes.iter().cloned().collect(),
            }),
            "ibc_recv_packet" => Some(IbcEvent {
                event_type: IbcEventType::RecvPacket,
                chain_id: "near-testnet".to_string(),
                height: raw_event.height,
                tx_hash: raw_event.tx_hash.clone(),
                attributes: raw_event.attributes.iter().cloned().collect(),
            }),
            _ => None,
        }
    }
}