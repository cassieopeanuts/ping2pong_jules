<script lang="ts">
  import Leaderboard from "./Leaderboard.svelte";
  import Lobby from "./Lobby.svelte";
  import PlayButton from "./PlayButton.svelte";
  import GlobalChat from "../chat/GlobalChat.svelte"; // Added import
  // import { currentRoute } from "../../stores/routeStore"; // No longer needed here for routing
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  // function handlePlay() { // REMOVED - PlayButton now handles its own matchmaking logic
  //   currentRoute.set("gameplay");
  // }
</script>

<div class="dashboard-layout">
  <div class="dashboard-col-left">
    <Leaderboard />
  </div>
  <div class="dashboard-col-center">
    <PlayButton />
    <GlobalChat />
  </div>
  <div class="dashboard-col-right">
    <!-- Lobby dispatches join-game events -->
    <Lobby on:join-game={(e) => dispatch("join-game", e.detail)} />
  </div>
</div>
