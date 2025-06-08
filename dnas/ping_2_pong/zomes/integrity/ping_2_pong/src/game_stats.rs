use hdk::prelude::*;
use holo_hash::{ActionHash, AgentPubKey};

#[hdk_entry_helper]
#[derive(Clone)]
pub struct GameStats {
    pub game_id: ActionHash,
    pub player_1: AgentPubKey,
    pub player_2: AgentPubKey,
    pub latency_ms: u64,
    pub time_to_write_score_ms: u64,
    pub time_to_read_score_ms: u64,
    pub created_at: Timestamp,
}
