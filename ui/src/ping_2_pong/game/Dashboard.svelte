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
    <!-- Static Global Chat Placeholder Removed -->
  </div>
  <div class="dashboard-sidebar-right">
    <!-- Lobby dispatches join-game events -->
    <Lobby on:join-game={(e) => dispatch("join-game", e.detail)} />
  </div>
</div>
