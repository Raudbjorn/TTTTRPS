# Chat Persistence & Campaign Integration - Design Document

## Overview

This document specifies the technical design for:
1. Fixing chat persistence (race condition causing silent failures)
2. Integrating conversations with campaigns and game sessions
3. Supporting purpose-driven conversation threads

The codebase already has the foundational infrastructure:
- `global_chat_sessions` with `linked_campaign_id` and `linked_game_session_id`
- `conversation_threads` with `campaign_id`, `wizard_id`, and `ConversationPurpose`
- `ConversationManager` for thread and message operations

## Architecture

### Current Flow (Broken)

```
┌─────────────────────────────────────────────────────────────────┐
│ Component Mount                                                  │
├─────────────────────────────────────────────────────────────────┤
│ 1. spawn_local(get_or_create_chat_session())  ───► ASYNC       │
│ 2. User can interact immediately              ───► RACE!        │
│ 3. If send before session ready: persistence skipped            │
└─────────────────────────────────────────────────────────────────┘
```

### Proposed Flow (Fixed)

```
┌─────────────────────────────────────────────────────────────────┐
│ Component Mount                                                  │
├─────────────────────────────────────────────────────────────────┤
│ 1. is_loading_history = true (blocks input)                     │
│ 2. spawn_local/Effect::new to run async work                    │
│ 3. await get_or_create_chat_session()                           │
│ 4. await get_chat_messages()                                    │
│ 5. is_loading_history = false (enables input)                   │
│ 6. Send requires session_id.is_some() assertion                 │
└─────────────────────────────────────────────────────────────────┘
```

**Implementation Note (Leptos Async):**

Leptos component bodies are synchronous. Async work must use reactive primitives:

```rust
// Signals for async state
let (session_id, set_session_id) = create_signal(None::<String>);
let (is_loading_history, set_is_loading_history) = create_signal(true);

// Run async work in Effect or spawn_local
Effect::new(move |_| {
    spawn_local(async move {
        set_is_loading_history.set(true);

        match get_or_create_chat_session().await {
            Ok(session) => {
                set_session_id.set(Some(session.id.clone()));
                // Load messages
                if let Ok(msgs) = get_chat_messages(session.id).await {
                    // populate message list
                }
            }
            Err(e) => {
                show_error("Session Error", Some(&e), None);
            }
        }

        set_is_loading_history.set(false);
    });
});

// Send button disabled until session ready
let can_send = move || session_id.get().is_some() && !is_loading_history.get();
```

## Component Changes

### Chat Component (`frontend/src/components/chat/mod.rs`)

#### Change 1: Block Input Until Session Ready

The `is_loading_history` signal already exists but only affects display. Extend it to block the send button.

```rust
// Current (line 764): Input disabled only by is_loading (stream in progress)
disabled=is_loading

// Proposed: Input disabled while loading history OR streaming
disabled=Signal::derive(move || is_loading.get() || is_loading_history.get())
```

#### Change 2: Guard Message Send

Add an assertion that session ID is available before sending:

```rust
let send_message_streaming = move || {
    let msg = message_input.get();
    if msg.trim().is_empty() || is_loading.get() {
        return;
    }

    // NEW: Require session to be ready
    let session_id = match chat_session_id.get() {
        Some(id) => id,
        None => {
            show_error("Chat Error", Some("Session not ready. Please wait."), None);
            return;
        }
    };

    // ... rest of function uses session_id directly (not Option)
};
```

#### Change 3: Await Persistence Before Continuing

Currently, message persistence happens in fire-and-forget `spawn_local` calls. Change to await or use proper error handling:

```rust
// Current: Fire and forget
spawn_local(async move {
    if let Err(e) = add_chat_message(sid, ...).await {
        log::error!("Failed to persist user message: {}", e);
    }
});

// Proposed: Timeout + Retry Queue (non-blocking with resilience)

/// Message pending persistence retry
#[derive(Clone)]
struct PendingMessage {
    session_id: String,
    role: String,
    content: String,
    attempts: u32,
    created_at: DateTime<Utc>,
}

/// Retry queue (persisted to IndexedDB for crash recovery)
static RETRY_QUEUE: Lazy<RwLock<VecDeque<PendingMessage>>> = Lazy::new(|| {
    RwLock::new(VecDeque::with_capacity(100))
});

const PERSIST_TIMEOUT_MS: u64 = 5000;
const MAX_RETRY_ATTEMPTS: u32 = 3;
const MAX_QUEUE_SIZE: usize = 100;

/// Persist with timeout, queue on failure
async fn persist_with_retry(session_id: String, role: String, content: String) {
    let timeout = gloo_timers::future::sleep(Duration::from_millis(PERSIST_TIMEOUT_MS));
    let persist = add_chat_message(session_id.clone(), role.clone(), content.clone());

    match futures::future::select(Box::pin(persist), Box::pin(timeout)).await {
        futures::future::Either::Left((Ok(record), _)) => {
            // Success
        }
        futures::future::Either::Left((Err(e), _)) | futures::future::Either::Right(_) => {
            // Timeout or error - queue for retry
            let mut queue = RETRY_QUEUE.write().unwrap();

            // Backpressure: reject if queue full
            if queue.len() >= MAX_QUEUE_SIZE {
                show_error("Save Queue Full", Some("Too many pending messages"), None);
                return;
            }

            queue.push_back(PendingMessage {
                session_id,
                role,
                content,
                attempts: 1,
                created_at: Utc::now(),
            });

            show_warning("Message queued for retry");
        }
    }
}

/// Background task to drain retry queue (spawn on app init)
fn start_retry_worker() {
    spawn_local(async move {
        loop {
            gloo_timers::future::sleep(Duration::from_secs(5)).await;

            let pending = {
                let mut queue = RETRY_QUEUE.write().unwrap();
                queue.pop_front()
            };

            if let Some(mut msg) = pending {
                match add_chat_message(msg.session_id.clone(), msg.role.clone(), msg.content.clone()).await {
                    Ok(_) => { /* success, don't re-queue */ }
                    Err(e) => {
                        msg.attempts += 1;
                        if msg.attempts < MAX_RETRY_ATTEMPTS {
                            // Exponential backoff by re-queuing at back
                            let mut queue = RETRY_QUEUE.write().unwrap();
                            queue.push_back(msg);
                        } else {
                            log::error!("Message persistence failed after {} attempts: {}", MAX_RETRY_ATTEMPTS, e);
                        }
                    }
                }
            }
        }
    });
}
```

