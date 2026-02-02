# Chat Persistence & Campaign Integration - Implementation Tasks

## Overview

Sequenced tasks for:
1. Fixing chat persistence (Phase 1-2)
2. Adding campaign context (Phase 3-4)
3. Conversation threads UI (Phase 5-6)

## Task Sequence

### Phase 1: Fix Race Condition (Critical)

- [ ] **1.1 Block Input During Session Load**
  - File: `frontend/src/components/chat/mod.rs`
  - Change the Input disabled prop from `is_loading` to include `is_loading_history`
  - Line ~764: `disabled=Signal::derive(move || is_loading.get() || is_loading_history.get())`
  - Also block send button (line ~784)
  - _Requirements: FR-002, FR-003_

- [ ] **1.2 Add Session Guard to Send Function**
  - File: `frontend/src/components/chat/mod.rs`
  - In `send_message_streaming` (line ~385), add early return if `chat_session_id.get()` is None
  - Use `show_error()` to notify user if session not ready
  - _Requirements: FR-001, FR-003_

- [ ] **1.3 Verify Persistence Calls**
  - File: `frontend/src/components/chat/mod.rs`
  - Ensure `add_chat_message` is called with valid session_id (not Option)
  - Add debug logging to trace persistence flow
  - _Requirements: FR-001_

### Phase 2: Error Visibility

- [ ] **2.1 Add Persistence Error Toast**
  - File: `frontend/src/components/chat/mod.rs`
  - In user message persistence (line ~408-416), show toast on error
  - In assistant placeholder creation (line ~433-454), show toast on error
  - _Requirements: FR-006_

- [ ] **2.2 Add Stream Finalization Error Handling**
  - File: `frontend/src/components/chat/mod.rs`
  - In chunk listener when `is_final` (line ~263-282), handle update_chat_message error
  - Show toast if final content fails to persist
  - _Requirements: FR-006, FR-007_

### Phase 3: Loading UX

- [ ] **3.1 Add Loading Indicator**
  - File: `frontend/src/components/chat/mod.rs`
  - In message area (line ~722-755), add loading state display
  - Show "Loading conversation..." with animation during `is_loading_history`
  - _Requirements: FR-002_

- [ ] **3.2 Update Send Button State**
  - File: `frontend/src/components/chat/mod.rs`
  - When loading, show disabled send button with "..." or spinner
  - _Requirements: FR-002, FR-003_

### Phase 4: Testing

- [ ] **4.1 Manual Verification**
  - Start app fresh, send messages, verify DB:
    ```bash
    sqlite3 ~/.local/share/com.ttrpg.assistant/ttrpg_assistant.db \
      "SELECT COUNT(*) FROM chat_messages"
    ```
  - Navigate away and back, verify messages reload
  - Restart app, verify messages persist
  - _Requirements: AC-001, AC-002_

- [ ] **4.2 Race Condition Test**
  - Add artificial delay to `get_or_create_chat_session` (temporarily)
  - Open Chat and immediately try to send
  - Verify input is disabled until session ready
  - _Requirements: AC-003_

- [ ] **4.3 Error Handling Test**
  - Simulate DB error (e.g., change table name temporarily)
  - Send message, verify error toast appears
  - _Requirements: AC-004_

### Phase 5: Cleanup

- [ ] **5.1 Remove Debug Logging**
  - Remove any temporary console.log or debug statements added in Phase 1

- [ ] **5.2 Update CLAUDE.md**
  - Add note about chat persistence implementation
  - Document the race condition fix

## Code Snippets

### 1.1 Input Disabled Signal

```rust
// Before (line ~764)
disabled=is_loading

// After
disabled=Signal::derive(move || is_loading.get() || is_loading_history.get())
```

### 1.2 Session Guard

```rust
// In send_message_streaming (around line 385)
let send_message_streaming = move || {
    let msg = message_input.get();
    if msg.trim().is_empty() || is_loading.get() {
        return;
    }

    // NEW: Guard against missing session
    let session_id = match chat_session_id.get() {
        Some(id) => id,
        None => {
            show_error(
                "Chat Not Ready",
                Some("Please wait for the conversation to load."),
                None,
            );
            return;
        }
    };

    // Use session_id directly (not Option) in rest of function
    // ...
};
```

