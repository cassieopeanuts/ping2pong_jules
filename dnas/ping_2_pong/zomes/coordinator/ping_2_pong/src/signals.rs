// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/signals.rs
use hdk::prelude::*;
use crate::{Signal, Game}; // Assuming Game is ping_2_pong_integrity::Game or similar
// use std::ops::Sub; // Not strictly needed if using checked_sub method directly

// kitsune_p2p_timestamp::Timestamp is used for sent_at.
// sys_time() also returns kitsune_p2p_timestamp::Timestamp.

/// ───────────────────────── init helper ─────────────────────────
pub fn grant_remote_signal_cap() -> ExternResult<()> {
    create_cap_grant(CapGrantEntry {
        tag: "remote-signal".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(
            vec![("ping_2_pong".into(), "receive_remote_signal".into())]
                .into_iter()
                .map(Into::into)
                .collect(),
        ),
    })?;
    Ok(())
}

/// ──────────────────────── local re-emit ───────────────────────
#[hdk_extern]
pub fn receive_remote_signal(signal: Signal) -> ExternResult<()> {
    match &signal {
        Signal::PaddleUpdate { game_id: _, player: _, paddle_y: _, sent_at } => { // Destructure to get sent_at
            let received_at_for_signal = sys_time()?;
            let duration_option = received_at_for_signal.checked_sub(*sent_at);

            let duration_since_sent = match duration_option {
                Some(duration) => duration,
                None => {
                    error!(
                        "Timestamp underflow for PaddleUpdate: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    );
                    return Err(wasm_error!(WasmErrorInner::Guest(format!(
                        "Timestamp underflow for PaddleUpdate: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    ))));
                }
            };

            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!(
                "Signal latency (PaddleUpdate): {} ms. Sent: {:?}, Received: {:?}",
                latency_ms, sent_at, received_at_for_signal
            );
        }
        Signal::BallUpdate { game_id: _, ball_x: _, ball_y: _, ball_dx: _, ball_dy: _, sent_at } => { // Destructure
            let received_at_for_signal = sys_time()?;
            let duration_option = received_at_for_signal.checked_sub(*sent_at);

            let duration_since_sent = match duration_option {
                Some(duration) => duration,
                None => {
                    error!(
                        "Timestamp underflow for BallUpdate: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    );
                    return Err(wasm_error!(WasmErrorInner::Guest(format!(
                        "Timestamp underflow for BallUpdate: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    ))));
                }
            };

            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!(
                "Signal latency (BallUpdate): {} ms. Sent: {:?}, Received: {:?}",
                latency_ms, sent_at, received_at_for_signal
            );
        }
        Signal::ScoreUpdate { game_id: _, score1: _, score2: _, sent_at } => { // Destructure
            let received_at_for_signal = sys_time()?;
            let duration_option = received_at_for_signal.checked_sub(*sent_at);

            let duration_since_sent = match duration_option {
                Some(duration) => duration,
                None => {
                    error!(
                        "Timestamp underflow for ScoreUpdate: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    );
                    return Err(wasm_error!(WasmErrorInner::Guest(format!(
                        "Timestamp underflow for ScoreUpdate: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    ))));
                }
            };

            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!(
                "Signal latency (ScoreUpdate): {} ms. Sent: {:?}, Received: {:?}",
                latency_ms, sent_at, received_at_for_signal
            );
        }
        Signal::GameOver { game_id: _, winner: _, score1: _, score2: _, sent_at } => { // Destructure
            let received_at_for_signal = sys_time()?;
            let duration_option = received_at_for_signal.checked_sub(*sent_at);

            let duration_since_sent = match duration_option {
                Some(duration) => duration,
                None => {
                    error!(
                        "Timestamp underflow for GameOver: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    );
                    return Err(wasm_error!(WasmErrorInner::Guest(format!(
                        "Timestamp underflow for GameOver: sent_at {:?} is later than received_at {:?}.",
                        sent_at, received_at_for_signal
                    ))));
                }
            };

            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!(
                "Signal latency (GameOver): {} ms. Sent: {:?}, Received: {:?}",
                latency_ms, sent_at, received_at_for_signal
            );
        }
        _ => {
            // Other signals that don't have 'sent_at' or don't need latency calculation.
        }
    }

    // Existing logic to re-emit the signal locally
    emit_signal(&signal)
}

/// ─────────────────────── payload structs ──────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaddleUpdatePayload {
    pub game_id:  ActionHash,
    pub paddle_y: u32,
    pub sent_at: Timestamp, // This is kitsune_p2p_timestamp::Timestamp
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BallUpdatePayload {
    pub game_id: ActionHash,
    pub sent_at: Timestamp,
    pub ball_x:  u32,
    pub ball_y:  u32,
    pub ball_dx: i32,
    pub ball_dy: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameOverPayload {
    pub game_id: ActionHash,
    pub winner:  Option<AgentPubKey>,
    pub score1:  u32,
    pub score2:  u32,
    pub sent_at: Timestamp,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameAbandonedPayload {
    pub game_id: ActionHash,
}

/// Return the `Record` at the *tip* of an update chain
fn latest_record(original: &ActionHash) -> ExternResult<Record> {
    let mut current = original.clone();

    loop {
        let details = get_details(current.clone(), GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Game details not found".into())))?;

        match details {
            Details::Record(rec) => {
                if let Some(update) = rec.updates.last() {
                    current = update.action_address().clone();
                } else {
                    return Ok(rec.record);
                }
            }
            _ => return Err(wasm_error!(WasmErrorInner::Guest("Unexpected details variant".into()))),
        }
    }
}

/// ───────────────────── broadcast helper ──────────────────────
fn broadcast_to_opponents(game_id: &ActionHash, signal_to_broadcast: &Signal) -> ExternResult<()> {
    let record = latest_record(game_id)?;
    let game: Game = record
        .entry()
        .to_app_option::<Game>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Malformed Game entry".into())))?;

    let me = agent_info()?.agent_latest_pubkey;
    let recipients = [&Some(game.player_1.clone()), &game.player_2]
        .iter()
        .filter_map(|o| o.as_ref())
        .filter(|pk| **pk != me)
        .cloned()
        .collect::<Vec<_>>();

    for agent in recipients {
        let _ = call_remote(
            agent,
            "ping_2_pong",
            "receive_remote_signal".into(),
            None,
            signal_to_broadcast,
        );
    }
    Ok(())
}

/// ───────────────────── externs used by UI ────────────────────
#[hdk_extern]
pub fn send_paddle_update(mut payload: PaddleUpdatePayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal_to_send = Signal::PaddleUpdate {
        game_id:  payload.game_id.clone(),
        player:   agent_info()?.agent_latest_pubkey,
        paddle_y: payload.paddle_y,
        sent_at: payload.sent_at,
    };
    broadcast_to_opponents(&payload.game_id, &signal_to_send)
}

#[hdk_extern]
pub fn send_game_abandoned_signal(payload: GameAbandonedPayload) -> ExternResult<()> {
    let abandoned_by_player = agent_info()?.agent_latest_pubkey;
    let signal_to_send = Signal::GameAbandoned {
        game_id: payload.game_id.clone(),
        abandoned_by_player,
    };
    broadcast_to_opponents(&payload.game_id, &signal_to_send)
}

#[hdk_extern]
pub fn send_ball_update(mut payload: BallUpdatePayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal_to_send = Signal::BallUpdate {
        game_id: payload.game_id.clone(),
        ball_x:  payload.ball_x,
        ball_y:  payload.ball_y,
        ball_dx: payload.ball_dx,
        ball_dy: payload.ball_dy,
        sent_at: payload.sent_at,
    };
    broadcast_to_opponents(&payload.game_id, &signal_to_send)
}

#[hdk_extern]
pub fn send_score_update(mut payload: GameOverPayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal_to_send = Signal::ScoreUpdate {
        game_id: payload.game_id.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
        sent_at: payload.sent_at,
    };
    broadcast_to_opponents(&payload.game_id, &signal_to_send)
}

#[hdk_extern]
pub fn send_game_over(mut payload: GameOverPayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal_to_send = Signal::GameOver {
        game_id: payload.game_id.clone(),
        winner:  payload.winner.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
        sent_at: payload.sent_at,
    };
    broadcast_to_opponents(&payload.game_id, &signal_to_send)
}
