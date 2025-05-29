<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { decodeHashFromBase64 } from '@holochain/client';

  export let inviter: string;           // already base-64
  export let gameId: string;            // base-64 **string**

  const dispatch = createEventDispatcher();

  /** user clicks ✅ */
  function acceptInvitation () {
    // convert the base-64 string back to the ActionHash bytes
    const hash = decodeHashFromBase64(gameId);
    dispatch('accept', { gameId: hash });   // <-- detail *is* the raw hash
  }

  /** user clicks ❌ */
  function declineInvitation () {
    dispatch('decline');                    // no payload needed
  }
</script>

  <div class="invitation-popup">
    <h3>Game Invitation</h3>
    <p>You have been invited by <strong>{inviter}</strong> to join a game.</p>
    <div class="invitation-popup-buttons">
      <button on:click={acceptInvitation}>Accept</button>
      <button on:click={declineInvitation}>Decline</button>
    </div>
  </div>