**Recommendation:** Use timeout + retry queue for resilience without blocking UI.

#### Change 4: Visual Loading State

Add a loading indicator in the message area:

```rust
// In message area view
{move || {
    if is_loading_history.get() {
        Some(view! {
            <div class="flex items-center justify-center p-8 text-gray-400">
                <span class="animate-pulse">"Loading conversation..."</span>
            </div>
        })
    } else {
        None
    }
}}
```

### Database Operations (No Changes Required)

The SQLite operations in `src-tauri/src/database/chat.rs` are correct. They properly:
- Use prepared statements
- Handle transactions
- Return appropriate errors

### Tauri Commands (No Changes Required)

Commands in `src-tauri/src/commands/session/chat.rs` are correctly implemented.

## State Machine

```
┌──────────────┐     mount      ┌───────────────┐
│   Unmounted  │ ──────────────►│    Loading    │
└──────────────┘                └───────┬───────┘
       ▲                                │
       │                    ┌───────────┴───────────┐
       │                    │                       │
       │ unmount            │ success               │ get_or_create_chat_session() failed
       │                    ▼                       ▼
       │            ┌───────────────┐       ┌───────────────┐
       └────────────│     Ready     │◄──────│ Loading Failed│
                    └───────┬───────┘ retry └───────────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
            │ send message  │               │ persist message failed
            ▼               │               ▼
    ┌───────────────┐       │       ┌─────────────────────┐
    │   Streaming   │───────┘       │ Persistence Failed  │
    └───────────────┘               └─────────┬───────────┘
         response complete                    │
                                              │ retry/recover
                                              ▼
                                      (back to Ready)
```

### State Descriptions

| State | Entry Condition | Exit Transitions |
|-------|-----------------|------------------|
| Unmounted | Component not rendered | mount → Loading |
| Loading | Component mounted | success → Ready, failure → Loading Failed |
| Loading Failed | get_or_create_chat_session() error | retry → Loading, unmount → Unmounted |
| Ready | Session loaded, input enabled | send → Streaming, persist fail → Persistence Failed |
| Streaming | Message sent, awaiting response | complete → Ready |
| Persistence Failed | add_chat_message() error | retry → Ready, dismiss → Ready |

## Error Handling

### Persistence Failure

```rust
enum PersistenceError {
    SessionNotReady,
    DatabaseError(String),
    NetworkTimeout,
}

// On error:
// 1. Log with full context
// 2. Show toast notification
// 3. Keep message in UI (marked as "not saved")
// 4. Retry on next send or component remount
```

### Session Load Failure

```rust
// If get_or_create_chat_session() fails:
// 1. Show error message in chat area
// 2. Provide "Retry" button
// 3. Disable input until session is available
```

## Error Recovery Strategies

### Strategy 1: Retry with Exponential Backoff

For transient failures (network timeout, DB lock), implement automatic retry:

```rust
const MAX_RETRIES: u32 = 3;
const BASE_DELAY_MS: u64 = 1000;

async fn persist_with_retry<F, T, E>(
    operation: F,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut attempts = 0;
    let mut last_error = None;

    while attempts < MAX_RETRIES {
        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                last_error = Some(e);

                if attempts < MAX_RETRIES {
                    let delay = BASE_DELAY_MS * 2u64.pow(attempts - 1);
                    log::warn!(
                        "{} failed (attempt {}), retrying in {}ms",
                        operation_name, attempts, delay
                    );
                    gloo_timers::future::sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}
```

### Strategy 2: Fallback Persistence Queue

When immediate persistence fails, queue for background retry:

```rust
/// Pending message awaiting persistence
#[derive(Clone, Serialize, Deserialize)]
struct PendingMessage {
    session_id: String,
    role: String,
    content: String,
    created_at: DateTime<Utc>,
    attempts: u32,
    last_attempt: Option<DateTime<Utc>>,
}

/// Persistence queue stored in IndexedDB for crash recovery
struct PersistenceQueue {
    queue: VecDeque<PendingMessage>,
    max_size: usize,
}

impl PersistenceQueue {
    fn new(max_size: usize) -> Self {
        Self { queue: VecDeque::new(), max_size }
    }

    fn enqueue(&mut self, msg: PendingMessage) -> Result<(), QueueFullError> {
        if self.queue.len() >= self.max_size {
            return Err(QueueFullError);
        }
        self.queue.push_back(msg);
        self.save_to_indexeddb();
        Ok(())
    }

    fn dequeue(&mut self) -> Option<PendingMessage> {
        let msg = self.queue.pop_front();
        if msg.is_some() {
            self.save_to_indexeddb();
        }
        msg
    }

    fn requeue_failed(&mut self, mut msg: PendingMessage) {
        msg.attempts += 1;
        msg.last_attempt = Some(Utc::now());
        // Add to back with incremented attempt count
        self.queue.push_back(msg);
        self.save_to_indexeddb();
    }
}
```

