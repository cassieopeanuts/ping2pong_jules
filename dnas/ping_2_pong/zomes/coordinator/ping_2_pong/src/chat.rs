use hdk::prelude::*;
use crate::{Signal, ChatMessagePayload}; // Assuming ChatMessagePayload is in lib.rs or imported there

#[hdk_extern]
pub fn send_global_chat_message(content: String) -> ExternResult<()> {
    let agent_info = agent_info()?;
    let now_timestamp = sys_time()?; // hdk::prelude::holo_hash::Timestamp doesn't seem to have a `now()` or equivalent. Using sys_time() which returns a `Timestamp`.

    let payload = ChatMessagePayload {
        timestamp: now_timestamp,
        sender: agent_info.agent_latest_pubkey,
        content,
    };

    let signal = Signal::GlobalChatMessage(payload);
    emit_signal(&signal)?;
    Ok(())
}