### 2.1 Persistence Error Toast

```rust
// User message persistence (around line 408-416)
if let Some(sid) = session_id_opt.clone() {
    let msg_content = msg.clone();
    spawn_local(async move {
        if let Err(e) = add_chat_message(sid, "user".to_string(), msg_content, None).await {
            log::error!("Failed to persist user message: {}", e);
            // NEW: Show error to user
            show_error(
                "Save Failed",
                Some(&format!("Message may not be saved: {}", e)),
                None,
            );
        }
    });
}
```

### 3.1 Loading Indicator

```rust
// In message area view (around line 722)
<div class="flex-1 p-4 overflow-y-auto space-y-4">
    {move || {
        if is_loading_history.get() {
            view! {
                <div class="flex items-center justify-center h-32 text-gray-400">
                    <div class="flex items-center gap-2">
                        <div class="w-4 h-4 border-2 border-gray-500 border-t-transparent rounded-full animate-spin"></div>
                        <span>"Loading conversation..."</span>
                    </div>
                </div>
            }.into_any()
        } else {
            view! {
                // ... existing For loop for messages ...
            }.into_any()
        }
    }}
</div>
```

---

## Campaign Integration Tasks

### Phase 3: Campaign Context Provider

- [ ] **3.1 Create Chat Context Service**
  - File: `frontend/src/services/chat_context.rs` (NEW)
  - Create `ChatContext` struct with campaign, session, NPCs, locations
  - Add `provide_chat_context()` function
  - Add `set_campaign_context()` and `clear_campaign_context()` functions
  - _Requirements: FR-100, FR-101_

- [ ] **3.2 Integrate Context Provider in App**
  - File: `frontend/src/app.rs`
  - Call `provide_chat_context()` after other providers
  - File: `frontend/src/services/mod.rs`
  - Export the new module
  - _Requirements: FR-100_

- [ ] **3.3 Load Campaign Data on Session Navigation**
  - File: `frontend/src/components/session/mod.rs`
  - On mount, call `set_campaign_context(campaign_id)`
  - Load campaign record, active session, NPCs, locations
  - On cleanup, call `clear_campaign_context()`
  - _Requirements: FR-100, FR-101_

### Phase 4: Context-Augmented Chat

- [ ] **4.1 Add System Prompt Parameter to stream_chat**
  - File: `src-tauri/src/commands/llm/streaming.rs`
  - Add `system_prompt: Option<String>` parameter
  - Use provided prompt or fall back to default
  - _Requirements: FR-102_

- [ ] **4.2 Update Frontend Binding**
  - File: `frontend/src/bindings/ai.rs`
  - Update `stream_chat()` to accept optional system_prompt
  - _Requirements: FR-102_

- [ ] **4.3 Build Context-Aware System Prompt in Chat**
  - File: `frontend/src/components/chat/mod.rs`
  - Read from `ChatContext`
  - Format campaign, NPCs, locations into system prompt
  - Pass to `stream_chat()`
  - _Requirements: FR-102, FR-108_

- [ ] **4.4 Link Chat Session to Campaign**
  - File: `frontend/src/components/chat/mod.rs`
  - When `ChatContext` has campaign, call `link_chat_to_game_session()`
  - Update on context change
  - _Requirements: FR-100, FR-101_

### Phase 5: Conversation Threads Backend

- [ ] **5.1 Add Conversation Thread Commands**
  - File: `src-tauri/src/commands/campaign/mod.rs`
  - Add `list_campaign_conversations(campaign_id, purpose)`
  - Add `get_thread_messages(thread_id, limit)`
  - Add `create_conversation_thread(campaign_id, purpose, title)`
  - Add `add_thread_message(thread_id, role, content)`
  - Re-export from `src-tauri/src/commands/mod.rs`
  - _Requirements: FR-103, FR-104, FR-107_

- [ ] **5.2 Register Commands in Tauri**
  - File: `src-tauri/src/lib.rs`
  - Add new commands to `invoke_handler()`
  - _Requirements: FR-103, FR-104_

