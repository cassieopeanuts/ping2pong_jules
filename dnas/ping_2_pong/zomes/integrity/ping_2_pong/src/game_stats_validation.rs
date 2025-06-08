use hdk::prelude::*;
use crate::GameStats;

#[hdk_extern]
pub fn validate_create_game_stats(
    _action: SignedActionHashed,
    _game_stats: GameStats,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Add actual validation logic here
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
pub fn validate_update_game_stats(
    _action: SignedActionHashed,
    _game_stats: GameStats,
    _original_action: SignedActionHashed,
    _original_game_stats: GameStats,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Add actual validation logic here
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
pub fn validate_delete_game_stats(
    _action: SignedActionHashed,
    _original_action: SignedActionHashed,
    _original_game_stats: GameStats,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: Add actual validation logic here
    Ok(ValidateCallbackResult::Valid)
}
