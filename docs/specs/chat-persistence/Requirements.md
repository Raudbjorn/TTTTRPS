# Chat Persistence & Campaign Integration - Requirements Specification

## Overview

Chat conversations in the TTRPG Assistant must:
1. Persist across navigation, app restarts, and system reboots
2. Integrate with campaigns and game sessions for contextual assistance
3. Support purpose-driven conversation threads (session planning, NPC generation, etc.)

Currently, chat state is lost when navigating away from the Chat component despite existing SQLite persistence infrastructure. Additionally, the existing `linked_campaign_id` and `linked_game_session_id` fields in `global_chat_sessions` are not being utilized.

## Problem Analysis

### Root Cause Investigation

**Evidence:**
- Database has 1 active `global_chat_sessions` record
- Database has 0 `chat_messages` records
- SQLite infrastructure (tables, migrations, commands) is complete
- Frontend bindings correctly map to Tauri commands

**Likely Cause: Race Condition**
The session loading happens in an async `spawn_local` effect. If a user sends a message before `get_or_create_chat_session()` completes, `chat_session_id.get()` returns `None` and persistence is silently skipped:

```rust
// Line 391: Gets session ID (may be None if async load hasn't completed)
let session_id_opt = chat_session_id.get();

// Line 409: Only persists if session exists
if let Some(sid) = session_id_opt.clone() {
    // This block is skipped if session_id_opt is None
    spawn_local(async move {
        add_chat_message(sid, ...).await
    });
}
```

### Architecture Context

| Component | State Management | Notes |
|-----------|-----------------|-------|
| SQLite (local) | Persistent | Tables exist, CRUD operations work |
| Meilisearch Chat | Stateless | Uses `_meiliAppendConversationMessage` tool for client tracking |
| LLM Providers | Stateless | Full history must be sent with each request |
| Leptos Frontend | Ephemeral | RwSignal state lost on component unmount |

## User Stories

### US-001: Persistent Conversation History
**As a** Game Master
**I want** my chat conversations to persist when I navigate away and return
**So that** I don't lose context during a game session

### US-002: Cross-Session Continuity
**As a** Game Master
**I want** to resume my previous conversation when I restart the app
**So that** I can continue where I left off between sessions

### US-003: Conversation Archive Access
**As a** Game Master
**I want** to access previous chat conversations
**So that** I can reference past DM advice and rulings

### US-004: Persistence Visibility
**As a** Game Master
**I want** visual confirmation that my messages are being saved
**So that** I can trust the system is working

### US-005: Campaign-Aware Conversations
**As a** Game Master
**I want** my chat to understand my current campaign context
**So that** the AI can reference my NPCs, locations, and world state

### US-006: Session Planning Assistance
**As a** Game Master
**I want** to have planning conversations tied to specific game sessions
**So that** my session prep is organized and retrievable

### US-007: Contextual Chat Switching
**As a** Game Master
**I want** to switch between general chat and campaign-specific conversations
**So that** I can keep different contexts separate

### US-008: Session Workspace Integration
**As a** Game Master
**I want** to access relevant chat history from within the session workspace
**So that** I can reference planning discussions during gameplay

### US-009: NPC Conversations
**As a** Game Master
**I want** to have conversations specifically about or "as" an NPC
**So that** I can develop their personality, backstory, and prepare for roleplay

### US-010: Character-Linked Dialogue
**As a** Game Master
**I want** conversation threads linked to specific NPCs
**So that** I can build consistent characterization over time

### US-011: NPC Voice Development
**As a** Game Master
**I want** to practice dialogue with an NPC before a session
**So that** I'm prepared for player interactions

## Functional Requirements

### FR-001: Guaranteed Message Persistence
**WHEN** user sends a message
**THEN** system **SHALL** persist the message to SQLite before displaying the response
**AND** system **SHALL** block message sending until session ID is available

### FR-002: Session Load on Mount
**WHEN** Chat component mounts
**THEN** system **SHALL** load the active session and its messages before enabling input
**AND** system **SHALL** display a loading indicator during this process

### FR-003: Race Condition Prevention
**WHEN** session loading is in progress
**THEN** system **SHALL** disable the send button
**AND** system **SHALL** queue or block message sends until session is ready

### FR-004: Message Load on Return
**WHEN** user navigates back to Chat
**THEN** system **SHALL** reload all messages from the active session
**AND** system **SHALL** maintain chronological order