- [ ] **5.3 Add Frontend Bindings for Threads**
  - File: `frontend/src/bindings/ai.rs` or new `conversation.rs`
  - Add bindings for new thread commands
  - _Requirements: FR-103, FR-104, FR-107_

### Phase 6: Session Chat Panel UI

- [ ] **6.1 Create Thread Tabs Component**
  - File: `frontend/src/components/session/thread_tabs.rs` (NEW)
  - Display tabs for conversation threads
  - "General" tab for global chat
  - "+ New Thread" button
  - _Requirements: FR-105, FR-106_

- [ ] **6.2 Create Session Chat Panel**
  - File: `frontend/src/components/session/chat_panel.rs` (NEW)
  - Integrate thread tabs
  - Show messages for active thread
  - Route input to correct thread
  - _Requirements: FR-105, FR-106_

- [ ] **6.3 Add Chat Panel to Session Workspace**
  - File: `frontend/src/components/session/mod.rs`
  - Add `SessionChatPanel` to session layout
  - Position in sidebar or collapsible panel
  - _Requirements: FR-106_

- [ ] **6.4 Create New Thread Dialog**
  - File: `frontend/src/components/session/new_thread_dialog.rs` (NEW)
  - Purpose selector (SessionPlanning, NpcGeneration, WorldBuilding)
  - Title input
  - Create button
  - _Requirements: FR-104_

### Phase 7: Session Planning Flow

- [ ] **7.1 Add Session Planning Purpose Prompt**
  - File: `src-tauri/src/core/campaign/conversation/prompts.rs` (or similar)
  - Define system prompt for `SessionPlanning` purpose
  - Include session notes, previous session summary
  - _Requirements: FR-103_

- [ ] **7.2 "Plan Session" Quick Action**
  - File: `frontend/src/components/session/control_panel.rs`
  - Add "Plan Session" button
  - Creates new thread with `SessionPlanning` purpose
  - Opens chat panel with thread active
  - _Requirements: FR-103, FR-106_

### Phase 8: Campaign Conversation History

- [ ] **8.1 Add Conversations Tab to Campaign Details**
  - File: `frontend/src/components/campaign_details/mod.rs`
  - Add "Conversations" tab alongside existing tabs
  - _Requirements: FR-107_

- [ ] **8.2 Create Conversation List Component**
  - File: `frontend/src/components/campaign_details/conversation_list.rs` (NEW)
  - List threads grouped by purpose
  - Show thread title, message count, last updated
  - Click to view/resume thread
  - _Requirements: FR-107_

---

## Verification Checklist

### Core Persistence

- [ ] Send message → message appears in DB
- [ ] Navigate away → messages still in DB
- [ ] Navigate back → messages reload from DB
- [ ] Restart app → messages reload from DB
- [ ] Rapid send (before load) → blocked with message
- [ ] Persistence error → toast notification shown

### Campaign Integration

- [ ] Navigate to session workspace → chat knows campaign context
- [ ] Ask about NPCs → AI lists campaign NPCs
- [ ] Ask about setting → AI describes campaign setting
- [ ] Leave session workspace → context cleared

### Conversation Threads

- [ ] Create planning thread → persists to DB
- [ ] Add messages to thread → messages saved
- [ ] Switch threads → correct history loads
- [ ] View campaign conversations → all threads listed

### NPC Conversations

- [ ] Click "Chat" on NPC → conversation panel opens
- [ ] Send message about NPC → AI responds with NPC context
- [ ] Toggle to "Speak as NPC" → AI responds in character
- [ ] Navigate away and back → conversation persists
- [ ] View NPC detail → conversation history shown

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking existing functionality | Test each phase independently |
| Performance regression | Monitor load time, keep under 500ms |
| UI freeze on await | Use spawn_local for non-critical persistence |
| Context data too large | Summarize NPCs/locations, limit count |
| Thread switching lag | Cache recently accessed threads |

---

## NPC Conversation Tasks

### Phase 9: NPC Conversation Backend

