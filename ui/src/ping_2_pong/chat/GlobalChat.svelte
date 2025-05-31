<script lang="ts">
  import { onMount, getContext, onDestroy } from 'svelte';
  import { globalChatMessages } from '../../stores/chatStore';
  import { clientContext, type ClientContext } from '../../contexts'; // Added ClientContext for typing
  import type { AppClient, AgentPubKeyB64 } from '@holochain/client'; // Added AgentPubKeyB64
  import { truncatePubkey } from '../../utils';
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from '../../holochainConfig';
  import { writable, get as getStoreValue } from 'svelte/store'; // Added Svelte store imports
  import { getOrFetchProfile, type DisplayProfile } from '../../stores/profilesStore'; // Import profile store

  let messageContent: string = "";
  let chatBox: HTMLElement; // For auto-scrolling
  let unsubscribeFromStore: (() => void) | undefined;

  let sendError: string | null = null;
  let isSending: boolean = false;

  let client: AppClient; // To be initialized in onMount
  const appClientContext = getContext<ClientContext>(clientContext); // Typed getContext

  // Store for fetched sender profiles
  let senderProfiles = writable<Map<AgentPubKeyB64, DisplayProfile | null>>(new Map());

  // Reactive block to fetch profiles for new senders
  $: if ($globalChatMessages && client) {
    const currentProfiles = getStoreValue(senderProfiles);
    for (const msg of $globalChatMessages) {
      if (!currentProfiles.has(msg.sender)) {
        // Set to null initially to indicate loading / prevent multiple fetches
        senderProfiles.update(m => {
          const newMap = new Map(m);
          newMap.set(msg.sender, null);
          return newMap;
        });
        getOrFetchProfile(client, msg.sender).then(profile => {
          if (profile) {
            senderProfiles.update(m => {
              const newMap = new Map(m);
              newMap.set(msg.sender, profile);
              return newMap;
            });
          }
          // If profile is null (error or not found), it remains null in the map,
          // which will cause fallback to truncatePubkey in the template.
        });
      }
    }
  }

  // Removed old getClient() as client is now initialized in onMount and passed around.
  // The sendMessage function will use the module-level 'client'.

  async function sendMessage() {
    if (!messageContent.trim()) return;
    isSending = true;
    sendError = null;
    try {
      const client = await getClient(); // Ensure client is resolved
      await client.callZome({
        cap_secret: null,
        role_name: HOLOCHAIN_ROLE_NAME,
        zome_name: HOLOCHAIN_ZOME_NAME,
        fn_name: "send_global_chat_message",
        payload: messageContent,
      });
      messageContent = ""; // Clear message content on success
    } catch (e: any) {
      console.error("Error sending chat message:", e);
      sendError = e.data?.data || e.message || "Failed to send message. Please try again.";
    } finally {
      isSending = false;
    }
  }

  // Clear error when user starts typing
  $: if (messageContent && sendError) {
    sendError = null;
  }

  function formatTimestamp(timestamp: number): string {
    if (!timestamp) return ""; // Handle cases where timestamp might be undefined or 0
    return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
  }

  // Basic auto-scroll
  function scrollToBottom() {
    if (chatBox) {
      // Use requestAnimationFrame to wait for DOM updates before scrolling
      requestAnimationFrame(() => {
        chatBox.scrollTop = chatBox.scrollHeight;
      });
    }
  }

  onMount(async () => { // Made onMount async
    client = await appClientContext.getClient(); // Initialize client

    // Scroll to bottom when component mounts and when messages change
    unsubscribeFromStore = globalChatMessages.subscribe((messages) => {
      if (messages.length > 0) { // Only scroll if there are messages
        scrollToBottom();
      }
    });
    // Initial scroll attempt, useful if messages are already loaded
    // Ensure chatBox is rendered before scrolling
    setTimeout(scrollToBottom, 50);
  });

  onDestroy(() => {
    if (unsubscribeFromStore) {
      unsubscribeFromStore();
    }
  });

  // Helper to truncate sender pubkey for display - REMOVED, using imported version
  // function truncatePubkey(pubkey: string): string {
  //   if (!pubkey || typeof pubkey !== 'string') return "anonymous"; // Handle undefined or non-string pubkeys
  //   // Assuming pubkey is Base64. A typical Holochain AgentPubKey (uCA...) is longer.
  //   // Adjust slicing if needed based on actual pubkey format and length.
  //   const prefixLength = 8;
  //   const suffixLength = 6;
  //   if (pubkey.length <= prefixLength + suffixLength + 3) return pubkey; // Don't truncate if too short
  //   return pubkey.slice(0, prefixLength) + "..." + pubkey.slice(-suffixLength);
  // }

</script>

<div class="global-chat-placeholder">
  <h4>Global Chat</h4>
  <div class="chat-messages-placeholder" bind:this={chatBox}>
    {#each $globalChatMessages as msg (msg.timestamp.toString() + msg.sender)}
      {@const profile = $senderProfiles.get(msg.sender)}
      <p>
        <span title={msg.sender} class="sender"> <!-- Added class="sender" for consistent styling if needed -->
          {profile?.nickname || truncatePubkey(msg.sender, 4, 4)}:
        </span>
        <!-- Message content will be styled by '.chat-messages-placeholder p' -->
        {msg.content}
        <span class="chat-timestamp">{formatTimestamp(msg.timestamp)}</span>
      </p>
    {:else}
      <!-- This paragraph will inherit styles from '.chat-messages-placeholder p' and can be centered with a utility class if needed -->
      <p class="text-center">
        No messages yet. Be the first to say something!
      </p>
    {/each}
  </div>
   <!-- Form styled to lay out input and button horizontally, using global styles for elements -->
   <form on:submit|preventDefault={sendMessage} style="display: flex; flex-direction: column; gap: 8px; margin-top: 1rem;">
     <div style="display: flex; gap: 8px; align-items: center;">
       <input type="text" bind:value={messageContent} placeholder="Type a message..." aria-label="Chat message input" style="flex-grow: 1; margin: 0;" disabled={isSending} />
       <button type="submit" disabled={isSending}>
         {#if isSending}Sending...{:else}Send{/if}
       </button>
     </div>
     {#if sendError}
       <p class="error-message" style="margin: 0.5rem 0 0 0; padding: 0.5em;">{sendError}</p>
     {/if}
   </form>
</div>

<!-- The <style> block has been removed entirely. -->