### Strategy 3: Streaming Placeholder Fallback

When assistant placeholder creation fails, create it at finalization time:

```rust
/// Handle streaming message finalization
async fn finalize_streaming_message(
    stream_id: &str,
    final_content: &str,
    tokens: Option<(i32, i32)>,
    streaming_persistent_id: &RwSignal<Option<String>>,
    session_id: &str,
) {
    // Check if we have a persistent ID from placeholder creation
    if let Some(pid) = streaming_persistent_id.get_untracked() {
        // Normal path: update existing placeholder
        match update_chat_message(pid.clone(), final_content.to_string(), tokens, false).await {
            Ok(_) => {
                log::debug!("Successfully updated placeholder {}", pid);
            }
            Err(e) => {
                log::error!("Failed to update placeholder: {}", e);
                // Fallback: try to create a new message
                fallback_create_message(session_id, final_content, tokens).await;
            }
        }
    } else {
        // Placeholder creation failed earlier - create message now
        log::warn!("No placeholder ID, creating message at finalization");
        fallback_create_message(session_id, final_content, tokens).await;
    }
}

async fn fallback_create_message(
    session_id: &str,
    content: &str,
    tokens: Option<(i32, i32)>,
) {
    match add_chat_message(
        session_id.to_string(),
        "assistant".to_string(),
        content.to_string(),
        tokens,
    ).await {
        Ok(record) => {
            log::info!("Fallback message created: {}", record.id);
        }
        Err(e) => {
            log::error!("Fallback persistence also failed: {}", e);
            show_error(
                "Message Lost",
                Some("Unable to save the assistant's response. Please try again."),
                None,
            );
        }
    }
}
```

### Strategy 4: Graceful Degradation

When persistence is completely unavailable, degrade gracefully:

```rust
/// Persistence availability state
#[derive(Clone, Copy, PartialEq)]
enum PersistenceState {
    Available,
    Degraded,      // Retrying, queue growing
    Unavailable,   // All retries exhausted
}

/// UI adaptation based on persistence state
fn adapt_ui_to_persistence_state(state: PersistenceState) -> impl IntoView {
    match state {
        PersistenceState::Available => {
            view! { /* Normal UI */ }.into_any()
        }
        PersistenceState::Degraded => {
            view! {
                <div class="bg-yellow-100 text-yellow-800 p-2 text-sm">
                    "⚠️ Some messages pending save. Don't close the app."
                </div>
            }.into_any()
        }
        PersistenceState::Unavailable => {
            view! {
                <div class="bg-red-100 text-red-800 p-2 text-sm">
                    "❌ Message saving unavailable. Chat will work but history may be lost."
                    <button class="underline ml-2" on:click=retry_persistence>
                        "Retry"
                    </button>
                </div>
            }.into_any()
        }
    }
}
```

### Strategy 5: Session Recovery on Component Remount

When Chat component remounts, check for orphaned messages:

```rust
/// Check for messages that were being streamed when component unmounted
async fn recover_orphaned_messages(session_id: &str) {
    let messages = get_chat_messages(session_id.to_string(), Some(10)).await.ok();

    if let Some(msgs) = messages {
        // Find any messages still marked as streaming
        for msg in msgs.iter().filter(|m| m.is_streaming) {
            log::warn!("Found orphaned streaming message: {}", msg.id);

            // Mark as incomplete rather than streaming
            if let Err(e) = update_chat_message(
                msg.id.clone(),
                format!("{}\n\n[Stream interrupted]", msg.content),
                None,
                false, // is_streaming = false
            ).await {
                log::error!("Failed to recover orphaned message: {}", e);
            }
        }
    }
}
```

### Error Recovery Decision Tree

```
Message send initiated
        │
        ▼
┌───────────────────────┐
│ Session ID available? │
└───────┬───────────────┘
        │
    ┌───┴───┐
    │ No    │ Yes
    ▼       ▼
┌───────┐ ┌───────────────────────┐
│ Block │ │ Persist user message  │
│ send  │ └───────────┬───────────┘
└───────┘             │
                  ┌───┴───┐
                  │ Fail? │
                  └───┬───┘
              ┌───────┴───────┐
              │ Yes           │ No
              ▼               ▼
       ┌──────────────┐  ┌──────────────┐
       │ Retry (3x)   │  │ Create       │
       │ w/ backoff   │  │ placeholder  │
       └──────┬───────┘  └──────┬───────┘
              │                 │
          ┌───┴───┐         ┌───┴───┐
          │ Fail? │         │ Fail? │
          └───┬───┘         └───┬───┘
      ┌───────┴───────┐     ┌───┴───┐
      │ Yes           │ No  │ Yes   │ No
      ▼               ▼     ▼       ▼
┌───────────┐  ┌─────────┐ ┌─────────────────┐ ┌────────────┐
│ Queue for │  │Continue │ │Set fallback flag│ │ Stream     │
│ bg retry  │  │         │ │(create at final)│ │ response   │
└───────────┘  └─────────┘ └─────────────────┘ └────────────┘
      │                           │                   │
      ▼                           │                   ▼
┌───────────┐                     │          ┌────────────────┐
│ Show toast│                     │          │ Finalize:      │
│ "pending" │                     │          │ update or      │
└───────────┘                     └──────────│ fallback create│
                                             └────────────────┘
```

## Testing Strategy

### Unit Tests

1. **Session Guard Test**: Verify send is blocked when `chat_session_id` is None
2. **Persistence Test**: Mock `add_chat_message` and verify it's called with correct args
3. **Retry Test**: Simulate transient failure, verify retry logic

### Integration Tests

1. **Navigation Test**:
   - Mount Chat, send message, verify DB has message
   - Navigate away, navigate back, verify message displayed

