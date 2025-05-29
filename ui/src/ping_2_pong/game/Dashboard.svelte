<script lang="ts">
  import Leaderboard from "./Leaderboard.svelte";
  import Lobby from "./Lobby.svelte";
  import PlayButton from "./PlayButton.svelte";
  import { currentRoute } from "../../stores/routeStore";
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  function handlePlay() {
    currentRoute.set("gameplay");
  }
</script>

<div class="dashboard-layout">
  <div class="dashboard-sidebar-left">
    <Leaderboard />
  </div>
  <div class="dashboard-center">
    <PlayButton on:play={handlePlay} />
    <div class="global-chat-placeholder">
      <h4>Global Chat (Coming Soon)</h4>
      <div class="chat-messages-placeholder">
        <p><span>Player1:</span> Hello world!</p>
        <p><span>Player2:</span> Hey there, ready to pong?</p>
        <p><span>Player3:</span> Waiting for matchmaking...</p>
      </div>
      <div class="chat-input-placeholder">
        Chat disabled - backend not ready
      </div>
    </div>
  </div>
  <div class="dashboard-sidebar-right">
    <!-- Lobby dispatches join-game events -->
    <Lobby on:join-game={(e) => dispatch("join-game", e.detail)} />
  </div>
</div>