### FR-005: App Restart Recovery
**WHEN** app starts
**THEN** system **SHALL** load the most recent active session
**AND** system **SHALL** display all persisted messages from that session

### FR-006: Persistence Error Handling
**IF** message persistence fails
**THEN** system **SHALL** display an error notification to the user
**AND** system **SHALL** retry persistence up to 3 times
**AND** system **SHALL** log the error with full context

### FR-007: Streaming Message Finalization
**WHEN** streaming response completes
**THEN** system **SHALL** update the message record with final content
**AND** system **SHALL** update token usage counts
**AND** system **SHALL NOT** mark as complete until database update succeeds

### FR-008: Session Archival
**WHEN** user explicitly ends a conversation (future feature)
**THEN** system **SHALL** archive the current session
**AND** system **SHALL** create a new active session
**AND** system **SHALL** clear the message display

---

## Campaign Integration Requirements

### FR-100: Campaign Context Linking
**WHEN** user navigates to `/session/:campaign_id`
**THEN** system **SHALL** set `linked_campaign_id` on the active chat session
**AND** system **SHALL** include campaign context in AI prompts

### FR-101: Session Context Linking
**WHEN** user starts or resumes a game session
**THEN** system **SHALL** set `linked_game_session_id` on the active chat session
**AND** system **SHALL** include session notes and events in AI prompts

### FR-102: Context-Aware System Prompt
**WHEN** chat session has `linked_campaign_id`
**THEN** system **SHALL** inject campaign name, setting, system, and world state into system prompt
**AND** system **SHALL** provide access to NPC summaries and location data

### FR-103: Session Planning Threads
**WHEN** user creates a session planning conversation
**THEN** system **SHALL** create a `conversation_thread` with purpose `SessionPlanning`
**AND** system **SHALL** link it to the target session via `session_id`
**AND** system **SHALL** persist all messages to `conversation_messages` table

### FR-104: Purpose-Based Conversation Creation
**WHEN** user initiates a specialized conversation (NPC generation, world building, etc.)
**THEN** system **SHALL** create a `conversation_thread` with appropriate `ConversationPurpose`
**AND** system **SHALL** use purpose-specific system prompts
**AND** system **SHALL** optionally link to campaign/session

### FR-105: Conversation Thread Switching
**WHEN** user switches between conversation threads
**THEN** system **SHALL** save current thread state
**AND** system **SHALL** load target thread's message history
**AND** system **SHALL** update AI context to match target thread's purpose

### FR-106: Session Workspace Chat Panel
**WHEN** user is in session workspace (`/session/:campaign_id`)
**THEN** system **SHALL** display a chat panel with campaign context
**AND** system **SHALL** show recent session planning threads
**AND** system **SHALL** allow quick access to NPC conversations

### FR-107: Campaign Conversation History
**WHEN** user views campaign details
**THEN** system **SHALL** list all conversation threads linked to that campaign
**AND** system **SHALL** group by purpose (planning, NPCs, world building)
**AND** system **SHALL** allow viewing and resuming threads

### FR-108: Auto-Context Detection
**WHEN** user mentions an NPC, location, or entity by name in chat
**THEN** system **SHOULD** detect the reference
**AND** system **SHOULD** include relevant entity data in the next AI request

---

## NPC Conversation Requirements

### FR-200: NPC-Linked Conversation Thread
**WHEN** user creates a conversation about an NPC
**THEN** system **SHALL** create a `conversation_thread` with `npc_id` reference
**AND** system **SHALL** include NPC data (name, background, personality, voice) in system prompt

### FR-201: NPC Voice Mode
**WHEN** user toggles "Speak as NPC" mode in an NPC conversation
**THEN** system **SHALL** instruct AI to respond as the NPC character
**AND** system **SHALL** use the NPC's personality traits and speech patterns
**AND** system **SHALL** maintain first-person perspective for NPC responses

### FR-202: NPC Conversation History
**WHEN** user views an NPC's detail page
**THEN** system **SHALL** display all conversation threads linked to that NPC
**AND** system **SHALL** show conversation count and last interaction date

### FR-203: NPC Quick Chat
**WHEN** user clicks "Chat" on an NPC card
**THEN** system **SHALL** open or resume the most recent conversation for that NPC
**AND** system **SHALL** pre-load NPC context into the system prompt