2. **Race Condition Test**:
   - Mount Chat with delayed session load
   - Immediately attempt send
   - Verify send is blocked or queued

3. **Restart Test**:
   - Send messages, close app
   - Reopen app, verify messages load

### Manual Testing

1. Open DevTools console
2. Send a message
3. Check `~/.local/share/com.ttrpg.assistant/ttrpg_assistant.db`:
   ```sql
   SELECT * FROM chat_messages ORDER BY created_at DESC LIMIT 5;
   ```

## Migration Notes

No database migrations required. Existing schema is sufficient.

## Files to Modify

| File | Change |
|------|--------|
| `frontend/src/components/chat/mod.rs` | Block input during load, guard send, await persistence |
| `frontend/src/components/design_system/input.rs` | (optional) Add "saving" visual indicator |

## Rollout Plan

1. **Phase 1**: Fix race condition (block input during load)
2. **Phase 2**: Add error visibility (toast on persistence failure)
3. **Phase 3**: Add retry logic (resilience)
4. **Phase 4**: Add loading/saving indicators (UX polish)

## Alternatives Considered

### A: Global State Store

**Approach:** Use a global store (like `provide_context`) for chat state that persists across component mounts.

**Rejected because:**
- Doesn't solve app restart persistence (still need DB)
- Adds complexity without solving root cause
- Current SQLite approach is correct, just needs race condition fix

### B: Meilisearch Conversation Storage

**Approach:** Store conversations in Meilisearch for searchability.

**Rejected because:**
- Meilisearch Chat is explicitly stateless
- Would require custom index for conversations
- SQLite is simpler and already works
- Could be added later as enhancement for search

### C: LLM Provider Session IDs

**Approach:** Some providers (future) may support session-based conversations.

**Not applicable:**
- Claude, Gemini, OpenAI are all stateless
- Would require per-provider implementation
- Not a universal solution

## Decision Log

| Decision | Rationale |
|----------|-----------|
| Block input during session load | Simplest fix for race condition |
| Keep SQLite as primary store | Already implemented, proven technology |
| Await persistence before UI update | Ensures data integrity, acceptable latency |
| Use toast for errors | Non-blocking, visible feedback |

---

# Part 2: Campaign & Session Integration

## Existing Infrastructure

### Database Tables (Already Exist)

```
┌─────────────────────┐     ┌─────────────────────┐
│ global_chat_sessions│     │ conversation_threads│
├─────────────────────┤     ├─────────────────────┤
│ id                  │     │ id                  │
│ status              │     │ campaign_id (FK)    │
│ linked_campaign_id ─┼──┐  │ wizard_id (FK)      │
│ linked_game_session_│  │  │ purpose             │
│ created_at          │  │  │ title               │
│ updated_at          │  │  │ active_personality  │
└─────────────────────┘  │  │ branched_from       │
         │               │  └─────────────────────┘
         │               │           │
         ▼               │           ▼
┌─────────────────────┐  │  ┌─────────────────────┐
│ chat_messages       │  │  │ conversation_messages│
├─────────────────────┤  │  ├─────────────────────┤
│ id                  │  │  │ id                  │
│ session_id (FK)     │  │  │ thread_id (FK)      │
│ role                │  │  │ role                │
│ content             │  │  │ content             │
│ tokens_*            │  │  │ suggestions (JSON)  │
│ is_streaming        │  │  │ citations (JSON)    │
└─────────────────────┘  │  └─────────────────────┘
                         │
         ┌───────────────┘
         ▼
┌─────────────────────┐     ┌─────────────────────┐
│ campaigns           │────►│ sessions            │
├─────────────────────┤     ├─────────────────────┤
│ id                  │     │ id                  │
│ name                │     │ campaign_id (FK)    │
│ system              │     │ session_number      │
│ world_state (JSON)  │     │ status              │
│ house_rules (JSON)  │     │ notes               │
└─────────────────────┘     └─────────────────────┘
```

### ConversationPurpose Enum

```rust
pub enum ConversationPurpose {
    CampaignCreation,   // Campaign wizard flow
    SessionPlanning,    // Planning a specific session
    NpcGeneration,      // Creating NPCs
    WorldBuilding,      // Building campaign world
    CharacterBackground,// Character backstories
    General,            // General DM assistance
}
```

## Integration Architecture

### Two Chat Systems

| System | Table | Use Case | Context |
|--------|-------|----------|---------|
| Global Chat | `global_chat_sessions` | Main DM chat, always available | Optional campaign/session link |
| Conversation Threads | `conversation_threads` | Purpose-driven, structured | Campaign/wizard link, suggestions |

**Design Decision:** Keep both systems. Global chat is the quick-access assistant. Conversation threads are for structured, purpose-driven interactions (campaign creation, session planning).

### Context Flow

```
┌──────────────────────────────────────────────────────────────────┐
│                        User Navigation                            │
├──────────────────────────────────────────────────────────────────┤
│                                                                    │
│  /chat (global)          /session/:campaign_id                    │
│       │                           │                               │
│       ▼                           ▼                               │
│  ┌─────────────┐           ┌─────────────┐                       │
│  │ Global Chat │           │ Session Chat│                       │
│  │ (unlinked)  │           │ (linked)    │                       │
│  └─────────────┘           └─────────────┘                       │
│                                   │                               │
│                    ┌──────────────┼──────────────┐               │
│                    ▼              ▼              ▼               │
│            ┌───────────┐  ┌───────────┐  ┌───────────┐          │
│            │ Campaign  │  │ Session   │  │   NPC     │          │
│            │ Context   │  │ Notes     │  │ Summaries │          │
│            └───────────┘  └───────────┘  └───────────┘          │
│                    │              │              │               │
│                    └──────────────┴──────────────┘               │
│                                   │                               │
│                                   ▼                               │
│                    ┌────────────────────────┐                    │
│                    │    AI System Prompt    │                    │
│                    │  (context-augmented)   │                    │
│                    └────────────────────────┘                    │
│                                                                    │
└──────────────────────────────────────────────────────────────────┘
```

