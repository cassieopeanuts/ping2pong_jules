// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/signals.rs
use hdk::prelude::*;
use crate::{Signal, Game};

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
    // The received_at time is captured at the beginning of the function.
    // For latency calculation, it's better to get it just before processing each relevant signal,
    // but for simplicity and consistency with previous logic, we can use a single `received_at`
    // if the processing within the match arms is quick.
    // However, the instruction implies fetching it fresh for each calculation block.
    // Let's adhere to that for precision.

    match &signal {
        Signal::PaddleUpdate { sent_at, .. } => {
            let received_at_for_signal = sys_time()?;
            let duration_since_sent = received_at_for_signal.sub(*sent_at)
                .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
                    format!("Timestamp underflow for PaddleUpdate: sent_at {:?} may be later than received_at {:?} or other issue.", sent_at, received_at_for_signal)
                )))?;
            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!("Signal latency (PaddleUpdate): {} ms. Sent: {:?}, Received: {:?}", latency_ms, sent_at, received_at_for_signal);
        }
        Signal::BallUpdate { sent_at, .. } => {
            let received_at_for_signal = sys_time()?;
            let duration_since_sent = received_at_for_signal.sub(*sent_at)
                .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
                    format!("Timestamp underflow for BallUpdate: sent_at {:?} may be later than received_at {:?} or other issue.", sent_at, received_at_for_signal)
                )))?;
            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!("Signal latency (BallUpdate): {} ms. Sent: {:?}, Received: {:?}", latency_ms, sent_at, received_at_for_signal);
        }
        Signal::ScoreUpdate { sent_at, .. } => {
            let received_at_for_signal = sys_time()?;
            let duration_since_sent = received_at_for_signal.sub(*sent_at)
                .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
                    format!("Timestamp underflow for ScoreUpdate: sent_at {:?} may be later than received_at {:?} or other issue.", sent_at, received_at_for_signal)
                )))?;
            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!("Signal latency (ScoreUpdate): {} ms. Sent: {:?}, Received: {:?}", latency_ms, sent_at, received_at_for_signal);
        }
        Signal::GameOver { sent_at, .. } => {
            let received_at_for_signal = sys_time()?;
            let duration_since_sent = received_at_for_signal.sub(*sent_at)
                .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
                    format!("Timestamp underflow for GameOver: sent_at {:?} may be later than received_at {:?} or other issue.", sent_at, received_at_for_signal)
                )))?;
            let latency_ms = (duration_since_sent.as_micros() / 1000) as u64;
            debug!("Signal latency (GameOver): {} ms. Sent: {:?}, Received: {:?}", latency_ms, sent_at, received_at_for_signal);
        }
        // For other signals that don't have `sent_at`, we just emit them
        _ => {}
    }

    emit_signal(&signal)
}

/// ─────────────────────── payload structs ──────────────────────
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PaddleUpdatePayload {
    pub game_id:  ActionHash,
    pub paddle_y: u32,
    pub sent_at: Timestamp,
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
            .ok_or(wasm_error!("Game details not found"))?;

        match details {
            Details::Record(rec) => {
                // Any further updates?
                if let Some(update) = rec.updates.last() {
                    current = update.action_address().clone();
                } else {
                    return Ok(rec.record);
                }
            }
            // Only `Record` details make sense for an ActionHash.
            _ => return Err(wasm_error!("Unexpected details variant")),
        }
    }
}

/// ───────────────────── broadcast helper ──────────────────────
fn broadcast_to_opponents(game_id: &ActionHash, signal: &Signal) -> ExternResult<()> {
    // 1. load the *latest* Game entry
    let record = latest_record(game_id)?;
    let game: Game = record
        .entry()
        .to_app_option::<Game>()
        .map_err(|e| wasm_error!(e.to_string()))?
        .ok_or(wasm_error!("Malformed Game entry"))?;

    // 2. build recipient list (everyone except me)
    let me = agent_info()?.agent_latest_pubkey;
    let recipients = [&Some(game.player_1.clone()), &game.player_2]
        .iter()
        .filter_map(|o| o.as_ref())
        .filter(|pk| **pk != me)
        .cloned()
        .collect::<Vec<_>>();

    // 3. fire-and-forget
    for agent in recipients {
        let _ = call_remote(
            agent,
            "ping_2_pong",               // zome
            "receive_remote_signal".into(),
            None,                        // no cap secret
            signal,              // payload
        );
    }
    Ok(())
}

/// ───────────────────── externs used by UI ────────────────────
#[hdk_extern]
pub fn send_paddle_update(mut payload: PaddleUpdatePayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal = Signal::PaddleUpdate {
        game_id:  payload.game_id.clone(),
        player:   agent_info()?.agent_latest_pubkey,
        paddle_y: payload.paddle_y,
        sent_at: payload.sent_at, // Populate sent_at in the signal
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_game_abandoned_signal(payload: GameAbandonedPayload) -> ExternResult<()> {
    let abandoned_by_player = agent_info()?.agent_latest_pubkey;
    let signal = Signal::GameAbandoned { // Ensure Signal::GameAbandoned matches the enum in lib.rs
        game_id: payload.game_id.clone(),
        abandoned_by_player,
    };
    
    // emit_signal(&signal)?; // Optional: emit locally for the abandoner
    
    // Broadcast to the opponent
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_ball_update(mut payload: BallUpdatePayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal = Signal::BallUpdate {
        game_id: payload.game_id.clone(),
        ball_x:  payload.ball_x,
        ball_y:  payload.ball_y,
        ball_dx: payload.ball_dx,
        ball_dy: payload.ball_dy,
        sent_at: payload.sent_at, // Populate sent_at in the signal
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_score_update(mut payload: GameOverPayload) -> ExternResult<()> {
    // we re-use GameOverPayload because it already contains score1/score2
    payload.sent_at = sys_time()?;
    let signal = Signal::ScoreUpdate {
        game_id: payload.game_id.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
        sent_at: payload.sent_at, // Populate sent_at in the signal
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}

#[hdk_extern]
pub fn send_game_over(mut payload: GameOverPayload) -> ExternResult<()> {
    payload.sent_at = sys_time()?;
    let signal = Signal::GameOver {
        game_id: payload.game_id.clone(),
        winner:  payload.winner.clone(),
        score1:  payload.score1,
        score2:  payload.score2,
        sent_at: payload.sent_at, // Populate sent_at in the signal
    };
    emit_signal(&signal)?;
    broadcast_to_opponents(&payload.game_id, &signal)
}
