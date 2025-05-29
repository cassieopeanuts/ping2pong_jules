<script lang="ts">
  import { onMount, getContext, onDestroy } from 'svelte';
  import { globalChatMessages } from '../../stores/chatStore'; // Adjust path if necessary, seems correct
  import { clientContext } from '../../contexts'; // Adjust path if necessary, seems correct
  import type { AppClient } from '@holochain/client';
  // GlobalChatMessageSignal is implicitly used by $globalChatMessages store, explicit import not strictly needed here for that
  // but good for clarity if we were to type a variable with it.
  // import type { GlobalChatMessageSignal } from '../ping_2_pong/types'; // Adjust path if necessary, seems correct

  let messageContent: string = "";
  let chatBox: HTMLElement; // For auto-scrolling
  let unsubscribeFromStore: (() => void) | undefined;

  // Correctly get the Svelte context value
  const appClientContextValue = getContext(clientContext);

  async function getClient(): Promise<AppClient> {
    // The context might store the client directly or a promise/function that resolves to it
    // Based on typical patterns, it's often an object with a getClient method or the client itself.
    // The example used `(appClientContext as any).getClient()`. Let's refine this a bit
    // if `clientContext` provides `AppClient | Promise<AppClient>` or an object like `{ getClient: () => Promise<AppClient> }`
    if (typeof (appClientContextValue as any)?.getClient === 'function') {
      return await (appClientContextValue as any).getClient();
    } else if (appClientContextValue instanceof Promise) {
      return await appClientContextValue;
    } else if (typeof (appClientContextValue as any)?.callZome === 'function') { // It might be the client itself
      return appClientContextValue as AppClient;
    }
    throw new Error("AppClient could not be retrieved from context.");
  }

  async function sendMessage() {
    if (!messageContent.trim()) return;
    
    try {
      const client = await getClient(); // Ensure client is resolved
      await client.callZome({
        cap_secret: null,
        role_name: "ping_2_pong", // Ensure this matches your DNA role_name
        zome_name: "ping_2_pong", // Ensure this matches your Zome name
        fn_name: "send_global_chat_message",
        payload: messageContent,
      });
      messageContent = "";
    } catch (e) {
      console.error("Error sending chat message:", e);
      alert("Failed to send message. See console for details."); // User feedback
    }
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

  onMount(() => {
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

  // Helper to truncate sender pubkey for display
  function truncatePubkey(pubkey: string): string {
    if (!pubkey || typeof pubkey !== 'string') return "anonymous"; // Handle undefined or non-string pubkeys
    // Assuming pubkey is Base64. A typical Holochain AgentPubKey (uCA...) is longer.
    // Adjust slicing if needed based on actual pubkey format and length.
    const prefixLength = 8;
    const suffixLength = 6;
    if (pubkey.length <= prefixLength + suffixLength + 3) return pubkey; // Don't truncate if too short
    return pubkey.slice(0, prefixLength) + "..." + pubkey.slice(-suffixLength);
  }

</script>

<div class="chat-container">
  <div class="chat-messages" bind:this={chatBox}>
    {#each $globalChatMessages as msg (msg.timestamp.toString() + msg.sender)}
      <div class="message">
        <span class="sender" title={msg.sender}>{truncatePubkey(msg.sender)}:</span>
        <span class="content">{msg.content}</span>
        <span class="timestamp">{formatTimestamp(msg.timestamp)}</span>
      </div>
    {:else}
      <p class="no-messages">No messages yet. Be the first to say something!</p>
    {/each}
  </div>
  <form class="chat-input" on:submit|preventDefault={sendMessage}>
    <input type="text" bind:value={messageContent} placeholder="Type a message..." aria-label="Chat message input" />
    <button type="submit">Send</button>
  </form>
</div>

<style>
  .chat-container {
    display: flex;
    flex-direction: column;
    height: 300px; /* Example height, adjust as needed */
    border: 1px solid #ccc;
    border-radius: 4px;
    overflow: hidden;
    font-family: Arial, sans-serif; /* Common sans-serif font */
    width: 100%; 
    max-width: 500px; /* Max width for larger screens */
    margin: 10px auto; /* Center component if it has max-width */
    box-shadow: 0 2px 5px rgba(0,0,0,0.1);
  }

  .chat-messages {
    flex-grow: 1;
    overflow-y: auto;
    padding: 10px;
    background-color: #f9f9f9;
    display: flex;
    flex-direction: column; 
  }

  .message {
    margin-bottom: 8px;
    padding: 6px 10px;
    border-radius: 15px; /* More rounded bubbles */
    background-color: #e0e0e0; /* Light grey for messages */
    color: #333;
    word-wrap: break-word; 
    max-width: 80%; /* Messages don't take full width */
    align-self: flex-start; /* Align to left by default */
  }

  /* Example: differentiate sender's own messages (if myPubKey is available) */
  /* .message.own {
    background-color: #007bff; 
    color: white;
    align-self: flex-end;
  } */

  .message .sender {
    font-weight: bold;
    margin-right: 8px; /* Increased spacing */
    color: #0056b3; /* Darker blue for sender */
  }
  .message .content {
     color: #111;
  }

  .message .timestamp {
    font-size: 0.7em; /* Slightly smaller timestamp */
    color: #666; /* Lighter grey for timestamp */
    margin-left: 10px;
    display: inline-block; /* Keep on same line if space allows */
    margin-top: 2px;
  }
  
  .no-messages {
    color: #888;
    text-align: center;
    margin-top: 20px;
    flex-grow: 1; /* Center vertically if no messages */
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .chat-input {
    display: flex;
    padding: 10px;
    border-top: 1px solid #ccc;
    background-color: #f0f0f0; /* Slightly different background for input area */
  }

  .chat-input input {
    flex-grow: 1;
    padding: 10px; /* More padding */
    border: 1px solid #ddd;
    border-radius: 20px; /* Rounded input */
    margin-right: 10px;
    font-size: 0.9em;
  }

  .chat-input button {
    padding: 10px 18px; /* More padding */
    border: none;
    background-color: #007bff;
    color: white;
    border-radius: 20px; /* Rounded button */
    cursor: pointer;
    font-size: 0.9em;
    transition: background-color 0.2s ease;
  }

  .chat-input button:hover {
    background-color: #0056b3;
  }
</style>