## Component Changes

### 1. Context Provider (`frontend/src/services/chat_context.rs` - NEW)

```rust
/// Campaign context for chat augmentation
#[derive(Clone, Default)]
pub struct ChatContext {
    pub campaign: Option<CampaignRecord>,
    pub session: Option<SessionRecord>,
    pub npcs: Vec<NpcSummary>,
    pub locations: Vec<LocationSummary>,
    /// True while loading campaign data
    pub loading: bool,
    /// Error message if loading failed
    pub error: Option<String>,
}

/// Provide chat context at app level
pub fn provide_chat_context() {
    let context = RwSignal::new(ChatContext::default());
    provide_context(context);
}

/// Update context when navigating to session workspace.
/// Returns Err if any fetch fails.
pub async fn set_campaign_context(campaign_id: &str) -> Result<(), String> {
    let ctx = use_context::<RwSignal<ChatContext>>()
        .ok_or_else(|| "ChatContext not provided".to_string())?;

    // Set loading state
    ctx.update(|c| {
        c.loading = true;
        c.error = None;
    });

    // Fetch campaign data
    let campaign = get_campaign_by_id(campaign_id).await.map_err(|e| {
        ctx.update(|c| {
            c.loading = false;
            c.error = Some(format!("Failed to load campaign: {}", e));
        });
        e
    })?;

    // Fetch active session
    let session = get_active_session(campaign_id).await.ok(); // Optional

    // Fetch NPCs and locations
    let npcs = load_campaign_npcs(campaign_id).await.unwrap_or_default();
    let locations = load_campaign_locations(campaign_id).await.unwrap_or_default();

    // Update context atomically
    ctx.update(|c| {
        c.campaign = Some(campaign);
        c.session = session;
        c.npcs = npcs;
        c.locations = locations;
        c.loading = false;
        c.error = None;
    });

    Ok(())
}

/// Clear context when leaving session workspace
pub fn clear_campaign_context() {
    if let Some(ctx) = use_context::<RwSignal<ChatContext>>() {
        ctx.set(ChatContext::default());
    }
}
```

### 2. Chat Component Enhancement (`frontend/src/components/chat/mod.rs`)

```rust
// On mount, check for campaign context
let chat_context = use_context::<RwSignal<ChatContext>>();

// Build system prompt with context
let build_system_prompt = move || {
    let base_prompt = "You are a TTRPG assistant...";

    if let Some(ctx) = chat_context {
        let context = ctx.get();
        if let Some(campaign) = &context.campaign {
            format!(
                "{}\n\n## Campaign Context\n\
                 Campaign: {}\n\
                 System: {}\n\
                 Setting: {}\n\n\
                 ## NPCs\n{}\n\n\
                 ## Locations\n{}",
                base_prompt,
                campaign.name,
                campaign.system,
                campaign.setting.as_deref().unwrap_or("Not specified"),
                format_npcs(&context.npcs),
                format_locations(&context.locations),
            )
        } else {
            base_prompt.to_string()
        }
    } else {
        base_prompt.to_string()
    }
};
```

### 3. Session Workspace Integration (`frontend/src/components/session/mod.rs`)

```rust
// Cleanup signal to prevent stale updates after unmount
let cleanup_signal = RwSignal::new(false);

// In Session component mount
Effect::new(move |_| {
    let campaign_id = campaign_id.clone();
    let active_session_id = active_session_id.clone();

    spawn_local(async move {
        // Set campaign context for chat (with proper error handling)
        if let Err(e) = set_campaign_context(&campaign_id).await {
            log::error!("Failed to set campaign context: {}", e);
            show_error("Context Error", Some(&e), None);
            return;
        }

        // Check if component was unmounted during async work
        if cleanup_signal.get() {
            log::debug!("Component unmounted, aborting session link");
            return;
        }

        // Link active chat session to this campaign
        match get_or_create_chat_session().await {
            Ok(session) => {
                // Check cleanup signal again before making another async call
                if cleanup_signal.get() {
                    return;
                }

                if let Err(e) = link_chat_to_game_session(
                    session.id,
                    active_session_id,
                    Some(campaign_id.clone()),
                ).await {
                    log::error!("Failed to link chat session: {}", e);
                    // Non-fatal: chat still works, just not linked
                }
            }
            Err(e) => {
                log::error!("Failed to get/create chat session: {}", e);
                show_error("Session Error", Some(&e), None);
            }
        }
    });
});

// On unmount - set cleanup signal BEFORE clearing context
on_cleanup(move || {
    cleanup_signal.set(true);
    clear_campaign_context();
});
```

### 4. Conversation Thread UI (`frontend/src/components/session/chat_panel.rs` - NEW)

```rust
#[component]
pub fn SessionChatPanel(campaign_id: String) -> impl IntoView {
    // State for thread list and active thread
    let threads = RwSignal::new(Vec::<ConversationThread>::new());
    let active_thread = RwSignal::new(Option::<String>::None);

    // Load threads for this campaign on mount
    spawn_local(async move {
        let campaign_threads = list_conversation_threads(campaign_id, None).await?;
        threads.set(campaign_threads);
    });

    view! {
        <div class="flex flex-col h-full">
            // Thread selector tabs
            <ThreadTabs threads=threads active=active_thread />

            // Message display (conditional on active thread)
            {move || match active_thread.get() {
                Some(tid) => view! { <ThreadMessages thread_id=tid /> }.into_any(),
                None => view! { <GlobalChatEmbed campaign_id=campaign_id.clone() /> }.into_any(),
            }}

            // Input (routes to active thread or global chat)
            <ChatInput active_thread=active_thread />
        </div>
    }
}
```

