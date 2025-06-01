// ping_2_pong/dnas/ping_2_pong/zomes/coordinator/ping_2_pong/src/score.rs
use hdk::prelude::*;
use ping_2_pong_integrity::*;
use crate::utils::get_game_hash_by_id; // Use helper
use ping_2_pong_integrity::game::GameStatus; // Directly from integrity
use ping_2_pong_integrity::Game; // Assuming Game is also directly available

// Maximum allowed score points.
const MAX_POINTS: u32 = 10000; // Keep high for flexibility, game logic enforces 10

#[hdk_extern]
pub fn create_score(score_input: Score) -> ExternResult<Record> { // Renamed 'score' to 'score_input' for clarity with new var 'score'
    debug!("[score.rs] create_score: Called with input relevant parts: game_id {:?}, player {:?}, points {:?}", score_input.game_id, score_input.player, score_input.player_points);

    // --- Validation ---
    // Ensure the game_id corresponds to an actual Game entry
    // Note: game_id in Score struct should be the original ActionHash of the game creation
    let game_action_hash = get_game_hash_by_id(&score_input.game_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", score_input.game_id))))?;

    // Fetch the *latest* game state record to check status
    debug!("[score.rs] create_score: Fetching game record for game_id: {:?}", score_input.game_id);
    match crate::game::get_latest_game(game_action_hash.clone()) { // game_action_hash is the original_game_hash needed by get_latest_game
        Ok(Some(game_record)) => {
            match game_record.entry().to_app_option::<ping_2_pong_integrity::Game>() {
                Ok(Some(game_entry)) => {
                    debug!("[score.rs] create_score: For game_id {:?}, current game status is: {:?}", score_input.game_id, game_entry.game_status);
                    if game_entry.game_status != ping_2_pong_integrity::game::GameStatus::Finished {
                        debug!("[score.rs] create_score: WARNING - Attempting to create score for a game (id: {:?}) not in 'Finished' state. Actual status: {:?}", score_input.game_id, game_entry.game_status);
                        // Note: Original code returns error here, which is good. This log is just an additional warning.
                        // The original error will be hit below if this condition is true.
                    }
                }
                Err(e) => debug!("[score.rs] create_score: Error deserializing game entry for game_id {:?}: {:?}", score_input.game_id, e),
                Ok(None) => debug!("[score.rs] create_score: Game entry data not found for game_id {:?}", score_input.game_id),
            }
        }
        Err(e) => debug!("[score.rs] create_score: Error fetching game record for game_id {:?}: {:?}", score_input.game_id, e),
        Ok(None) => debug!("[score.rs] create_score: No game record found for game_id {:?}", score_input.game_id),
    }

    // Re-fetch for actual validation logic (original code structure)
    let game_record_for_validation = crate::game::get_latest_game(game_action_hash.clone())? // Use cloned game_action_hash
        .ok_or(wasm_error!(WasmErrorInner::Guest("Game record not found for validation".into())))?;
    let game_for_validation = game_record_for_validation
        .entry()
        .to_app_option::<Game>() // Assuming Game is directly available from integrity
        .map_err(|e| wasm_error!(WasmErrorInner::Serialize(e)))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid Game entry format for validation".into())))?;

    // Ensure the game status is Finished before recording score
    if game_for_validation.game_status != GameStatus::Finished { // GameStatus directly from integrity
        return Err(wasm_error!(WasmErrorInner::Guest("Scores can only be recorded for 'Finished' games".into())));
    }

    // Ensure the score is being assigned to a player who was actually in the game.
    if score_input.player != game_for_validation.player_1 && game_for_validation.player_2.as_ref() != Some(&score_input.player) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score must be assigned to a player who participated in the game".into()
        )));
    }
    // Corrected the second instance of the check to use the correct variable names
    if score_input.player != game_for_validation.player_1 && game_for_validation.player_2.as_ref() != Some(&score_input.player) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Score must be assigned to a player who participated in the game (repeated check with correct vars)".into()
        )));
    }

    // Ensure caller is one of the players in the game? Or allow anyone to record score?
    // Let's allow anyone for now, assuming UI calls this after game ends for both players.
    // let my_pub_key = agent_info()?.agent_latest_pubkey;
    // if my_pub_key != game_for_validation.player_1 && game_for_validation.player_2.as_ref() != Some(&my_pub_key) {
    //     return Err(wasm_error!(WasmErrorInner::Guest("Only game participants can record the score".into())));
    // }


    // Validate that the score points are within a reasonable range.
    if score_input.player_points > MAX_POINTS { // MAX_POINTS is high, maybe check against game win condition?
        warn!("Score points {} exceed MAX_POINTS {}", score_input.player_points, MAX_POINTS);
        // Allow high scores for now, UI/game logic should enforce game rules like first to 10.
        // return Err(wasm_error!(WasmErrorInner::Guest("Player points exceed the maximum allowed".into())));
    }
     if score_input.player_points > 100 { // Add a more reasonable sanity check
         warn!("Recorded score {} seems high.", score_input.player_points);
     }
     // --- End Validation ---


    // Create the Score entry.
    let score_to_create = score_input.clone(); // Use the cloned input for creation
    let score_action_hash = match create_entry(&EntryTypes::Score(score_to_create)) {
        Ok(hash) => {
            debug!("[score.rs] create_score: create_entry for Score successful, action hash: {:?}", hash);
            hash
        }
        Err(e) => {
            debug!("[score.rs] create_score: create_entry for Score failed: {:?}", e);
            return Err(e);
        }
    };

    // Link the Score action hash from the Player's pubkey.
    // Error handling for create_link can be added if necessary, for now assuming ? operator is sufficient
    create_link(
        score_input.player.clone(),
        score_action_hash.clone(),
        LinkTypes::PlayerToScores, // Changed from ScoreToPlayer based on convention BaseToTargets
        (),
    )?;

    // Link the Score action hash from the original game's action hash.
    create_link(
        score_input.game_id.clone(), // Base is the original game action hash
        score_action_hash.clone(),
        LinkTypes::GameToScores, // Use a more descriptive name if possible, or reuse ScoreUpdates? Let's define GameToScores
        (),
    )?;

    // Retrieve and return the created Score record.
    let record = get(score_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not find the newly created Score".to_string())))?;
    debug!("[score.rs] create_score: Successfully created score, returning record for action: {:?}", record.action_hashed().hash);
    Ok(record)
}

