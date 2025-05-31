use hdk::prelude::*;
use crate::{Signal, ChatMessagePayload}; // Assuming ChatMessagePayload is in lib.rs or imported there
use crate::player::get_all_player_pubkeys; // Import the new function

#[hdk_extern]
pub fn send_global_chat_message(content: String) -> ExternResult<()> {
    let my_agent_info = agent_info()?;
    let my_pub_key = my_agent_info.agent_latest_pubkey.clone();
    let now_timestamp = sys_time()?;

    let payload = ChatMessagePayload {
        timestamp: now_timestamp,
        sender: my_pub_key.clone(),
        content,
    };

    let signal = Signal::GlobalChatMessage(payload);

    // 1. Emit locally for sender's UI
    emit_signal(&signal)?;

    // 2. Get all player pubkeys
    let all_player_pubkeys = get_all_player_pubkeys(())?; // Call the new function

    // 3. Send to all other players via call_remote
    for target_agent_key in all_player_pubkeys {
        if target_agent_key != my_pub_key { // Don't send to self again via call_remote
            let _ = call_remote(
                target_agent_key.clone(),
                "ping_2_pong", // Zome name
                "receive_remote_signal".into(),
                None, // Unrestricted cap grant assumed for receive_remote_signal
                signal.clone() // Clone signal for each call
            );
            // Error handling for call_remote can be added if necessary,
            // but often chat messages are fire-and-forget.
            // Example logging if needed:
            // match result {
            //     Ok(_) => debug!("Successfully sent remote signal to {:?}", target_agent_key),
            //     Err(e) => warn!("Failed to send remote signal to {:?}: {:?}", target_agent_key, e),
            // }
        }
    }
    Ok(())
}