### 5. Backend: Context-Augmented Streaming

The `stream_chat` command needs to include campaign context in the request:

```rust
// In src-tauri/src/commands/llm/streaming.rs

#[tauri::command]
pub async fn stream_chat(
    messages: Vec<StreamingChatMessage>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    system_prompt: Option<String>,  // NEW: Allow custom system prompt
    provided_stream_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<String, String> {
    // If system_prompt is provided, use it (includes campaign context)
    // Otherwise, use default DM system prompt
    let effective_system = system_prompt.unwrap_or_else(|| default_dm_prompt());

    // ... rest of streaming logic
}
```

## Data Flow: Session Planning

```
User clicks "Plan Session" in Session Workspace
         │
         ▼
┌──────────────────────────────────────────────────────────────────┐
│ 1. Create ConversationThread                                      │
│    - purpose: SessionPlanning                                     │
│    - campaign_id: current campaign                                │
│    - title: "Session N Planning"                                  │
└──────────────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────────────────────────────┐
│ 2. Load Session Planning System Prompt                           │
│    "You are helping plan an engaging TTRPG session..."           │
│    + Campaign context (NPCs, locations, world state)             │
│    + Previous session summary (if available)                     │
└──────────────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────────────────────────────┐
│ 3. User sends planning messages                                   │
│    - Saved to conversation_messages                               │
│    - AI responses include suggestions (parsed from JSON blocks)  │
└──────────────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────────────────────────────┐
│ 4. User can accept suggestions                                    │
│    - Create NPCs from suggestions                                 │
│    - Add plot points from suggestions                             │
│    - Track in acceptance_events table                             │
└──────────────────────────────────────────────────────────────────┘
```

## API Additions

### New Tauri Commands

```rust
// Get conversation threads for a campaign
#[tauri::command]
pub async fn list_campaign_conversations(
    campaign_id: String,
    purpose: Option<String>,  // Filter by purpose
    state: State<'_, AppState>,
) -> Result<Vec<ConversationThread>, String>;

// Get messages for a conversation thread
#[tauri::command]
pub async fn get_thread_messages(
    thread_id: String,
    limit: Option<i32>,
    state: State<'_, AppState>,
) -> Result<Vec<ConversationMessage>, String>;

// Create a new conversation thread
#[tauri::command]
pub async fn create_conversation_thread(
    campaign_id: Option<String>,
    purpose: String,
    title: Option<String>,
    state: State<'_, AppState>,
) -> Result<ConversationThread, String>;

// Add message to a thread (handles suggestions/citations parsing)
#[tauri::command]
pub async fn add_thread_message(
    thread_id: String,
    role: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<ConversationMessage, String>;

// Stream chat with context (augmented version)
#[tauri::command]
pub async fn stream_chat_with_context(
    messages: Vec<StreamingChatMessage>,
    campaign_id: Option<String>,
    session_id: Option<String>,
    purpose: Option<String>,  // Determines system prompt
    thread_id: Option<String>,  // If set, saves to thread
    state: State<'_, AppState>,
) -> Result<String, String>;
```

### Frontend Bindings

```rust
// In frontend/src/bindings/ai.rs

pub async fn list_campaign_conversations(
    campaign_id: String,
    purpose: Option<String>,
) -> Result<Vec<ConversationThread>, String>;

pub async fn create_conversation_thread(
    campaign_id: Option<String>,
    purpose: String,
    title: Option<String>,
) -> Result<ConversationThread, String>;

pub async fn stream_chat_with_context(
    messages: Vec<StreamingChatMessage>,
    campaign_id: Option<String>,
    session_id: Option<String>,
    purpose: Option<String>,
    thread_id: Option<String>,
) -> Result<String, String>;
```

## Migration

### Add session_id to conversation_threads (Optional Enhancement)

```sql
-- Migration V28: Add session_id to conversation_threads (idempotent)

-- Step 1: Add column if not exists (SQLite workaround: check pragma)
-- Note: SQLite doesn't support IF NOT EXISTS for columns, use migration framework guard
-- or check: SELECT COUNT(*) FROM pragma_table_info('conversation_threads') WHERE name='session_id';

-- If column doesn't exist:
ALTER TABLE conversation_threads ADD COLUMN session_id TEXT;

-- Step 2: Create index idempotently
CREATE INDEX IF NOT EXISTS idx_conversation_threads_session_id
    ON conversation_threads(session_id);

-- Step 3: Add foreign key constraint in separate migration AFTER sessions table exists
-- Migration V29 (runs after sessions table is created):
-- Note: SQLite doesn't support adding FK constraints after table creation.
-- Use application-level validation or recreate table with FK if needed.
```

**Idempotency Notes:**
- Check column existence before ALTER TABLE (migration framework or pragma check)
- Use `CREATE INDEX IF NOT EXISTS` for index creation
- Foreign key constraint requires sessions table to exist first
- Consider splitting into V28 (add column) and V29 (add FK via table recreation)

This allows threads to be linked to specific sessions, not just campaigns.

## Rollout Plan

### Phase 1: Fix Persistence (No Campaign Integration)
- Fix race condition
- Add error visibility
- Verify messages persist

### Phase 2: Campaign Context
- Add `ChatContext` provider
- Augment system prompt with campaign data
- Link chat session to campaign when in session workspace