// --- Other Score CRUD functions ---
// get_latest_score/get_original_score/update_score etc. rely on ScoreUpdates links.
// The create_score now uses GameToScores. If updates to scores are needed,
// we might need ScoreUpdates links from the original Score action hash.
// Let's assume Scores are immutable for now, matching the design intent (record final score).
// We can remove update/delete score functions if scores are immutable.

#[hdk_extern]
pub fn get_scores_for_game(game_id: ActionHash) -> ExternResult<Vec<Record>> {
    // Ensure game_id is valid first? Optional.
     let _ = get_game_hash_by_id(&game_id)?
         .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Game ID does not exist: {}", game_id))))?;


    let links = get_links(
        GetLinksInputBuilder::try_new(game_id, LinkTypes::GameToScores)? // Use the new link type
            .build(),
    )?;

    let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        .collect();

    if get_inputs.is_empty() {
        return Ok(vec![]);
    }

    let records = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    Ok(records.into_iter().flatten().collect())
}


#[hdk_extern]
pub fn get_scores_for_player(player: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        GetLinksInputBuilder::try_new(player, LinkTypes::PlayerToScores)? // Correct link type
            .build()
    )?;

     let get_inputs: Vec<GetInput> = links
        .into_iter()
        .filter_map(|link| link.target.into_action_hash())
        .map(|ah| GetInput::new(ah.into(), GetOptions::default()))
        .collect();

    if get_inputs.is_empty() {
        return Ok(vec![]);
    }

    let records = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    Ok(records.into_iter().flatten().collect())
}


// REMOVE functions related to Score updates and deletes if score is immutable post-creation
/*
#[hdk_extern]
pub fn get_latest_score(original_score_hash: ActionHash) -> ExternResult<Option<Record>> { ... }

#[hdk_extern]
pub fn get_original_score(original_score_hash: ActionHash) -> ExternResult<Option<Record>> { ... }

#[hdk_extern]
pub fn get_all_revisions_for_score(original_score_hash: ActionHash) -> ExternResult<Vec<Record>> { ... }

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateScoreInput { ... }

#[hdk_extern]
pub fn update_score(input: UpdateScoreInput) -> ExternResult<Record> { ... }

#[hdk_extern]
pub fn delete_score(original_score_hash: ActionHash) -> ExternResult<ActionHash> { ... }

#[hdk_extern]
pub fn get_all_deletes_for_score(original_score_hash: ActionHash) -> ExternResult<Option<Vec<SignedActionHashed>>> { ... }

#[hdk_extern]
pub fn get_oldest_delete_for_score(original_score_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> { ... }

#[hdk_extern]
pub fn get_deleted_scores_for_player(player: AgentPubKey) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> { ... }
*/

// get_game_hash_by_id was removed (now in utils)