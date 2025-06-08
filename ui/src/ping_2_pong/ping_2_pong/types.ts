import type { AgentPubKey, ActionHash, Record, HoloHash } from '@holochain/client';

// Base64 encoded HoloHash (often used for ActionHash and EntryHash in UI contexts)
export type HoloHashB64 = string;
// Base64 encoded representation of an Agent's Public Key
export type AgentPubKeyB64 = string; // This is usually the same as HoloHashB64 for AgentPubKeys

// Holochain's Timestamp [seconds: number, nanoseconds: number]
export type Timestamp = [number, number];

// Signal Payloads
export interface PaddleUpdatePayload {
    game_id: ActionHash; // Using ActionHash (Uint8Array) from client, assuming direct use
    paddle_y: number;
    sent_at: Timestamp;
}

export interface BallUpdatePayload {
    game_id: ActionHash;
    ball_x: number;
    ball_y: number;
    ball_dx: number;
    ball_dy: number;
    sent_at: Timestamp;
}

// ScoreUpdatePayload is not explicitly defined as a struct in backend,
// it reuses GameOverPayload for sending signals.
// We can define it for clarity in UI if needed, or use GameOverPayload directly.
export interface ScoreUpdatePayload { // Matches fields in Signal::ScoreUpdate
    game_id: ActionHash;
    score1: number;
    score2: number;
    sent_at: Timestamp;
}

export interface GameOverPayload {
    game_id: ActionHash;
    winner?: AgentPubKey; // Or AgentPubKeyB64 if serialized that way
    score1: number;
    score2: number;
    sent_at: Timestamp;
}

// GameStats structure for creating game statistics
export interface GameStats {
    game_id: ActionHash; // This is the original_action_hash of the game
    player_1: AgentPubKey; // Or AgentPubKeyB64
    player_2: AgentPubKey; // Or AgentPubKeyB64
    latency_ms: number;
    time_to_write_score_ms: number;
    time_to_read_score_ms: number;
    created_at: Timestamp;
}

// Output type for the create_score zome function
export interface CreateScoreOutput {
    score_hash: ActionHash; // ActionHash of the created score entry
    write_duration_ms: number;
}

// Output type for the get_score_and_measure_time zome function
export interface GetScoreOutput {
    score_record?: Record; // The retrieved score record, could be undefined if not found
    read_duration_ms: number;
}


// --- Existing types from the file, ensure they are compatible or updated ---

// HdkTimestamp was previously number (ms since epoch), changing to Holochain's [number, number]
// For UI purposes, we might still want to convert this, but for payloads, it should match the backend.
// export type HdkTimestamp = number; // Keeping this commented out to avoid confusion with Timestamp

export interface GlobalChatMessageSignal {
  type: "GlobalChatMessage";
  timestamp: Timestamp; // Changed from HdkTimestamp
  sender: AgentPubKeyB64; // AgentPubKeyB64 is fine for string representation
  content: string;
}

export interface ChatMessagePayloadU {
  timestamp: Timestamp; // Changed from HdkTimestamp
  sender: AgentPubKeyB64;
  content: string;
}

export interface Player {
  player_name: string;
  // player_key: AgentPubKey; // AgentPubKey from client is Uint8Array
  player_key: AgentPubKeyB64; // Often, UIs handle AgentKeys as B64 strings until actual client call
}

// Additional types that might be useful from context
export interface Game {
    game_id: ActionHash; // Original action hash of the game
    player_1: AgentPubKey;
    player_2?: AgentPubKey;
    status: GameStatus; // e.g., "Playing", "Finished"
    // ... other game fields
}

export enum GameStatus {
    Pending = "Pending",
    InProgress = "InProgress",
    Finished = "Finished",
    Abandoned = "Abandoned",
}

// Representing signals as they come from backend (raw.App.payload)
// This is a generic structure, specific signal types will be in payload
export interface AppSignal {
    type: string; // e.g., "PaddleUpdate", "BallUpdate"
    game_id: ActionHash; // Or ActionHashB64 if serialized as string
    // ... other common fields if any
    // Specific fields like player, paddle_y, ball_x, etc., and sent_at will be part of the specific signal type
}

// Example of how a specific signal might look when received, before full decoding
export interface RawPaddleUpdateSignal extends AppSignal {
    type: "PaddleUpdate";
    player: AgentPubKey; // Or AgentPubKeyB64
    paddle_y: number;
    sent_at: Timestamp;
}
// Similar Raw types for BallUpdate, ScoreUpdate, GameOver can be defined.