### Phase 3: Conversation Threads UI
- Add session chat panel component
- Thread selector tabs
- Purpose-specific thread creation

### Phase 4: Session Planning Flow
- "Plan Session" button
- Session planning system prompt
- Suggestion parsing and acceptance

---

# Part 3: NPC Conversation Integration

## Existing NPC Conversation System

The codebase already has an NPC conversation system in `npc_conversations` table:

```rust
pub struct NpcConversation {
    pub id: String,
    pub npc_id: String,           // Links to npcs table
    pub campaign_id: String,      // Campaign context
    pub messages_json: String,    // Serialized Vec<ConversationMessage>
    pub unread_count: u32,
    pub last_message_at: String,
    pub created_at: String,
    pub updated_at: String,
}
```

### Integration Approach

Rather than creating a new system, we'll enhance the existing `npc_conversations` with:
1. Streaming support (currently synchronous)
2. Voice mode toggle (AI speaks as NPC)
3. UI integration in NPC detail views

## NPC Conversation Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      NPC Detail Page                             │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────────────────────────┐   │
│  │  NPC Info       │  │  Conversation Panel                  │   │
│  │  - Name         │  │  ┌─────────────────────────────────┐│   │
│  │  - Background   │  │  │ [Toggle: About NPC | As NPC]    ││   │
│  │  - Personality  │  │  ├─────────────────────────────────┤│   │
│  │  - Voice        │  │  │                                 ││   │
│  │                 │  │  │  Message History                ││   │
│  │  [Edit] [Chat]  │  │  │  - User: Tell me about...       ││   │
│  │                 │  │  │  - AI: [context-aware response] ││   │
│  └─────────────────┘  │  │                                 ││   │
│                       │  ├─────────────────────────────────┤│   │
│                       │  │  [Input field]  [Send]          ││   │
│                       │  └─────────────────────────────────┘│   │
│                       └─────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## NPC Conversation Modes

### Mode 1: "About NPC" (Default)
- User asks questions about the NPC
- AI responds as a DM assistant
- System prompt includes NPC data as context
- Useful for developing backstory, motivations

```
System: You are helping develop an NPC named {name}.
        Background: {background}
        Personality: {personality}
        Help the GM flesh out this character.

User: What secret might they be hiding?
AI: Given {name}'s background as a former soldier, they might...
```

### Mode 2: "As NPC" (Voice Mode)
- AI roleplays as the NPC
- First-person responses
- Uses NPC's speech patterns and personality
- Useful for dialogue practice, voice consistency

```
System: You ARE {name}, an NPC in a TTRPG campaign.
        Speak in first person with these traits: {personality}
        Voice style: {voice_description}
        Never break character.

User: What do you think of the adventurers?
AI (as NPC): *eyes the party suspiciously* Those mercenaries?
             They seem capable enough, but I don't trust anyone
             who works for coin alone...
```

### Message Schema with Mode Tracking

Each message must record which mode it was sent in:

```rust
/// NPC conversation message with mode tracking
#[derive(Clone, Serialize, Deserialize)]
pub struct NpcMessage {
    pub id: String,
    pub role: String,        // "user" or "assistant"
    pub content: String,
    pub mode: String,        // "about" or "as"
    pub timestamp: DateTime<Utc>,
}
```

**Context Filtering by Mode:**

When reconstructing conversation history for chat streaming, filter by current mode:

```rust
fn build_message_context(messages: &[NpcMessage], current_mode: &str) -> Vec<ChatMessage> {
    messages
        .iter()
        .filter(|msg| msg.mode == current_mode)  // Only same-mode messages
        .map(|msg| ChatMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        })
        .collect()
}
```

### Prompt Injection Protection

The "As NPC" mode is vulnerable to prompt injection via NPC fields. Implement guards:

```rust
/// Patterns that indicate prompt injection attempts
const INJECTION_PATTERNS: &[&str] = &[
    "ignore previous",
    "ignore all previous",
    "disregard previous",
    "forget previous",
    "new instructions",
    "system:",
    "assistant:",
    "human:",
    "```",           // Code blocks can hide instructions
    "---",           // Markdown separators
    "##",            // Markdown headers
];

/// Detect potential prompt injection in user input
fn detect_injection(input: &str) -> bool {
    let lower = input.to_lowercase();
    INJECTION_PATTERNS.iter().any(|p| lower.contains(p))
}

/// Guard mode switch - prevent rapid toggling
fn can_switch_mode(last_switch: DateTime<Utc>) -> bool {
    (Utc::now() - last_switch).num_seconds() > 5  // 5 second cooldown
}

