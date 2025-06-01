// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/lib.rs

// Declare modules
pub mod chat;
pub mod game;
pub mod player;
pub mod score;
pub mod statistics;
pub mod utils;
pub mod signals;
pub mod invitations;

use hdk::prelude::*;
// Use integrity crate types (EntryTypes, LinkTypes)
use ping_2_pong_integrity::*;

// Import Timestamp and AgentPubKey for ChatMessagePayload
use hdk::prelude::{Timestamp, AgentPubKey};


/// ---------- 1. grant the capability on startup ----------
#[hdk_extern]
fn init(_: ()) -> ExternResult<InitCallbackResult> {
    // everybody can call `receive_remote_signal`
    let grant = CapGrantEntry {
        tag: "remote-signal".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(
            vec![("ping_2_pong".into(), "receive_remote_signal".into())]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
    };
    create_cap_grant(grant)?;
    Ok(InitCallbackResult::Pass)
}


// ChatMessagePayload struct definition
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessagePayload {
    pub timestamp: Timestamp,
    pub sender: AgentPubKey,
    pub content: String,
}

// Signal enum definition
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Signal {
    // Standard Holochain signals
    LinkCreated { action: SignedActionHashed, link_type: LinkTypes },
    LinkDeleted { action: SignedActionHashed, create_link_action: SignedActionHashed, link_type: LinkTypes },
    EntryCreated { action: SignedActionHashed, app_entry: EntryTypes },
    EntryUpdated { action: SignedActionHashed, app_entry: EntryTypes, original_app_entry: EntryTypes },
    EntryDeleted { action: SignedActionHashed, original_app_entry: EntryTypes },

    // Custom application signals
    GameInvitation {
        game_id: ActionHash,
        inviter: AgentPubKey,
        message: String,
    },
    // *** MODIFIED GameStarted to include both players ***
    GameStarted {
        game_id: ActionHash,
        player_1: AgentPubKey, // Player 1's public key
        player_2: AgentPubKey, // Player 2's public key (the joiner)
    },
    PaddleUpdate {
        game_id: ActionHash,
        player: AgentPubKey,
        relative_paddle_y: f32, // Changed from paddle_y: u32
    },
    BallUpdate {
        game_id: ActionHash,
        relative_ball_x: f32, // Changed from ball_x: u32
        relative_ball_y: f32, // Changed from ball_y: u32
        ball_dx: i32,
        ball_dy: i32,
    },
    ScoreUpdate {
        game_id: ActionHash,
        score1:  u32,
        score2:  u32,
    },
    GameOver {
        game_id: ActionHash,
        winner: Option<AgentPubKey>,
        score1: u32,
        score2: u32,
    },
    GlobalChatMessage(ChatMessagePayload),
}

// post_commit hook (no changes needed here)
#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}

// signal_action helper (no changes needed here)
fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
     // ... (keep existing implementation) ...
     match action.action().clone() {
        Action::CreateLink(create_link) => {
            if let Ok(Some(link_type)) = LinkTypes::from_type(create_link.zome_index, create_link.link_type) {
                emit_signal(Signal::LinkCreated { action, link_type })?;
            }
            Ok(())
        }
        Action::DeleteLink(delete_link) => {
            match get(delete_link.link_add_address.clone(), GetOptions::default())? {
                Some(record) => {
                    if let Action::CreateLink(create_link) = record.action().clone() {
                        if let Ok(Some(link_type)) = LinkTypes::from_type(create_link.zome_index, create_link.link_type) {
                            emit_signal(Signal::LinkDeleted {
                                action,
                                link_type,
                                create_link_action: record.signed_action.clone(),
                            })?;
                        }
                    } else { warn!("Original action for DeleteLink signal is not CreateLink: {:?}", delete_link.link_add_address); }
                }
                None => { warn!("Could not find matching CreateLink record for DeleteLink signal: {:?}", delete_link.link_add_address); }
            }
            Ok(())
        }
        Action::Create(create) => {
            if let EntryType::App(_) = create.entry_type {
                 match get_entry_for_action(&action.hashed.hash) {
                    Ok(Some(app_entry)) => { emit_signal(Signal::EntryCreated { action, app_entry })?; },
                    Ok(None) => { debug!("Could not get entry for signal EntryCreated: {}", action.hashed.hash); },
                    Err(e) => { error!("Error getting entry for signal EntryCreated: {:?}", e); }
                 }
             }
            Ok(())
        }
        Action::Update(update) => {
             let maybe_app_entry = get_entry_for_action(&action.hashed.hash);
             let maybe_original_app_entry = get_entry_for_action(&update.original_action_address);
             match (maybe_app_entry, maybe_original_app_entry) {
                 (Ok(Some(app_entry)), Ok(Some(original_app_entry))) => {
                      emit_signal(Signal::EntryUpdated { action, app_entry, original_app_entry, })?;
                 },
                 (Ok(_), Ok(_)) => { debug!("Could not get entries for signal EntryUpdated: new: {}, original: {}", action.hashed.hash, update.original_action_address); },
                 (Err(e), _) | (_, Err(e)) => { error!("Error getting entries for signal EntryUpdated: {:?}", e); }
             }
             Ok(())
        }
        Action::Delete(delete) => {
             match get_entry_for_action(&delete.deletes_address) {
                 Ok(Some(original_app_entry)) => { emit_signal(Signal::EntryDeleted { action, original_app_entry, })?; },
                 Ok(None) => { debug!("Could not get entry for signal EntryDeleted: {}", delete.deletes_address); },
                 Err(e) => { error!("Error getting entry for signal EntryDeleted: {:?}", e); }
             }
            Ok(())
        }
        _ => Ok(()),
    }
}

// get_entry_for_action helper (no changes needed here)
fn get_entry_for_action(action_hash: &ActionHash) -> ExternResult<Option<EntryTypes>> {
     // ... (keep existing implementation) ...
     let details = get_details(action_hash.clone(), GetOptions::default())?;
     let record = match details { Some(Details::Record(record_details)) => record_details.record, _ => return Ok(None), };
     let entry = match record.entry().as_option() { Some(entry) => entry.clone(), None => return Ok(None), };
     let action = record.action();
     let (zome_index, entry_index) = match action.entry_type() { Some(EntryType::App(AppEntryDef { zome_index, entry_index, .. })) => (*zome_index, *entry_index), _ => return Ok(None), };
     match EntryTypes::deserialize_from_type(zome_index, entry_index, &entry) {
         Ok(Some(entry_type)) => Ok(Some(entry_type)),
         Ok(None) => { warn!("Could not deserialize entry type for action {:?} with type index ({:?}, {:?})", action_hash, zome_index, entry_index); Ok(None) },
         Err(e) => { error!("Failed to deserialize entry for action {:?}: {:?}", action_hash, e); Err(e.into()) }
     }
}

// REMOVED Generic send_signal function

