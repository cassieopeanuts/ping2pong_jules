<script lang="ts">
  import { onMount, onDestroy, setContext } from "svelte";
  // Import Holochain client essentials
  import { AppWebsocket, encodeHashToBase64, decodeHashFromBase64 } from "@holochain/client";
  // Make sure ActionHash is imported if used in types
  import type { AppClient, HolochainError, ActionHash, AgentPubKey } from "@holochain/client";
  // Import Svelte helpers/stores
  import { derived, get } from "svelte/store"; // Import get from svelte/store
  import { clientContext } from "./contexts";
  import { currentRoute } from "./stores/routeStore";
  import { playerProfile, checkAndLoadExistingProfile } from "./stores/playerProfile";
  import { currentGame } from "./stores/currentGame";
  // Import invitation store and helpers
  import { invitations, addInvitation, removeInvitation } from "./stores/invitationStore";
  import { getOrFetchProfile, type DisplayProfile } from "./stores/profilesStore";
  // Import the specific signal type
  import type { GameInvitationSignal, GameStartedSignal, GlobalChatMessageSignal, GameAbandonedSignal } from "./ping_2_pong/ping_2_pong/types"; // Adjust path if necessary
  // Import chat store function
  import { addChatMessage } from "./stores/chatStore"; // Adjust path if necessary
  // Import utility functions
  import { truncatePubkey } from "./utils";
  // Import Holochain constants
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "./holochainConfig";

  // Import Components
  import WelcomePopup from "./ping_2_pong/WelcomePopup.svelte";
  import Dashboard from "./ping_2_pong/game/Dashboard.svelte";
  import type { Dashboard as DashboardType } from "./ping_2_pong/game/Dashboard.svelte"; // For instance binding
  import PongGame from "./ping_2_pong/game/PongGame.svelte";
  import StatisticsDashboard from "./ping_2_pong/game/StatisticsDashboard.svelte";
  import InvitationPopup from "./ping_2_pong/game/InvitationPopup.svelte"; // Adjust path if needed
  import OpponentLeftPopup from "./ping_2_pong/game/OpponentLeftPopup.svelte";

  // Define the UnsubscribeFunction type locally
  type UnsubscribeFunction = () => void;

  // Component State
  let client: AppClient;
  let error: HolochainError | undefined; // For critical/global errors
  let loading = true; // Global loading state
  let presenceIntervalId: ReturnType<typeof setInterval> | undefined;
  let unsubscribeFromSignals: UnsubscribeFunction | undefined; // Use the locally defined type
  let invitationError: string | null = null; // Specific for invitation errors
  let dashboardComponent: DashboardType; // Variable to hold Dashboard instance

  let showOpponentLeftPopup = false;
  let opponentWhoLeftNickname: string | null = null;
  let opponentWhoLeftAgentKeyB64: string | null = null;

  let showStatsDashboardForGameId: ActionHash | null = null; // For showing stats dashboard

  // Holochain Client Setup
  const appClientContext = {
    getClient: async (): Promise<AppClient> => {
      if (!client) {
        try {
          client = await AppWebsocket.connect({ url: new URL("ws://localhost:8888") });
        } catch (e) { console.error("AppWebsocket.connect error:", e); error = e as HolochainError; throw e; }
      }
      return client;
    }
  };

  // --- Presence Publishing ---
  async function publishPresence() {
      const regStatus = get(isRegistered);
      if (!client || !regStatus) return;
      try {
          await client.callZome({ cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME, fn_name: "publish_presence", payload: null, });
      } catch(e) {
          if ((e as HolochainError).message.includes("source chain head has moved")) {
              console.warn("Presence publishing skipped due to source chain conflict (likely harmless).");
          } else {
             console.error("Error publishing presence from App.svelte:", e);
          }
      }
  }


  // --- Signal Handler ---
  function handleSignal(signalPayload: any) {
      // console.log("%%%% RAW SIGNAL RECEIVED BY CLIENT:", JSON.stringify(signalPayload, null, 2));

      if (signalPayload && typeof signalPayload === 'object' && signalPayload.App) {
          const appSignalWrapper = signalPayload.App;
          const actualSignal = appSignalWrapper.payload;

          if (!actualSignal?.type) {
              return;
          }

          if (actualSignal.type === "GameInvitation") {
              const invitation = actualSignal as GameInvitationSignal;
              if (invitation.game_id && invitation.inviter && invitation.message) {
                  if (encodeHashToBase64(invitation.inviter) !== encodeHashToBase64(client.myPubKey)) {
                      addInvitation(invitation);
                  }
              } else {
                  console.warn("[App.svelte handleSignal] Malformed GameInvitation signal received:", invitation);
              }
          } else if (actualSignal.type === "GameStarted") {
              const { game_id, player_1, player_2 } = actualSignal as GameStartedSignal;
              if (game_id && player_1 && player_2) {
                   const myPubKeyB64 = encodeHashToBase64(client.myPubKey);
                   const p1B64 = encodeHashToBase64(player_1);
                   const p2B64 = encodeHashToBase64(player_2);
                   if (myPubKeyB64 === p1B64 || myPubKeyB64 === p2B64) {
                       currentGame.set(game_id);
                       currentRoute.set("gameplay");
                       invitations.set([]);
                   }
              } else {
                   console.warn("[App.svelte handleSignal GameStarted] Signal missing required fields", actualSignal);
              }
          } else if (actualSignal.type === "GlobalChatMessage") {
            const rawSignal = actualSignal as any;
            if (rawSignal.sender && typeof rawSignal.content === 'string' && typeof rawSignal.timestamp === 'number') {
                const messageTimestamp = Math.floor(rawSignal.timestamp / 1000);
                const senderB64 = encodeHashToBase64(rawSignal.sender);
                const chatSignal: GlobalChatMessageSignal = {
                    type: "GlobalChatMessage",
                    sender: senderB64,
                    content: rawSignal.content,
                    timestamp: messageTimestamp,
                };
                addChatMessage(chatSignal);
            } else if (rawSignal.sender && typeof rawSignal.content === 'string' &&
                Array.isArray(rawSignal.timestamp) && rawSignal.timestamp.length === 2 &&
                typeof rawSignal.timestamp[0] === 'number' && typeof rawSignal.timestamp[1] === 'number') {
                const messageTimestamp = rawSignal.timestamp[0] * 1000 + Math.floor(rawSignal.timestamp[1] / 1000000);
                const senderB64 = encodeHashToBase64(rawSignal.sender);
                const chatSignal: GlobalChatMessageSignal = {
                    type: "GlobalChatMessage",
                    sender: senderB64,
                    content: rawSignal.content,
                    timestamp: messageTimestamp,
                };
                addChatMessage(chatSignal);
            }
            else {
                console.warn("[App.svelte handleSignal] Malformed GlobalChatMessage signal received:", rawSignal);
            }
          } else if (actualSignal.type === "GameAbandoned") {
              const { game_id: abandonedGameId, abandoned_by_player } = actualSignal as GameAbandonedSignal;
              const currentLocalGameId = get(currentGame);
              if (currentLocalGameId && abandonedGameId && encodeHashToBase64(abandonedGameId) === encodeHashToBase64(currentLocalGameId)) {
                  getOrFetchProfile(client, abandoned_by_player).then(profile => {
                      if (profile && profile.nickname) {
                          opponentWhoLeftNickname = profile.nickname;
                      } else {
                          opponentWhoLeftNickname = truncatePubkey(abandoned_by_player);
                      }
                      opponentWhoLeftAgentKeyB64 = encodeHashToBase64(abandoned_by_player);
                      showOpponentLeftPopup = true;
                  }).catch(profileError => {
                      console.error("[App.svelte handleSignal] Error fetching profile for opponent who left:", profileError);
                      opponentWhoLeftNickname = truncatePubkey(abandoned_by_player);
                      opponentWhoLeftAgentKeyB64 = encodeHashToBase64(abandoned_by_player);
                      showOpponentLeftPopup = true;
                  });
                  currentGame.set(null);
                  currentRoute.set("dashboard");
                  invitations.set([]); 
              }
          }
      }
  }


  // --- Event Handlers ---
  function handleJoinGame(event: CustomEvent<{ gameHash: ActionHash }>) {
    invitations.set([]);
  }

  function handleRegistration() { /* For WelcomePopup */ }

  async function handleAcceptInvitation(event: CustomEvent<{ gameId: string | ActionHash }>) {
    const gameHash: ActionHash = typeof event.detail.gameId === "string" ? decodeHashFromBase64(event.detail.gameId) : event.detail.gameId;
    removeInvitation(gameHash);
    loading = true;
    invitationError = null;
    try {
      await client.callZome({
        cap_secret: null, role_name : HOLOCHAIN_ROLE_NAME, zome_name : HOLOCHAIN_ZOME_NAME,
        fn_name   : "accept_invitation", payload   : { game_id: gameHash }
      });
    } catch (e: any) {
      console.error("accept_invitation error:", e);
      invitationError = e.data?.data || e.message || "Failed to accept invitation.";
    } finally {
      loading = false;
    }
  }

  function handleDeclineInvitation(gameIdToDecline: ActionHash) {
      removeInvitation(gameIdToDecline);
      invitationError = null;
  }

  function exitGame() {
      currentGame.set(null);
      currentRoute.set("dashboard");
      invitations.set([]);
      if (dashboardComponent && typeof dashboardComponent.refreshLeaderboardData === 'function') {
        setTimeout(() => {
          if (dashboardComponent && typeof dashboardComponent.refreshLeaderboardData === 'function') {
            dashboardComponent.refreshLeaderboardData();
          }
        }, 0);
      }
  }

  function handleViewStats(event: CustomEvent<{ gameId: ActionHash }>) {
    if (event.detail.gameId) {
        showStatsDashboardForGameId = event.detail.gameId;
        // Optionally, can also set currentRoute to something like "stats-view" if that helps manage UI state
        // currentRoute.set("stats-view"); // Or keep route as "dashboard" and show stats as an overlay
    }
  }


  // --- Lifecycle Hooks ---
  onMount(async () => {
    try {
      loading = true;
      client = await appClientContext.getClient();
      if (client) {
          unsubscribeFromSignals = client.on("signal", handleSignal);
          await checkAndLoadExistingProfile(client);
      }
      presenceIntervalId = setInterval(publishPresence, 15000);
    } catch (e) { 
      console.error("Failed to initialize Holochain client or load profile:", e);
      error = e as HolochainError;
    }
    finally { 
      loading = false; 
    }
  });

  onDestroy(() => {
      if (unsubscribeFromSignals) { unsubscribeFromSignals(); }
      if (presenceIntervalId) { clearInterval(presenceIntervalId); }
  });

  setContext(clientContext, appClientContext);

  const isRegistered = derived(playerProfile, ($p) => $p !== null);
  let route: string; currentRoute.subscribe((value) => { route = value || 'dashboard'; });
  let gameIdProp: ActionHash | null = null; currentGame.subscribe((value) => { gameIdProp = value; }); // Renamed to avoid conflict
  let currentPlayerProfile: { nickname: string; agentKey: AgentPubKey } | null; playerProfile.subscribe((value) => { currentPlayerProfile = value; });

  let currentInvitationToShow: GameInvitationSignal | null = null;
  invitations.subscribe(invList => {
      if (invList.length > 0) {
        if (!currentInvitationToShow || encodeHashToBase64(currentInvitationToShow.game_id) !== encodeHashToBase64(invList[0].game_id)) {
          invitationError = null;
        }
        currentInvitationToShow = invList[0];
      } else {
        currentInvitationToShow = null;
        invitationError = null;
      }
  });

