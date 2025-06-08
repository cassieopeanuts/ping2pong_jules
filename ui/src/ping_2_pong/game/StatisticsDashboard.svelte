<script lang="ts">
  import { onMount, getContext, createEventDispatcher } from 'svelte'; // Added createEventDispatcher
  import type { AppClient, ActionHash } from '@holochain/client';
  import { encodeHashToBase64 } from '@holochain/client';
  import { clientContext, type ClientContext } from '../../contexts'; // Adjust path as needed
  import { fetchGameStats, gameStatsStore } from '../../stores/gameStatsStore'; // Adjust path as needed
  import type { GameStats } from '../ping_2_pong/types'; // Adjust path as needed

  export let gameId: ActionHash;

  const dispatch = createEventDispatcher(); // For close event

  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  let currentStats: GameStats | undefined | null = undefined; // undefined: loading, null: error or not found
  let errorMsg: string | null = null;
  let gameIdB64: string;

  // Reactive statement to get stats for the current gameId
  $: if (gameId && $gameStatsStore) {
    gameIdB64 = encodeHashToBase64(gameId);
    currentStats = $gameStatsStore[gameIdB64];
  }

  onMount(async () => {
    if (!gameId) {
      errorMsg = "Game ID prop is missing.";
      currentStats = null; // Set to null to indicate error/not loaded
      return;
    }
    gameIdB64 = encodeHashToBase64(gameId);
    console.log(`[StatisticsDashboard] Mounting for gameId: ${gameIdB64}`);

    client = await appClientContext.getClient();
    if (!client) {
      errorMsg = "Holochain client not available.";
      currentStats = null;
      return;
    }

    // Check if stats are already in the store from a previous fetch
    // Note: $gameStatsStore might not be initialized fully on first access here.
    // fetchGameStats handles store checking internally too.
    const statsFromStore = $gameStatsStore[gameIdB64];
    if (statsFromStore) {
      console.log(`[StatisticsDashboard] Stats for ${gameIdB64} already in store.`);
      currentStats = statsFromStore;
    } else {
      console.log(`[StatisticsDashboard] Fetching stats for ${gameIdB64}.`);
      currentStats = undefined; // Set to loading state
      errorMsg = null;
      try {
        const fetched = await fetchGameStats(client, gameId);
        if (fetched) {
          // currentStats will be updated by the reactive statement $:
          // but we can set it here if needed, though store update should trigger reactivity.
        } else {
          errorMsg = "Game statistics not found for this game.";
          currentStats = null; // Explicitly set to null if not found after fetch
        }
      } catch (e) {
        console.error("Error fetching game stats:", e);
        errorMsg = `Error fetching stats: ${(e as Error).message}`;
        currentStats = null;
      }
    }
  });

  // Helper to format timestamp
  function formatTimestamp(timestamp: [number, number] | undefined | null): string {
    if (!timestamp || !Array.isArray(timestamp) || timestamp.length !== 2) return 'N/A';
    try {
      const date = new Date(timestamp[0] * 1000 + Math.floor(timestamp[1] / 1_000_000));
      return date.toLocaleString();
    } catch (e) {
      return 'Invalid Date';
    }
  }

  function closeDashboard() {
    dispatch('close');
  }
</script>

<div class="stats-dashboard-modal">
  <div class="stats-dashboard-content">
    <button class="close-button" on:click={closeDashboard}>&times;</button>
    {#if gameIdB64}
      <h2>Game Statistics</h2>
      <p class="game-id-display"><strong>Game ID:</strong> {gameIdB64}</p>
    {/if}

    {#if currentStats === undefined}
      <p>Loading statistics...</p>
    {:else if currentStats === null || !currentStats}
      <p class="error">{errorMsg || "Statistics not available for this game."}</p>
    {:else}
      <div class="stats-grid">
        <div><strong>Player 1:</strong> {currentStats.player_1 ? encodeHashToBase64(currentStats.player_1) : 'N/A'}</div>
        <div><strong>Player 2:</strong> {currentStats.player_2 ? encodeHashToBase64(currentStats.player_2) : 'N/A'}</div>
        <div><strong>Average Latency:</strong> {currentStats.latency_ms} ms</div>
        <div><strong>Time to Write Score:</strong> {currentStats.time_to_write_score_ms} ms</div>
        <div><strong>Time to Read Score:</strong> {currentStats.time_to_read_score_ms} ms</div>
        <div><strong>Stats Recorded At:</strong> {formatTimestamp(currentStats.created_at)}</div>
      </div>
    {/if}
  </div>
</div>

<style>
  .stats-dashboard-modal {
    position: fixed;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    background-color: rgba(0, 0, 0, 0.6);
    display: flex;
    justify-content: center;
    align-items: center;
    z-index: 1000; /* Ensure it's on top */
  }
  .stats-dashboard-content {
    padding: 30px;
    font-family: Arial, sans-serif;
    background-color: #fff;
    border-radius: 10px;
    box-shadow: 0 5px 15px rgba(0,0,0,0.3);
    max-width: 600px;
    width: 90%;
    position: relative; /* For positioning the close button */
    max-height: 80vh;
    overflow-y: auto;
  }
  .close-button {
    position: absolute;
    top: 10px;
    right: 15px;
    font-size: 2rem;
    font-weight: bold;
    color: #888;
    background: none;
    border: none;
    cursor: pointer;
  }
  .close-button:hover {
    color: #333;
  }
  h2 {
    text-align: center;
    color: #333;
    margin-top: 0;
    margin-bottom: 15px;
  }
  .stats-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
    margin-top: 20px;
  }
  .stats-grid div {
    background-color: #f9f9f9;
    padding: 12px;
    border-radius: 6px;
    border: 1px solid #eee;
  }
  .stats-grid strong {
    color: #555;
  }
  .game-id-display {
    font-size: 0.9em;
    color: #777;
    word-wrap: break-word;
    text-align: center;
    margin-bottom: 20px;
  }
  .error {
    color: #d9534f; /* Bootstrap danger color */
    font-weight: bold;
    text-align: center;
    padding: 10px;
    background-color: #f2dede;
    border-radius: 4px;
  }
</style>