### FR-204: NPC Personality Extraction
**WHEN** user has a conversation developing an NPC
**THEN** system **SHOULD** offer to update NPC record with discovered traits
**AND** system **SHALL** track suggestions in `generation_drafts` table

### FR-205: Character Dialogue Practice
**WHEN** user is in "Voice Mode" for an NPC conversation
**THEN** system **SHALL** allow simulated player questions
**AND** system **SHALL** respond as the NPC would
**AND** system **SHOULD** offer suggestions for improving NPC voice consistency

### FR-206: NPC Conversation Persistence
**WHEN** NPC conversation messages are sent
**THEN** system **SHALL** persist to `npc_conversations` table
**AND** system **SHALL** update `last_message_at` timestamp
**AND** system **SHALL** increment `unread_count` for assistant messages

## Non-Functional Requirements

### Performance Requirements

#### NFR-001: Persistence Latency
System **SHALL** complete message persistence within 100ms under normal conditions.
System **SHALL** complete persistence within 500ms under degraded conditions (retry scenario).

#### NFR-002: Load Time
System **SHALL** load and display up to 100 messages within 500ms on mount.
System **SHALL** provide incremental loading indicator for histories exceeding 100 messages.

#### NFR-003: UI Responsiveness
System **SHALL NOT** block the main UI thread during persistence operations.
System **SHALL** maintain 60fps animation during streaming responses.
System **SHALL** respond to user input within 100ms.

### Reliability Requirements

#### NFR-004: Database Integrity
System **SHALL** use SQLite WAL mode for crash recovery.
System **SHALL** use transactions for multi-step operations.
System **SHALL** validate foreign key constraints on write operations.

#### NFR-005: Offline Resilience
**WHEN** LLM provider is unavailable
**THEN** system **SHALL** still persist user messages locally.
**WHEN** database is temporarily unavailable (locked)
**THEN** system **SHALL** retry up to 3 times with exponential backoff.

#### NFR-006: Data Durability
System **SHALL** persist messages before marking send as complete.
System **SHALL NOT** lose messages during normal operation, navigation, or app restart.
System **SHALL** queue failed persistence operations for retry (max 100 pending).

#### NFR-007: Recovery Time Objective
System **SHALL** recover from transient failures within 30 seconds.
System **SHALL** notify user of unrecoverable failures within 5 seconds.

### Scalability Requirements

#### NFR-008: Message History Depth
System **SHALL** support at least 10,000 messages per chat session.
System **SHALL** paginate message loading for histories exceeding 100 messages.

#### NFR-009: Concurrent Operations
System **SHALL** handle up to 10 simultaneous persistence operations.
System **SHALL** serialize writes to the same chat session to maintain order.

#### NFR-010: Storage Limits
System **SHALL** warn user when database exceeds 500MB.
System **SHOULD** provide archival mechanism for old sessions (future).

### Security Requirements

#### NFR-011: Prompt Injection Prevention
System **SHALL** sanitize all user-controlled text before injecting into system prompts.
System **SHALL** remove markdown formatting, code blocks, and control characters from NPC fields.
System **SHALL** enforce maximum field length (500 chars) for interpolated data.

#### NFR-012: Data Isolation
System **SHALL** isolate chat data per user account (future multi-user).
System **SHALL NOT** expose chat data via unauthenticated IPC commands.

### Usability Requirements

#### NFR-013: Error Visibility
System **SHALL** display user-friendly error messages within 2 seconds of failure.
System **SHALL** provide actionable guidance ("Retry" button) for recoverable errors.
System **SHALL** log detailed error context for debugging.

#### NFR-014: State Indication
System **SHALL** visually indicate loading state (spinner/skeleton).
System **SHALL** visually indicate streaming state (animated cursor).
System **SHALL** visually indicate persistence failure (error icon on message).

## Constraints

### C-001: Meilisearch Chat Statelessness
Meilisearch Chat API does not persist conversation history. The `_meiliAppendConversationMessage` tool is a request to the client to track context, not server-side storage.

### C-002: LLM Provider Statelessness
All supported LLM providers (Claude, Gemini, OpenAI, Ollama) are stateless. Full conversation history must be passed with each request.

### C-003: Single Active Session
Database constraint allows only one active chat session at a time (enforced by partial unique index).

