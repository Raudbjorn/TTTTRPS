# Requirements: Unified Gate Trait

## Introduction

The TTRPG Assistant application currently has two separate OAuth-based API client implementations: `claude` for Anthropic's Claude API and `gemini` for Google's Gemini API via Cloud Code. Both modules share substantial overlapping functionality including OAuth 2.0 with PKCE authentication flows, token storage backends, error handling patterns, and client lifecycle management.

This refactoring aims to extract the common patterns into a unified `gate` trait/interface that both implementations can share, reducing code duplication, improving maintainability, and establishing a consistent contract for adding future API providers. The unified architecture will make it easier to swap providers, share storage implementations, and apply consistent error handling across the application.

## Requirements

### Requirement 1: Unified Token Storage Trait

**User Story:** As a developer, I want a single token storage interface that works with any OAuth provider, so that I can reuse storage backends (file, keyring, memory) across providers without duplication.

#### Acceptance Criteria

1. WHEN a new storage backend is created THEN system SHALL implement a single `TokenStorage` trait applicable to both Claude and Gemini tokens
2. IF token storage is provider-agnostic THEN system SHALL store tokens keyed by provider identifier (e.g., "anthropic", "google")
3. WHEN loading tokens THEN system SHALL return `Option<TokenInfo>` where `TokenInfo` is provider-agnostic
4. WHEN saving tokens THEN system SHALL persist access_token, refresh_token, expires_at, and token_type
5. IF storage backend is unavailable THEN system SHALL return appropriate error without panicking
6. WHEN file storage is used THEN system SHALL enforce 0600 permissions on Unix systems
7. WHEN keyring storage is used THEN system SHALL check availability before attempting operations

### Requirement 2: Unified Token Model

**User Story:** As a developer, I want a single token model that represents OAuth tokens from any provider, so that storage and auth flows can be provider-agnostic.

#### Acceptance Criteria

1. WHEN creating a TokenInfo THEN system SHALL accept access_token, refresh_token, and expires_in (seconds)
2. WHEN checking expiration THEN system SHALL use `is_expired()` returning true if current time >= expires_at
3. WHEN checking refresh need THEN system SHALL use `needs_refresh()` returning true if token expires within 5 minutes
4. IF token has custom fields THEN system SHALL allow provider-specific extension via serde flatten or additional fields
5. WHEN serializing tokens THEN system SHALL produce JSON compatible with existing file formats for backward compatibility

### Requirement 3: Unified OAuth Flow Trait

**User Story:** As a developer, I want a common OAuth flow interface for any OAuth 2.0 provider with PKCE, so that authentication code can be shared and new providers added easily.

#### Acceptance Criteria

1. WHEN starting authorization THEN system SHALL generate PKCE challenge/verifier pair using SHA256
2. WHEN building authorization URL THEN system SHALL include client_id, redirect_uri, scopes, code_challenge, and state
3. WHEN exchanging authorization code THEN system SHALL validate state parameter to prevent CSRF attacks
4. IF state mismatch occurs THEN system SHALL return `Error::Auth(AuthError::StateMismatch)`
5. WHEN token exchange succeeds THEN system SHALL automatically save token to configured storage
6. WHEN getting access token THEN system SHALL automatically refresh if token is expired or expiring soon
7. IF refresh fails with invalid_grant THEN system SHALL return `Error::Auth(AuthError::InvalidGrant)`
8. WHEN checking authentication status THEN system SHALL return bool indicating valid token exists
9. WHEN logging out THEN system SHALL remove token from storage and clear pending flow state

### Requirement 4: Unified Error Handling

**User Story:** As a developer, I want consistent error types across all gate implementations, so that error handling code can be shared and error responses are predictable.

#### Acceptance Criteria

1. WHEN auth error occurs THEN system SHALL return `Error::Auth(AuthError)` with specific variant
2. WHEN API error occurs THEN system SHALL return `Error::Api` with status, message, and optional retry_after
3. WHEN storage error occurs THEN system SHALL return `Error::Storage` with descriptive message
4. WHEN network error occurs THEN system SHALL return `Error::Network` wrapping reqwest::Error
5. IF error is rate limit (429) THEN system SHALL provide `is_rate_limit()` and `retry_after()` methods
6. IF error requires re-authentication THEN system SHALL provide `requires_reauth()` returning true for 401 and auth failures
7. WHEN converting from external errors THEN system SHALL implement `From` for reqwest::Error, serde_json::Error, std::io::Error

### Requirement 5: Provider-Specific Configuration

**User Story:** As a developer, I want to configure each provider with its specific OAuth endpoints and scopes while using the unified interface, so that I can support multiple providers without code duplication.

#### Acceptance Criteria

1. WHEN creating a provider config THEN system SHALL accept client_id, auth_url, token_url, redirect_uri, and scopes
2. IF provider has client_secret THEN system SHALL allow optional client_secret configuration
3. WHEN using Claude/Anthropic THEN system SHALL default to Anthropic's OAuth endpoints and scopes
4. WHEN using Gemini/Google THEN system SHALL default to Google's OAuth endpoints and Cloud Code scopes
5. IF provider requires custom callback port THEN system SHALL allow configurable callback_port
6. WHEN provider config is incomplete THEN system SHALL validate required fields and return Config error

### Requirement 6: Backward Compatibility

**User Story:** As a developer, I want the refactored gate to maintain compatibility with existing stored tokens and API consumers, so that users don't need to re-authenticate.

#### Acceptance Criteria

1. WHEN loading existing claude tokens THEN system SHALL successfully deserialize from current JSON format
2. WHEN loading existing gemini tokens THEN system SHALL successfully deserialize from current JSON format
3. IF new unified format differs THEN system SHALL provide migration path or dual-format support
4. WHEN existing commands.rs code calls gate functions THEN system SHALL maintain compatible function signatures or provide adapters
5. WHEN tests reference gate types THEN system SHALL maintain type compatibility or provide clear migration path

## Non-Functional Requirements

### NFR-1: Code Reduction
- WHEN refactoring is complete THEN combined gate code SHALL be at least 30% smaller than sum of current implementations

### NFR-2: Performance
- WHEN performing OAuth operations THEN system SHALL add no more than 1ms overhead compared to current implementations

### NFR-3: Type Safety
- WHEN using unified traits THEN system SHALL leverage Rust's type system to prevent invalid state combinations at compile time

### NFR-4: Thread Safety
- WHEN trait implementations are used concurrently THEN system SHALL be Send + Sync compatible for async runtime use

## Constraints and Assumptions

### Constraints
1. Must use async/await for all I/O operations (tokio runtime)
2. Must maintain compatibility with Tauri 2.1 command system
3. Must support both `keyring` feature-gated and non-keyring builds
4. Cannot break existing LLM provider integrations in `core/llm/providers/`

### Assumptions
1. Both Claude and Gemini use standard OAuth 2.0 with PKCE (which they do)
2. Token refresh mechanics are similar enough to share (access_token + refresh_token + expires_in)
3. File storage JSON schema can be unified or made compatible
4. Provider-specific API calls (messages, chat) remain separate from auth/storage
