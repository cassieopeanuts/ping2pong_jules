<script lang="ts">
  import { onMount, onDestroy, getContext, createEventDispatcher } from "svelte";
  // Import types from holochain/client
  import type { AppClient, ActionHash, AgentPubKey, Record, Entry } from "@holochain/client";
  import { encodeHashToBase64 } from "@holochain/client";
  // Import local types and context
  import { clientContext, type ClientContext } from "../../contexts";
  import { decode } from "@msgpack/msgpack";
  // Import local types
  import type {
    Game,
    GameStatus,
    UpdateGameInput,
    PaddleUpdatePayload,
    BallUpdatePayload,
    ScoreUpdatePayload,
    GameOverPayload,
    Timestamp,
    CreateScoreOutput,   // Added CreateScoreOutput
    GetScoreOutput,      // Added GetScoreOutput
    GameStats            // Added GameStats for payload
  } from "../ping_2_pong/types";
  import { getOrFetchProfile, type DisplayProfile } from "../../stores/profilesStore";
  import { HOLOCHAIN_ROLE_NAME, HOLOCHAIN_ZOME_NAME } from "../../holochainConfig";

  // Create dispatcher to send events up to the parent (App.svelte)
  const dispatch = createEventDispatcher();

  // Component Props passed from App.svelte
  export let gameId: ActionHash; // The ORIGINAL ActionHash of the game
  export let playerKey: AgentPubKey; // The current user's public key

  // Holochain Client
  let client: AppClient;
  const appClientContext = getContext<ClientContext>(clientContext);

  // Game Constants
  const CANVAS_WIDTH = 800;
  const CANVAS_HEIGHT = 600;
  const PADDLE_WIDTH = 10;
  const PADDLE_HEIGHT = 100;
  const BALL_RADIUS = 10;
  const WINNING_SCORE = 10;
  const PADDLE_SPEED = 25;
  const UPDATE_INTERVAL = 50; // ms interval for sending signal updates

  // Component State
  let gameRecord: Record | undefined; // Stores the latest fetched Holochain record for the game
  let liveGame: Game | undefined; // Stores the deserialized Game data from the entry (set only when ready)
  let isPlayer1 = false; // Flag indicating if the current user is Player 1
  let isPlayer2 = false; // Flag indicating if the current user is Player 2
  let paddle1Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2; // Player 1 paddle Y position
  let paddle2Y = CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2; // Player 2 paddle Y position
  let ball = { x: CANVAS_WIDTH / 2, y: CANVAS_HEIGHT / 2, dx: 5, dy: 5 }; // Ball position and velocity
  let score = { player1: 0, player2: 0 }; // Current scores
  let gameOver = false; // Flag indicating if the game has ended
  let winner: AgentPubKey | null = null; // Stores the winner's public key if game is over
  let errorMsg: string | null = null; // Stores any error message for display
  let loadingMsg: string | null = "Initializing game..."; // Loading message

  // Player Profiles
  let player1Profile: DisplayProfile | null = null;
  let player2Profile: DisplayProfile | null = null;

  // Canvas & Animation
  let canvas: HTMLCanvasElement; // Reference to the canvas element
  let ctx: CanvasRenderingContext2D; // Canvas 2D rendering context
  let animationFrameId: number; // ID for the requestAnimationFrame loop

  // Signal Handling & Metrics
  let unsubscribeFromSignals: (() => void) | undefined; // Function to unsubscribe from signal listener
  let lastPaddleUpdate = 0; // Timestamp of the last paddle update sent
  let lastBallUpdate = 0; // Timestamp of the last ball update sent
  let RTTs: number[] = []; // For storing Round Trip Times (latency)
  let collectedTimeToWriteScoreMs: number = 0; // For storing time to write score
  let collectedTimeToReadScoreMs: number = 0; // For storing time to read score


  // Retry mechanism state
  let retryTimeoutId: ReturnType<typeof setTimeout> | undefined;
  let retryCount = 0;
  const MAX_RETRIES = 5; // e.g., try 5 times
  const RETRY_DELAY = 1000; // 1 second delay

  // --- Helper Functions ---

  // Shortens a public key for display purposes
  function truncatePubkey(pubkey: AgentPubKey | null | undefined): string {
    if (!pubkey) return "N/A";
    try {
      const base64 = encodeHashToBase64(pubkey);
      return base64.slice(0, 8) + "..." + base64.slice(-6);
    } catch (e) {
        console.error("Error encoding pubkey:", e);
        return "Error";
    }
  }

  // --- Core Functions ---

  // Fetches the latest game state, returns the Game object or null if not ready/error
  async function fetchGameState(): Promise<Game | null> {
    // Don't clear errorMsg here, initializeGame handles status display
    if (!client || !gameId) {
        console.error("[PongGame fetchGameState] Client or Game ID missing.");
        errorMsg = "Client/Game ID missing"; // Set error for display
        return null;
    }
    try {
      console.log(`[PongGame fetchGameState] Attempting fetch for game: ${encodeHashToBase64(gameId)}`);
      // Call the zome function to get the latest game record based on the original hash
      const result: Record | null = await client.callZome({
        cap_secret: null,
        role_name: HOLOCHAIN_ROLE_NAME,
        zome_name: HOLOCHAIN_ZOME_NAME,
        fn_name: "get_latest_game", // Gets the record associated with the latest update action
        payload: gameId, // Pass the original game hash
      });

      if (result) {
        gameRecord = result; // Store latest record
        const recordEntry = result.entry;
        let actualEntry: Entry | undefined = undefined;
        // Safely extract the Entry object from the Record
        if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
             const presentEntry = (recordEntry as { Present: Entry }).Present;
             if (presentEntry) actualEntry = presentEntry;
        }

        // Ensure we have a valid App entry containing Uint8Array data
        if (actualEntry && actualEntry.entry_type === 'App' && actualEntry.entry instanceof Uint8Array) {
            try {
                // Decode the MessagePack bytes into a Game object
                const decodedGame = decode(actualEntry.entry) as Game;
                console.log("[PongGame fetchGameState] Decoded game state:", decodedGame);
                // *** Check if game is ready (InProgress and Player 2 exists) ***
                if (decodedGame.game_status === 'InProgress' && decodedGame.player_2) {
                    console.log("[PongGame fetchGameState] Game state is InProgress with Player 2. Ready.");
                    return decodedGame; // Return the ready game state
                } else {
                    console.log(`[PongGame fetchGameState] Game state not ready yet (Status: ${decodedGame.game_status}, P2: ${decodedGame.player_2 ? 'Set' : 'Null'}). Will retry.`);
                    return null; // Indicate not ready
                }
            } catch (decodeError) {
                // Handle errors during MessagePack decoding
                console.error("[PongGame fetchGameState] Failed to decode entry:", decodeError);
                errorMsg = "Failed to decode game data";
                return null; // Error decoding
            }
        } else {
            // Handle cases where the entry data is missing or not in the expected format
            console.error("[PongGame fetchGameState] Could not extract valid App entry.", result);
            errorMsg = "Invalid game record structure";
            return null; // Invalid entry structure
        }
      } else {
          // Handle case where the game record itself wasn't found
          console.warn(`[PongGame fetchGameState] Failed to fetch record for gameId: ${encodeHashToBase64(gameId)}. Maybe DHT delay?`);
          // Don't set errorMsg yet, retry might succeed
          return null; // Record not found (could be DHT delay)
      }
    } catch (e) {
      // Handle errors during the zome call
      console.error("[PongGame fetchGameState] Error fetching game state:", e);
      errorMsg = `Error fetching game: ${(e as Error).message}`;
      return null; // Zome call error
    }
  }

  // Initializes the game, retrying fetchGameState if needed
  async function initializeGame() {
      console.log(`[PongGame initializeGame] Starting initialization. Retry count: ${retryCount}`);
      loadingMsg = `Initializing game... (Attempt ${retryCount + 1})`;
      errorMsg = null; // Clear previous errors

      const fetchedGame = await fetchGameState();

      if (fetchedGame) {
          // --- Game Ready ---
          loadingMsg = null; // Clear loading message
          liveGame = fetchedGame; // Set the live game state

          // Identify players based on the confirmed state
          const myPubKeyB64 = encodeHashToBase64(playerKey);
          isPlayer1 = encodeHashToBase64(liveGame.player_1) === myPubKeyB64;
          // We know player_2 exists because we checked for it in fetchGameState
          isPlayer2 = encodeHashToBase64(liveGame.player_2!) === myPubKeyB64;
          console.log(`[PongGame initializeGame] Player role identified: isPlayer1=${isPlayer1}, isPlayer2=${isPlayer2}`);

          // Fetch profiles
          if (liveGame.player_1) {
            getOrFetchProfile(client, liveGame.player_1).then(profile => player1Profile = profile);
          }
          if (liveGame.player_2) {
            getOrFetchProfile(client, liveGame.player_2).then(profile => player2Profile = profile);
          }

          // Initialize positions (only if score is 0)
          if (score.player1 === 0 && score.player2 === 0) {
              paddle1Y = liveGame.player_1_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
              paddle2Y = liveGame.player_2_paddle ?? (CANVAS_HEIGHT / 2 - PADDLE_HEIGHT / 2);
              ball.x = liveGame.ball_x ?? (CANVAS_WIDTH / 2);
              ball.y = liveGame.ball_y ?? (CANVAS_HEIGHT / 2);
              ball.dx = 5 * (Math.random() > 0.5 ? 1 : -1);
              ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
              console.log("[PongGame initializeGame] Initialized positions.");
          }

          // Start the game loop and listeners
          startGameLoop();

      } else if (retryCount < MAX_RETRIES) {
          // --- Game Not Ready, Retry ---
          retryCount++;
          console.log(`[PongGame initializeGame] Game not ready, scheduling retry #${retryCount} in ${RETRY_DELAY}ms`);
          retryTimeoutId = setTimeout(initializeGame, RETRY_DELAY); // Schedule next attempt
      } else {
          // --- Max Retries Reached ---
          console.error(`[PongGame initializeGame] Failed to fetch ready game state after ${MAX_RETRIES + 1} attempts.`);
          loadingMsg = null; // Clear loading message
          errorMsg = "Failed to load game state after multiple attempts. Please exit and try again.";
          // Keep drawing to show the error message
          if (ctx) draw();
      }
  }

  // Starts the main game loop and sets up listeners
  function startGameLoop() {
      if (!ctx) {
          console.error("[PongGame startGameLoop] Canvas context not available!");
          errorMsg = "Canvas failed to initialize.";
          return;
      }
      if (animationFrameId) {
          console.warn("[PongGame startGameLoop] Game loop already running?");
          return; // Avoid starting multiple loops
      }
      console.log("[PongGame startGameLoop] Starting game loop and listeners.");
      gameOver = false; // Ensure game isn't marked over
      RTTs = []; // Reset RTTs for the new game
      collectedTimeToWriteScoreMs = 0;
      collectedTimeToReadScoreMs = 0;
      draw(); // Start drawing loop
      window.addEventListener("keydown", handleKeyDown); // Listen for keyboard input
      unsubscribeFromSignals = subscribeToGameSignals(); // Subscribe to game signals
  }


  // Handles keyboard input ('ArrowUp', 'ArrowDown', 'w', 's') for paddle movement
  function handleKeyDown(e: KeyboardEvent) {
    if (gameOver || !liveGame) return; // Ignore input if game is over or not loaded

    let moved = false; // Flag to track if the paddle actually moved
    // Player 1 controls
    if (isPlayer1) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle1Y = Math.max(0, paddle1Y - PADDLE_SPEED); // Move up, clamp at top
        moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle1Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle1Y + PADDLE_SPEED); // Move down, clamp at bottom
        moved = true;
      }
    // Player 2 controls
    } else if (isPlayer2) {
      if (e.key === "ArrowUp" || e.key === "w" || e.key === "W") {
        paddle2Y = Math.max(0, paddle2Y - PADDLE_SPEED); // Move up, clamp at top
        moved = true;
      } else if (e.key === "ArrowDown" || e.key === "s" || e.key === "S") {
        paddle2Y = Math.min(CANVAS_HEIGHT - PADDLE_HEIGHT, paddle2Y + PADDLE_SPEED); // Move down, clamp at bottom
        moved = true;
      }
    }
    // If the paddle moved, send an update signal
    if (moved) sendPaddleUpdate();
  }

  // Sends the current player's paddle position update signal to the backend
  async function sendPaddleUpdate() {
    // Throttle updates to prevent sending too many signals
    const now = Date.now();
    if (gameOver || !client || !liveGame || !gameId || (now - lastPaddleUpdate < UPDATE_INTERVAL)) return;
    lastPaddleUpdate = now; // Update timestamp of last sent signal

    const payload: PaddleUpdatePayload = {
        game_id: gameId,
        paddle_y: Math.round(isPlayer1 ? paddle1Y : paddle2Y),
        sent_at: [Math.floor(Date.now() / 1000), (Date.now() % 1000) * 1_000_000] as Timestamp
    };

    try {
      await client.callZome({
          cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME,
          fn_name: "send_paddle_update",
          payload: payload
      });
    } catch (e) { console.error("Error sending paddle update signal:", e); }
  }

  // Sends the current ball position and velocity update signal (only Player 1 does this)
  async function sendBallUpdate() {
    const now = Date.now();
    if (gameOver || !isPlayer1 || !client || !liveGame || !gameId || (now - lastBallUpdate < UPDATE_INTERVAL)) return;
    lastBallUpdate = now;

    const payload: BallUpdatePayload = {
        game_id: gameId,
        ball_x: Math.round(ball.x),
        ball_y: Math.round(ball.y),
        ball_dx: Math.round(ball.dx),
        ball_dy: Math.round(ball.dy),
        sent_at: [Math.floor(Date.now() / 1000), (Date.now() % 1000) * 1_000_000] as Timestamp
    };

    try {
      await client.callZome({
          cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME,
          fn_name: "send_ball_update",
          payload: payload
      });
    } catch (e) { console.error("Error sending ball update signal:", e); }
  }

  async function sendScoreUpdate() {
    if (!client || !liveGame) return;
    try {
      const payload: ScoreUpdatePayload = {
          game_id: gameId,
          score1 : score.player1,
          score2 : score.player2,
          sent_at: [Math.floor(Date.now() / 1000), (Date.now() % 1000) * 1_000_000] as Timestamp
      };
      await client.callZome({
        cap_secret: null,
        role_name : HOLOCHAIN_ROLE_NAME,
        zome_name : HOLOCHAIN_ZOME_NAME,
        fn_name   : "send_score_update",
        payload: payload
      });
    } catch (e) { console.error("Score update failed:", e); }
  }

  // Sets up the listener for incoming signals related to this specific game
  function subscribeToGameSignals() {
    if (!client) return;

    return client.on("signal", (raw: any) => {
      const s = raw?.App?.payload;
      if (!s || !s.type || gameOver) return;

      let signalGameIdB64 = "";
      if (s.game_id instanceof Uint8Array) {
        signalGameIdB64 = encodeHashToBase64(s.game_id);
      } else if (typeof s.game_id === 'string') {
        signalGameIdB64 = s.game_id;
      } else {
        console.warn("Received signal with unknown game_id format:", s.game_id);
        return;
      }
      if (signalGameIdB64 !== encodeHashToBase64(gameId)) return;

      const meB64 = encodeHashToBase64(playerKey);
      const receivedAt = Date.now();

      try {
        if (s.sent_at && Array.isArray(s.sent_at) && s.sent_at.length === 2) {
            const sentAtMs = s.sent_at[0] * 1000 + Math.floor(s.sent_at[1] / 1_000_000);
            const latencyMs = receivedAt - sentAtMs;
            RTTs.push(latencyMs);
        }

        switch (s.type) {
          case "PaddleUpdate":
            let paddlePlayerB64 = "";
            if (s.player instanceof Uint8Array) {
                paddlePlayerB64 = encodeHashToBase64(s.player);
            } else if (typeof s.player === 'string') {
                paddlePlayerB64 = s.player;
            }
            if (paddlePlayerB64 !== meB64) {
              if (isPlayer1) paddle2Y = s.paddle_y;
              else paddle1Y = s.paddle_y;
            }
            break;
          case "BallUpdate":
            if (!isPlayer1) {
              ball.x = s.ball_x; ball.y = s.ball_y;
              ball.dx = s.ball_dx; ball.dy = s.ball_dy;
            }
            break;
          case "ScoreUpdate":
            score.player1 = s.score1;
            score.player2 = s.score2;
            break;
          case "GameOver":
            let winnerPubKey: AgentPubKey | null = null;
            if (s.winner) {
                if (s.winner instanceof Uint8Array) {
                    winnerPubKey = s.winner;
                } else if (typeof s.winner === 'string') {
                    console.warn("GameOver signal winner is a string, attempting to use as is:", s.winner);
                    try { winnerPubKey = decode(Uint8Array.from(atob(s.winner), c => c.charCodeAt(0))) as AgentPubKey; } catch { /*ignore, pass as is*/ winnerPubKey = s.winner as any; }
                }
            }
            handleRemoteGameOver(winnerPubKey);
            break;
        }
      } catch(e) { console.error("signal parse err", e); }
    });
  }

  // Updates ball physics, checks for collisions and scoring (only Player 1 executes this)
  function updateBallAndScore() {
    if (gameOver || !isPlayer1 || !liveGame) return;

    ball.x += ball.dx;
    ball.y += ball.dy;

    if (ball.y + BALL_RADIUS > CANVAS_HEIGHT || ball.y - BALL_RADIUS < 0) {
      ball.dy = -ball.dy;
      ball.y = Math.max(BALL_RADIUS, Math.min(CANVAS_HEIGHT - BALL_RADIUS, ball.y));
    }

    let hitPaddle = false;
    if (ball.dx < 0 && ball.x - BALL_RADIUS < PADDLE_WIDTH && ball.x > BALL_RADIUS && ball.y > paddle1Y && ball.y < paddle1Y + PADDLE_HEIGHT) {
        ball.dx = -ball.dx * 1.05;
        ball.x = PADDLE_WIDTH + BALL_RADIUS;
        ball.dy = (ball.y - (paddle1Y + PADDLE_HEIGHT / 2)) * 0.35;
        hitPaddle = true;
    }
    else if (ball.dx > 0 && ball.x + BALL_RADIUS > CANVAS_WIDTH - PADDLE_WIDTH && ball.x < CANVAS_WIDTH - BALL_RADIUS && ball.y > paddle2Y && ball.y < paddle2Y + PADDLE_HEIGHT) {
        ball.dx = -ball.dx * 1.05;
        ball.x = CANVAS_WIDTH - PADDLE_WIDTH - BALL_RADIUS;
        ball.dy = (ball.y - (paddle2Y + PADDLE_HEIGHT / 2)) * 0.35;
        hitPaddle = true;
    }

    let scored = false;
    if (ball.x + BALL_RADIUS < 0) {
      score.player2++; scored = true; sendScoreUpdate();
    } else if (ball.x - BALL_RADIUS > CANVAS_WIDTH) {
      score.player1++; scored = true; sendScoreUpdate();
    }

    if (scored) {
      console.log(`Score: ${score.player1} - ${score.player2}`);
      if (score.player1 >= WINNING_SCORE || score.player2 >= WINNING_SCORE) {
        winner = score.player1 >= WINNING_SCORE ? liveGame.player_1 : liveGame.player_2;
        gameOver = true;
        if(winner) console.log("Game Over! Winner:", truncatePubkey(winner));
        handleLocalGameOver();
      } else {
        ball.x = CANVAS_WIDTH / 2;
        ball.y = CANVAS_HEIGHT / 2;
        ball.dx = 5 * (score.player1 > score.player2 ? -1 : 1);
        ball.dy = 5 * (Math.random() > 0.5 ? 1 : -1);
        lastBallUpdate = 0;
        sendBallUpdate();
      }
    } else if (hitPaddle) {
      lastBallUpdate = 0;
      sendBallUpdate();
    } else {
      sendBallUpdate();
    }
  }

  // Handles actions needed when the game ends locally (P1 detects win condition)
  async function handleLocalGameOver() {
      if (!liveGame || !gameRecord || !gameRecord.signed_action) {
          console.error("Cannot handle game over: Missing liveGame, gameRecord, or signed_action");
          errorMsg = "Error handling game over: Missing essential game data.";
          return;
      }
      console.log("Handling local game over...");

      let latestGameState: Game | undefined;
      const recordEntry = gameRecord.entry;
      if (recordEntry && typeof recordEntry === 'object' && 'Present' in recordEntry && (recordEntry as any).Present) {
          const presentEntry = (recordEntry as { Present: Entry }).Present;
          if (presentEntry && presentEntry.entry_type === 'App' && presentEntry.entry instanceof Uint8Array) {
              try { latestGameState = decode(presentEntry.entry) as Game; } catch (e) { console.error("Decoding error in handleLocalGameOver:", e); }
          }
      }
      if (!latestGameState) {
          errorMsg = "Could not extract or decode latest game state in handleLocalGameOver."; console.error(errorMsg, gameRecord.entry); return;
      }

      const original_game_hash = gameId;
      const previous_game_hash = gameRecord.signed_action.hashed.hash;

      try {
            const finishedGameState: Game = {
                player_1: latestGameState.player_1,
                player_2: latestGameState.player_2,
                created_at: latestGameState.created_at,
                game_status: 'Finished',
                player_1_paddle: Math.round(paddle1Y),
                player_2_paddle: Math.round(paddle2Y),
                ball_x: Math.round(ball.x),
                ball_y: Math.round(ball.y),
            };
            const updatePayload: UpdateGameInput = {
                 original_game_hash: original_game_hash,
                 previous_game_hash: previous_game_hash,
                 updated_game: finishedGameState,
            };
            console.log("Updating game status to Finished with payload:", updatePayload);
            await client.callZome({ cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME, fn_name: "update_game", payload: updatePayload });
            console.log("Game status updated on DHT.");
       } catch (e) {
            console.error("Error updating game status:", e);
            errorMsg = `Failed to update game status: ${(e as Error).message}`;
            return;
       }

       let score1Result: CreateScoreOutput | undefined;
       try {
           if (!liveGame || !liveGame.player_1) { throw new Error("liveGame or player_1 missing"); }
           type CreateScorePayload = { game_id: ActionHash; player: AgentPubKey; player_points: number; };

           const score1Payload: CreateScorePayload = {
               game_id: original_game_hash,
               player: liveGame.player_1,
               player_points: score.player1,
           };
           score1Result = await client.callZome({
             cap_secret: null,
             role_name: HOLOCHAIN_ROLE_NAME,
             zome_name: HOLOCHAIN_ZOME_NAME,
             fn_name: "create_score",
             payload: score1Payload
           });
           if (score1Result) {
             collectedTimeToWriteScoreMs = score1Result.write_duration_ms;
             console.log(`Player 1 score created. Write duration: ${collectedTimeToWriteScoreMs}ms. Hash: ${encodeHashToBase64(score1Result.score_hash)}`);
           }

           if (liveGame.player_2) {
                const score2Payload: CreateScorePayload = {
                   game_id: original_game_hash,
                   player: liveGame.player_2,
                   player_points: score.player2,
                };
                const score2Result: CreateScoreOutput = await client.callZome({
                  cap_secret: null,
                  role_name: HOLOCHAIN_ROLE_NAME,
                  zome_name: HOLOCHAIN_ZOME_NAME,
                  fn_name: "create_score",
                  payload: score2Payload
                });
                console.log(`Player 2 score created. Write duration: ${score2Result.write_duration_ms}ms. Hash: ${encodeHashToBase64(score2Result.score_hash)}`);
                // If averaging: collectedTimeToWriteScoreMs = Math.round((collectedTimeToWriteScoreMs + score2Result.write_duration_ms) / 2);
           }
           console.log("Scores saved.");
       } catch (e) {
           console.error("Error saving scores:", e);
           errorMsg = "Failed to save scores.";
           return;
       }

       if (score1Result) {
           await measureReadTime(score1Result);
       } else {
           console.warn("Skipping read time measurement as P1 score creation failed or didn't return result.");
       }

       try {
           const gameOverPayload: GameOverPayload = {
                game_id: original_game_hash,
                winner: winner,
                score1: score.player1,
                score2: score.player2,
                sent_at: [Math.floor(Date.now() / 1000), (Date.now() % 1000) * 1_000_000] as Timestamp
           };
           await client.callZome({
               cap_secret: null, role_name: HOLOCHAIN_ROLE_NAME, zome_name: HOLOCHAIN_ZOME_NAME,
               fn_name: "send_game_over",
               payload: gameOverPayload
            });
           console.log("GameOver signal sent.");
       } catch(e) { console.error("Error sending GameOver signal:", e); }

       await saveGameStatistics();
  }

  async function measureReadTime(createdScoreOutput: CreateScoreOutput) {
    if (!client || !createdScoreOutput || !createdScoreOutput.score_hash) {
        console.warn("Cannot measure read time: Client or score_hash missing.", createdScoreOutput);
        return;
    }
    console.log("Attempting to measure read time for score hash:", encodeHashToBase64(createdScoreOutput.score_hash));
    try {
        const getScoreResult: GetScoreOutput = await client.callZome({
            cap_secret: null,
            role_name: HOLOCHAIN_ROLE_NAME,
            zome_name: HOLOCHAIN_ZOME_NAME,
            fn_name: "get_score_and_measure_time",
            payload: createdScoreOutput.score_hash,
        });
        collectedTimeToReadScoreMs = getScoreResult.read_duration_ms;
        console.log("Time to read score:", collectedTimeToReadScoreMs, "ms. Score Record found:", !!getScoreResult.score_record);
    } catch (e) {
        console.error("Error getting score for read time measurement:", e);
    }
  }

  async function saveGameStatistics() {
    if (!client || !liveGame || !gameId || !liveGame.player_2) { // Ensure player_2 exists for typical game stats
        console.warn("Cannot save game statistics: Missing client, liveGame, gameId, or player_2.", { client, liveGame, gameId });
        return;
    }
    console.log("Attempting to save game statistics...");

    const averageLatencyMs = RTTs.length > 0 ? RTTs.reduce((a, b) => a + b, 0) / RTTs.length : 0;

    const gameStatsPayload: GameStats = {
        game_id: gameId, // original_game_hash
        player_1: liveGame.player_1,
        player_2: liveGame.player_2, // Assuming player_2 is non-null for a completed game that needs stats
        latency_ms: Math.round(averageLatencyMs),
        time_to_write_score_ms: Math.round(collectedTimeToWriteScoreMs),
        time_to_read_score_ms: Math.round(collectedTimeToReadScoreMs),
        created_at: [Math.floor(Date.now() / 1000), (Date.now() % 1000) * 1_000_000] as Timestamp
    };

    try {
        await client.callZome({
            cap_secret: null,
            role_name: HOLOCHAIN_ROLE_NAME,
            zome_name: HOLOCHAIN_ZOME_NAME,
            fn_name: "create_game_stats",
            payload: gameStatsPayload,
        });
        console.log("Game statistics saved successfully.", gameStatsPayload);
    } catch (e) {
        console.error("Error saving game statistics:", e);
    }
  }

  function handleRemoteGameOver(remoteWinner: AgentPubKey | null) {
      if (gameOver) return;
      console.log("Handling remote game over signal...");
      gameOver = true;
      winner = remoteWinner;
  }

  async function requestExit() {
      console.log("PongGame: Requesting to abandon game and dispatching exit-game event");
      if (!client || !gameId) {
          console.error("PongGame: Client or gameId not available to abandon game.");
          dispatch("exit-game");
          return;
      }
      try {
          console.log(`PongGame: Calling abandon_game for gameId: ${encodeHashToBase64(gameId)}`);
          await client.callZome({
              cap_secret: null,
              role_name: HOLOCHAIN_ROLE_NAME,
              zome_name: HOLOCHAIN_ZOME_NAME,
              fn_name: "abandon_game",
              payload: gameId,
          });
          console.log("PongGame: abandon_game zome call successful.");
      } catch (e) {
          console.error("PongGame: Error calling abandon_game zome function:", e);
      }
      dispatch("exit-game");
  }

  function viewStats() {
    if (!gameId) {
        console.error("Cannot view stats: gameId is not available.");
        return;
    }
    console.log("Requesting to view stats for game:", encodeHashToBase64(gameId));
    dispatch('view-stats', { gameId });
  }

  function draw() {
    if (!ctx) {
        if (!errorMsg) animationFrameId = requestAnimationFrame(draw);
        return;
    }

    ctx.fillStyle = "#FFA500"; ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
    ctx.strokeStyle = "#000000"; ctx.lineWidth = 4; ctx.beginPath();
    ctx.setLineDash([10, 10]); ctx.moveTo(CANVAS_WIDTH / 2, 0); ctx.lineTo(CANVAS_WIDTH / 2, CANVAS_HEIGHT);
    ctx.stroke(); ctx.setLineDash([]);

    if (!liveGame && !gameOver) {
        ctx.fillStyle = "#000000"; ctx.font = "30px 'Press Start 2P', monospace"; ctx.textAlign = "center";
        ctx.fillText(errorMsg || loadingMsg || "Loading...", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
        if (!errorMsg && loadingMsg) animationFrameId = requestAnimationFrame(draw);
        return;
    }

    if (liveGame) {
        ctx.fillStyle = "#000000";
        ctx.fillRect(0, paddle1Y, PADDLE_WIDTH, PADDLE_HEIGHT);
        ctx.fillRect(CANVAS_WIDTH - PADDLE_WIDTH, paddle2Y, PADDLE_WIDTH, PADDLE_HEIGHT);
        ctx.beginPath(); ctx.arc(ball.x, ball.y, BALL_RADIUS, 0, 2 * Math.PI); ctx.fill();

        ctx.font = "40px 'Press Start 2P', monospace"; ctx.textAlign = "center";
        ctx.fillText(score.player1.toString(), CANVAS_WIDTH / 4, 60);
        ctx.fillText(score.player2.toString(), (3 * CANVAS_WIDTH) / 4, 60);
    }

    if (gameOver) {
        ctx.fillStyle = "rgba(0, 0, 0, 0.7)"; ctx.fillRect(0, 0, CANVAS_WIDTH, CANVAS_HEIGHT);
        ctx.fillStyle = "#000000"; ctx.font = "50px 'Press Start 2P', monospace"; ctx.textAlign = "center";
        ctx.fillText("GAME OVER", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 - 50);
        ctx.font = "30px 'Press Start 2P', monospace";
         if (winner && liveGame) {
             const winnerName = encodeHashToBase64(winner) === encodeHashToBase64(liveGame.player_1) ? (player1Profile?.nickname || "Player 1") : (player2Profile?.nickname || "Player 2");
             ctx.fillText(`${winnerName} Wins!`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2);
         } else { ctx.fillText("Game Finished", CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2); }
         ctx.font = "40px 'Press Start 2P', monospace";
         ctx.fillText(`${score.player1} - ${score.player2}`, CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2 + 50);
        return;
    }

    if (liveGame && liveGame.game_status === 'InProgress') {
        if (isPlayer1) updateBallAndScore();
        animationFrameId = requestAnimationFrame(draw);
    } else if (liveGame && liveGame.game_status === 'Waiting') {
        ctx.fillStyle = "#888"; ctx.font = "24px Arial"; ctx.textAlign = "center";
        ctx.fillText("Waiting for game to start...", CANVAS_WIDTH / 2, CANVAS_HEIGHT - 50);
        animationFrameId = requestAnimationFrame(draw);
    }
  }

  onMount(async () => {
    client = await appClientContext.getClient();
    if (canvas) {
        ctx = canvas.getContext("2d")!;
    } else {
        console.error("Canvas element not found on mount.");
        errorMsg = "Failed to initialize canvas.";
        if(ctx) draw();
        return;
    }
    initializeGame();
  });

  onDestroy(() => {
    console.log("PongGame component destroyed. Cleaning up...");
    if (retryTimeoutId) clearTimeout(retryTimeoutId);
    cancelAnimationFrame(animationFrameId);
    window.removeEventListener("keydown", handleKeyDown);
    if (unsubscribeFromSignals) unsubscribeFromSignals();
  });

</script>

<div class="game-container">
    {#if errorMsg && !ctx} <p class="error-message">Error: {errorMsg}</p> {/if}

    <div class="game-window">
        <div class="players-info">
            <div class="player player1">P1: {#if player1Profile?.nickname}{player1Profile.nickname}{:else if liveGame?.player_1}{truncatePubkey(liveGame.player_1)}{:else}Loading...{/if}</div>
            <div class="player player2">P2: {#if player2Profile?.nickname}{player2Profile.nickname}{:else if liveGame?.player_2}{truncatePubkey(liveGame.player_2)}{:else}Waiting...{/if}</div>
        </div>

        <canvas bind:this={canvas} width={CANVAS_WIDTH} height={CANVAS_HEIGHT}></canvas>

        {#if gameOver}
            <div class="game-over-menu">
                <button on:click={requestExit}>Back to Lobby</button>
                <button on:click={viewStats}>View Game Stats</button> <!-- Added button -->
            </div>
        {:else if liveGame || errorMsg}
            <div class="exit-game-button">
                 <button on:click={requestExit}>Exit Game</button>
            </div>
        {/if}
    </div>
</div>

<style>
  .game-container { display: flex; justify-content: center; align-items: center; flex-direction: column; padding-top: 20px; }
  .error-message { color: red; margin-bottom: 10px; font-weight: bold; }
  .game-window { position: relative; /* For positioning buttons */ }
  .players-info {
    position: absolute;
    top: -25px; /* Position above the canvas */
    left: 0;
    width: 100%;
    display: flex;
    justify-content: space-between;
    padding: 0 15px; /* Padding on the sides */
    box-sizing: border-box; /* Include padding in width calculation */
    color: orange;
    font-size: 0.9rem;
    font-weight: bold;
    z-index: 1; /* Ensure it's above the canvas */
    pointer-events: none; /* Prevent interaction */
  }
  .player { background-color: rgba(0,0,0,0.6); padding: 3px 6px; border-radius: 4px; }
  canvas {
    background-color: orange;
    display: block; /* Remove extra space below canvas */
    margin: 0 auto; /* Center canvas */
    border: 3px solid black; /* Border around the game area */
    box-shadow: none;
  }
  .exit-game-button {
    position: absolute;
    top: 10px; /* Position near the top */
    right: 10px; /* Position on the right */
    z-index: 10;
  }
  .exit-game-button button {
    font-size: 0.9rem;
    padding: 0.4rem 0.8rem;
    background-color: orange; /* Reddish color */
    color:black;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }
   .exit-game-button button:hover { background-color: red; }

  .game-over-menu {
    position: absolute;
    bottom: 30px; /* Position towards the bottom */
    left: 50%;
    transform: translateX(-50%); /* Center horizontally */
    z-index: 10;
    display: flex; /* Added for button layout */
    gap: 10px; /* Added for spacing between buttons */
  }
  .game-over-menu button {
    font-size: 1.2rem;
    padding: 0.8rem 1.5rem;
    background-color: orange; /* Blue color */
    color: black;
    border: none;
    border-radius: 5px;
    cursor: pointer;
    transition: background-color 0.2s ease;
  }
  .game-over-menu button:hover { background-color: red; }
</style>