## Acceptance Criteria

### AC-001: Navigation Persistence
1. Send a message and receive a response
2. Navigate to Settings page
3. Navigate back to Chat
4. **Expected:** Previous messages are visible

### AC-002: App Restart Persistence
1. Send multiple messages with responses
2. Close and restart the application
3. Navigate to Chat
4. **Expected:** All previous messages are visible in order

### AC-003: Rapid Message Prevention
1. Open Chat for the first time (fresh session)
2. Immediately attempt to send a message
3. **Expected:** Send is blocked or queued until session loads

### AC-004: Error Visibility
1. Simulate persistence failure (e.g., disk full)
2. Send a message
3. **Expected:** Error notification appears, message marked as failed

### AC-005: Campaign Context Injection
1. Create a campaign with NPCs and locations
2. Navigate to session workspace for that campaign
3. Open chat and ask "Who are the NPCs in this campaign?"
4. **Expected:** AI lists the campaign's NPCs by name

### AC-006: Session Planning Thread
1. Navigate to session workspace
2. Click "Plan Session" to create planning thread
3. Discuss session structure with AI
4. Navigate away and back
5. **Expected:** Planning conversation is preserved and accessible

### AC-007: Thread Switching
1. Have an active general chat
2. Switch to a campaign-specific thread
3. Switch back to general chat
4. **Expected:** Each thread retains its own message history

### AC-008: Campaign Conversation List
1. Create multiple conversation threads for a campaign
2. Open campaign details
3. **Expected:** All threads listed, grouped by purpose

### AC-009: NPC Conversation Creation
1. Navigate to NPC detail page
2. Click "Chat with [NPC Name]"
3. Send a message asking about the NPC's background
4. **Expected:** AI responds with NPC-aware context

### AC-010: NPC Voice Mode
1. Open NPC conversation
2. Enable "Speak as NPC" toggle
3. Ask "What do you think of the party?"
4. **Expected:** AI responds in first person as the NPC

### AC-011: NPC Conversation Persistence
1. Have a conversation about an NPC
2. Navigate away
3. Return to NPC detail page
4. **Expected:** Previous conversation visible and resumable

## Out of Scope (v1)

- Full-text search within chat history (use Meilisearch later)
- Export/import conversations
- Multi-device sync
- End-to-end encryption
- Message editing after send
- Real-time collaborative chat (multiple users)
- Voice transcription integration

## Dependencies

- SQLite database with migrations V18+ (global_chat_sessions, chat_messages)
- SQLite database with migrations V22+ (conversation_threads, conversation_messages)
- Campaign and Session models (migrations V1+)
- Tauri IPC commands for chat and conversations
- Frontend bindings for all CRUD operations
- ConversationManager in `src-tauri/src/core/campaign/conversation/`

## Traceability

### Core Persistence

| Requirement | User Story | Test Case |
|------------|------------|-----------|
| FR-001 | US-001 | AC-001, AC-003 |
| FR-002 | US-001 | AC-003 |
| FR-003 | US-001 | AC-003 |
| FR-004 | US-001 | AC-001 |
| FR-005 | US-002 | AC-002 |
| FR-006 | US-004 | AC-004 |
| FR-007 | US-001 | AC-001 |

### Campaign Integration

| Requirement | User Story | Test Case |
|------------|------------|-----------|
| FR-100 | US-005 | AC-005 |
| FR-101 | US-005 | AC-005 |
| FR-102 | US-005 | AC-005 |
| FR-103 | US-006 | AC-006 |
| FR-104 | US-006 | AC-006 |
| FR-105 | US-007 | AC-007 |
| FR-106 | US-008 | AC-006 |
| FR-107 | US-003 | AC-008 |
| FR-108 | US-005 | AC-005 |

### NPC Conversations

| Requirement | User Story | Test Case |
|------------|------------|-----------|
| FR-200 | US-009 | AC-009 |
| FR-201 | US-011 | AC-010 |
| FR-202 | US-010 | AC-011 |
| FR-203 | US-009 | AC-009 |
| FR-204 | US-009 | AC-009 |
| FR-205 | US-011 | AC-010 |
| FR-206 | US-010 | AC-011 |

---

**Version:** 3.1
**Last Updated:** 2026-02-03
**Status:** Draft - Pending Review