// In send_message handler:
if mode == "as" && detect_injection(&user_input) {
    show_warning("Message blocked: potential instruction override detected");
    return;
}
```

## Component Design

### NpcConversationPanel Component

```rust
#[component]
pub fn NpcConversationPanel(
    npc_id: String,
    campaign_id: String,
) -> impl IntoView {
    // Conversation mode: "about" or "as"
    let mode = RwSignal::new("about".to_string());

    // Messages from npc_conversations
    let messages = RwSignal::new(Vec::<NpcMessage>::new());

    // Load existing conversation on mount
    spawn_local(async move {
        let conv = get_or_create_npc_conversation(npc_id, campaign_id).await?;
        messages.set(parse_messages_json(&conv.messages_json));
    });

    // Build system prompt based on mode
    let system_prompt = move || {
        let npc = npc_data.get();
        match mode.get().as_str() {
            "as" => build_voice_mode_prompt(&npc),
            _ => build_about_mode_prompt(&npc),
        }
    };

    view! {
        <div class="flex flex-col h-full">
            // Mode toggle
            <div class="flex gap-2 p-2 border-b">
                <button
                    class=move || if mode.get() == "about" { "active" } else { "" }
                    on:click=move |_| mode.set("about".to_string())
                >
                    "About NPC"
                </button>
                <button
                    class=move || if mode.get() == "as" { "active" } else { "" }
                    on:click=move |_| mode.set("as".to_string())
                >
                    "Speak as NPC"
                </button>
            </div>

            // Messages
            <div class="flex-1 overflow-y-auto p-4">
                <For each=messages ... />
            </div>

            // Input
            <NpcChatInput
                npc_id=npc_id
                campaign_id=campaign_id
                mode=mode
                system_prompt=system_prompt
            />
        </div>
    }
}
```

## Backend Commands

### Enhanced NPC Conversation Commands

```rust
// Get or create conversation for an NPC
#[tauri::command]
pub async fn get_or_create_npc_conversation(
    npc_id: String,
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<NpcConversation, String>;

// Add message to NPC conversation
#[tauri::command]
pub async fn add_npc_conversation_message(
    conversation_id: String,
    role: String,
    content: String,
    state: State<'_, AppState>,
) -> Result<(), String>;

// Stream chat as/about NPC
#[tauri::command]
pub async fn stream_npc_chat(
    npc_id: String,
    campaign_id: String,
    messages: Vec<StreamingChatMessage>,
    mode: String,  // "about" or "as"
    state: State<'_, AppState>,
) -> Result<String, String>;

// Get NPC conversation history
#[tauri::command]
pub async fn get_npc_conversation_messages(
    npc_id: String,
    campaign_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NpcMessage>, String>;
```

## System Prompts

### Prompt Sanitization

User-controlled NPC fields interpolated into system prompts can contain prompt injection attacks.
All fields MUST be sanitized before interpolation:

```rust
const MAX_FIELD_LENGTH: usize = 500;

/// Sanitize user-controlled text before interpolating into system prompts.
/// Prevents prompt injection via newlines, markdown, and control characters.
fn sanitize_for_prompt(text: &str) -> String {
    let mut result = text
        // Normalize newlines to spaces (prevents instruction injection via line breaks)
        .replace('\n', " ")
        .replace('\r', " ")
        // Remove markdown headers (could inject new sections)
        .replace("##", "")
        .replace("# ", "")
        // Remove markdown separators
        .replace("---", "")
        .replace("***", "")
        // Remove code block markers
        .replace("```", "")
        // Remove control characters
        .chars()
        .filter(|c| !c.is_control() || *c == ' ')
        .collect::<String>();

    // Collapse multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    // Enforce max length
    if result.len() > MAX_FIELD_LENGTH {
        result.truncate(MAX_FIELD_LENGTH);
        result.push_str("...");
    }

    result.trim().to_string()
}
```

### About Mode Prompt

```rust
fn build_about_mode_prompt(npc: &NpcRecord) -> String {
    format!(r#"
You are a TTRPG assistant helping develop an NPC.

## NPC Information
Name: {name}
Background: {background}
Personality: {personality}
Role in Campaign: {role}
Connections: {connections}

Help the GM:
- Develop deeper backstory and motivations
- Create interesting secrets or conflicts
- Suggest memorable mannerisms or phrases
- Plan character arcs and development
"#,
        name = sanitize_for_prompt(&npc.name),
        background = sanitize_for_prompt(npc.background.as_deref().unwrap_or("Unknown")),
        personality = sanitize_for_prompt(npc.personality.as_deref().unwrap_or("Not defined")),
        role = sanitize_for_prompt(npc.role.as_deref().unwrap_or("Minor NPC")),
        connections = sanitize_for_prompt(&format_npc_connections(npc)),
    )
}
```

### Voice Mode Prompt

```rust
fn build_voice_mode_prompt(npc: &NpcRecord) -> String {
    format!(r#"
You ARE {name}, an NPC in a TTRPG campaign. Stay in character at all times.

## Your Character
Background: {background}
Personality: {personality}
Speaking Style: {voice}
Current Situation: {situation}

## Instructions
- Respond in first person as {name}
- Use speech patterns matching your personality
- React authentically to questions
- Never break character or refer to being an AI
- Include *actions* and *emotions* in asterisks when appropriate
"#,
        name = sanitize_for_prompt(&npc.name),
        background = sanitize_for_prompt(npc.background.as_deref().unwrap_or("a mysterious past")),
        personality = sanitize_for_prompt(npc.personality.as_deref().unwrap_or("reserved")),
        voice = sanitize_for_prompt(npc.voice_description.as_deref().unwrap_or("speaks plainly")),
        situation = "in conversation with the party",
    )
}
```

## Data Flow

```
User clicks "Chat" on NPC card
         │
         ▼
┌──────────────────────────────────────────────────────────────────┐
│ 1. get_or_create_npc_conversation(npc_id, campaign_id)           │
│    - Returns existing or creates new record                      │
│    - Loads messages_json                                         │
└──────────────────────────────────────────────────────────────────┘
         │
         ▼
┌──────────────────────────────────────────────────────────────────┐
│ 2. NpcConversationPanel renders                                   │
│    - Mode toggle (About / As)                                     │
│    - Message history                                              │
│    - Input field                                                  │
└──────────────────────────────────────────────────────────────────┘
         │
         ▼ (User sends message)
┌──────────────────────────────────────────────────────────────────┐
│ 3. stream_npc_chat(npc_id, campaign_id, messages, mode)          │
│    - Builds system prompt based on mode                           │
│    - Streams response                                             │
│    - Saves to npc_conversations.messages_json                     │
└──────────────────────────────────────────────────────────────────┘
```

---

**Version:** 3.1
**Last Updated:** 2026-02-03
**Status:** Draft - Pending Review
**Implements:** Requirements.md v3.1