</script>

{#if loading} <main><p>Connecting to Holochain...</p></main>
{:else if error} <main> <p>Error Connecting: {error.message}</p> <p>Please ensure the Holochain conductor is running...</p> </main>
{:else if !$isRegistered}
  <WelcomePopup on:registered={handleRegistration} />
{:else}
  <main class="app-main">
    {#if currentPlayerProfile}
      <header class="user-header">
        <p><strong>Name:</strong> {currentPlayerProfile.nickname}</p>
        <p><strong>Agent Key:</strong> {truncatePubkey(currentPlayerProfile.agentKey)}</p>
      </header>
    {/if}

    {#if currentInvitationToShow}
       {@const inviterName = truncatePubkey(currentInvitationToShow.inviter)}
       {@const gameIdString = encodeHashToBase64(currentInvitationToShow.game_id)}
       {@const gameIdObject = currentInvitationToShow.game_id}
       <InvitationPopup
         inviter={inviterName}
         gameId={gameIdString}
         error={invitationError}
         on:accept={(e) => handleAcceptInvitation(e)}
         on:decline={() => handleDeclineInvitation(gameIdObject)}
       />
    {/if}

    {#if route === "dashboard" && !showStatsDashboardForGameId}
      <Dashboard on:join-game={handleJoinGame} bind:this={dashboardComponent} />
    {:else if route === "gameplay" && !showStatsDashboardForGameId}
       {#if currentPlayerProfile?.agentKey && gameIdProp}
           <PongGame
             gameId={gameIdProp}
             playerKey={currentPlayerProfile.agentKey}
             on:exit-game={exitGame}
             on:view-stats={handleViewStats}
           />
       {:else}
           <p>Loading game data or missing information...</p>
           <button on:click={exitGame}>Back to Dashboard</button>
       {/if}
    {:else if showStatsDashboardForGameId}
        <StatisticsDashboard
            gameId={showStatsDashboardForGameId}
            on:close={() => {
                showStatsDashboardForGameId = null;
                // currentRoute.set("dashboard"); // Optionally force route back to dashboard
            }}
        />
    {:else}
       <Dashboard on:join-game={handleJoinGame} bind:this={dashboardComponent} />
       {() => { if (route !== 'dashboard') { console.warn(`Unknown route: ${route}, defaulting.`); setTimeout(() => currentRoute.set('dashboard'), 0); } return ''; }}
    {/if}

    {#if showOpponentLeftPopup && opponentWhoLeftNickname && opponentWhoLeftAgentKeyB64}
      <OpponentLeftPopup
        opponentNickname={opponentWhoLeftNickname}
        opponentAgentKeyB64={opponentWhoLeftAgentKeyB64}
        on:dismissed={() => showOpponentLeftPopup = false}
      />
    {/if}
  </main>
{/if}
<style>
  .app-main {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 1rem;
    min-height: 90vh; /* Ensure it takes up most of the viewport height */
  }

  .user-header {
    width: 100%;
    max-width: 800px; /* Max width for header content */
    background-color: #f0f0f0;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    box-shadow: 0 2px 4px rgba(0,0,0,0.1);
    margin-bottom: 1.5rem;
    text-align: center;
  }
  .user-header p {
    margin: 0.3rem 0;
    font-size: 0.9rem;
    color: #333;
  }
  .user-header strong {
    color: #000;
  }

  /* General styling for popups or important messages */
  .error-message {
    color: red;
    background-color: #ffe0e0;
    padding: 10px;
    border-radius: 5px;
    margin-bottom: 1rem;
  }

  main > p { /* For loading/error messages directly in main */
    font-size: 1.2rem;
    color: #555;
    margin-top: 3rem;
  }
</style>
