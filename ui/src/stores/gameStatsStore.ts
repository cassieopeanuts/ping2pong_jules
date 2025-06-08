import { writable } from 'svelte/store';
import type { AppClient, ActionHash, Record } from '@holochain/client';
import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../ping_2_pong/holochainConfig'; // Adjust path as needed
import type { GameStats } from '../ping_2_pong/ping_2_pong/types'; // Adjust path as needed
import { decode } from '@msgpack/msgpack';
import { encodeHashToBase64 } from '@holochain/client';

// Interface for the store's state
export interface GameStatsStore {
  [gameIdB64: string]: GameStats;
}

// Create a writable Svelte store instance
export const gameStatsStore = writable<GameStatsStore>({});

// Function to fetch game statistics
export async function fetchGameStats(client: AppClient, gameId: ActionHash): Promise<GameStats | null> {
  const gameIdB64 = encodeHashToBase64(gameId);
  let existingStats: GameStats | undefined;
  gameStatsStore.subscribe(value => {
    existingStats = value[gameIdB64];
  })(); // Immediately invoke to get current value

  if (existingStats) {
    console.log(`[gameStatsStore] Found existing stats for game ${gameIdB64} in store.`);
    return existingStats;
  }

  console.log(`[gameStatsStore] Fetching stats for game ${gameIdB64} from zome.`);
  try {
    const record: Record | null = await client.callZome({
      cap_secret: null,
      role_name: HOLOCHAIN_ROLE_NAME,
      zome_name: HOLOCHAIN_ZOME_NAME,
      fn_name: "get_game_stats_for_game", // This zome function needs to be created
      payload: gameId,
    });

    if (record && record.entry && record.entry.entry_type === 'App' && record.entry.entry) {
      const entryData = record.entry.entry as Uint8Array; // Assuming it's Uint8Array
      const stats = decode(entryData) as GameStats; // Ensure this matches the structure in GameStats entry

      // Validate if the decoded object is indeed GameStats, e.g. by checking a few key properties
      if (stats && typeof stats.latency_ms === 'number') {
        gameStatsStore.update(currentStats => {
          currentStats[gameIdB64] = stats;
          return currentStats;
        });
        console.log(`[gameStatsStore] Fetched and stored stats for game ${gameIdB64}:`, stats);
        return stats;
      } else {
        console.warn(`[gameStatsStore] Decoded entry for game ${gameIdB64} is not valid GameStats:`, stats);
        return null;
      }
    } else {
      console.log(`[gameStatsStore] No stats record found for game ${gameIdB64}.`);
      return null;
    }
  } catch (e) {
    console.error(`[gameStatsStore] Error fetching game statistics for ${gameIdB64}:`, e);
    return null;
  }
}
