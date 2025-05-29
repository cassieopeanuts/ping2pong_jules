<script lang="ts">
  import { createEventDispatcher, getContext } from "svelte";
  import { playerProfile } from "../stores/playerProfile";
  import { clientContext, type ClientContext } from "../contexts";
  import type { AppClient, AgentPubKey } from "@holochain/client";
  // Remove encodeHashToBase64 if not needed here
  // import { encodeHashToBase64 } from "@holochain/client";

  const dispatch = createEventDispatcher();
  let nickname: string = "";
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  async function register() {
    if (nickname.trim() === "") { /* ... alert ... */ return; }
    try {
      client = await appClientContext.getClient();
      const agentKey: AgentPubKey = client.myPubKey; // Get the raw AgentPubKey

      const playerPayload = { player_key: agentKey, player_name: nickname.trim() };

      const record = await client.callZome({
        cap_secret: null, role_name: "ping_2_pong", zome_name: "ping_2_pong",
        fn_name: "create_player", payload: playerPayload,
      });
      console.log("Player created:", record);

      // FIX: Set profile store with the raw AgentPubKey
      playerProfile.set({
        agentKey: agentKey, // Store the object/Uint8Array
        nickname: nickname.trim()
      });

      dispatch("registered", { nickname: nickname.trim() });
    } catch (e) { /* ... error handling ... */ }
  }
</script>

<div class="welcome-popup">
  <div class="welcome-popup-content">
    <h2>Welcome! Let's Pong to Ping!</h2>
    <input type="text" placeholder="Enter your nickname" bind:value={nickname} />
    <button on:click={register}>Register</button>
  </div>
</div>