- [ ] **9.1 Add NPC Conversation Commands**
  - File: `src-tauri/src/commands/npc.rs` or new file
  - Add `get_or_create_npc_conversation(npc_id, campaign_id)`
  - Add `get_npc_conversation_messages(npc_id, campaign_id)`
  - Add `add_npc_conversation_message(conversation_id, role, content)`
  - Register in Tauri invoke_handler
  - _Requirements: FR-200, FR-206_

- [ ] **9.2 Add NPC Streaming Command**
  - File: `src-tauri/src/commands/llm/streaming.rs`
  - Add `stream_npc_chat(npc_id, campaign_id, messages, mode)`
  - Build system prompt based on mode ("about" or "as")
  - Persist messages to npc_conversations table
  - _Requirements: FR-200, FR-201, FR-206_

- [ ] **9.3 Create NPC System Prompts**
  - File: `src-tauri/src/core/llm/prompts/npc.rs` (NEW)
  - `build_about_mode_prompt(npc)` - DM assistant mode
  - `build_voice_mode_prompt(npc)` - Roleplay as NPC mode
  - Include NPC data, personality, connections
  - _Requirements: FR-201, FR-205_

- [ ] **9.4 Add Frontend Bindings**
  - File: `frontend/src/bindings/npc.rs` or add to existing
  - Bindings for all NPC conversation commands
  - Type definitions for NpcConversation, NpcMessage
  - _Requirements: FR-200, FR-203_

### Phase 10: NPC Conversation UI

- [ ] **10.1 Create NpcConversationPanel Component**
  - File: `frontend/src/components/campaign_details/npc_chat.rs` (NEW)
  - Mode toggle (About NPC / Speak as NPC)
  - Message display with role styling
  - Input with send button
  - Load/save to npc_conversations
  - _Requirements: FR-200, FR-201, FR-203_

- [ ] **10.2 Add Chat Button to NPC Cards**
  - File: `frontend/src/components/campaign_details/npc_list.rs`
  - Add "Chat" icon button to each NPC card
  - Opens NpcConversationPanel in modal or side panel
  - _Requirements: FR-203_

- [ ] **10.3 Show Conversations on NPC Detail Page**
  - File: `frontend/src/components/campaign_details/npc_detail.rs` (if exists)
  - Add conversation panel or link
  - Show conversation count badge
  - Display last interaction date
  - _Requirements: FR-202_

- [ ] **10.4 Style Voice Mode Responses**
  - File: `frontend/src/components/campaign_details/npc_chat.rs`
  - Different styling for "as NPC" responses
  - Show NPC name/avatar for roleplay messages
  - Italic text for *actions*
  - _Requirements: FR-201, FR-205_

### Phase 11: NPC Development Features

- [ ] **11.1 Personality Extraction Suggestions**
  - File: `src-tauri/src/core/llm/prompts/npc.rs`
  - Add prompts that generate updateable suggestions
  - Parse `suggestion` JSON blocks from responses
  - _Requirements: FR-204_

- [ ] **11.2 "Apply to NPC" Button**
  - File: `frontend/src/components/campaign_details/npc_chat.rs`
  - When AI suggests traits, show "Apply" button
  - Updates NPC record with new personality/background
  - _Requirements: FR-204_

- [ ] **11.3 Conversation History in NPC List**
  - File: `frontend/src/components/campaign_details/npc_list.rs`
  - Show badge for NPCs with conversations
  - Show unread count if applicable
  - Sort by recent interaction option
  - _Requirements: FR-202_

---

## Dependencies Between Tasks

```
Phase 1-2 (Core Persistence)
    │
    ▼
Phase 3-4 (Campaign Context) ───────────────┬───────────────────┐
    │                                        │                   │
    ▼                                        ▼                   ▼
Phase 5 (Thread Backend)              Phase 6 (Thread UI)   Phase 9 (NPC Backend)
    │                                        │                   │
    └──────────────┬─────────────────────────┘                   │
                   │                                             │
                   ▼                                             ▼
            Phase 7-8 (Session Planning)                  Phase 10-11 (NPC UI)
```

---

**Version:** 3.0
**Last Updated:** 2026-02-01
**Implements:** Design.md v3.0
**Status:** Ready for Implementation
