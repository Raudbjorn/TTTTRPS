# Tasks: Unified Gate Trait

## Implementation Overview

This implementation follows a **foundation-first** strategy: build shared infrastructure first (types, traits, storage), then implement providers on top, and finally migrate consumers. Each task produces working, testable code that builds on previous tasks.

The implementation is organized into 6 phases:
1. **Foundation** - Core types, errors, traits
2. **Storage** - Token storage backends
3. **Auth** - PKCE and OAuth flow
4. **Providers** - Claude and Gemini implementations
5. **Integration** - Wire up to commands.rs
6. **Cleanup** - Remove old code

---

## Implementation Plan

### Phase 1: Foundation

- [ ] 1. Set up module structure and core types
- [ ] 1.1 Create gate module structure
  - Create `src-tauri/src/gate/mod.rs` with module declarations
  - Create empty files: `error.rs`, `token.rs`, `storage/mod.rs`, `auth/mod.rs`, `providers/mod.rs`
  - Add `pub mod gate;` to `src-tauri/src/lib.rs` or `main.rs`
  - Verify compilation with `cargo check`
  - _Requirements: N/A (infrastructure)_

- [ ] 1.2 Implement unified Error and AuthError types
  - Create `gate/error.rs` with `Error` enum (Auth, Api, Network, Json, Config, Storage, Io)
  - Create `AuthError` enum (NotAuthenticated, TokenExpired, InvalidGrant, StateMismatch, Cancelled, PkceVerificationFailed)
  - Implement `From` traits for reqwest::Error, serde_json::Error, std::io::Error
  - Add helper methods: `config()`, `storage()`, `api()`, `is_rate_limit()`, `is_auth_error()`, `requires_reauth()`, `retry_after()`
  - Write unit tests for error conversions and helper methods
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_

- [ ] 1.3 Implement TokenInfo model with backward compatibility
  - Create `gate/token.rs` with `TokenInfo` struct
  - Add serde aliases for backward compat: `#[serde(alias = "access")]`, `#[serde(alias = "expires")]`
  - Implement `new()`, `is_expired()`, `needs_refresh()`, `time_until_expiry()`
  - Add default for `token_type` when deserializing
  - Write unit tests for expiration logic
  - Write tests loading JSON from existing claude and gemini formats
  - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 6.1, 6.2_

### Phase 2: Storage

- [ ] 2. Implement storage backends
- [ ] 2.1 Create TokenStorage trait
  - Create `gate/storage/mod.rs` with `TokenStorage` trait
  - Define async methods: `load()`, `save()`, `remove()` with provider parameter
  - Add default `exists()` implementation
  - Add `name()` method for debugging
  - Implement blanket impls for `Arc<T>` and `Box<T>`
  - Write trait documentation with examples
  - _Requirements: 1.1, 1.2, 1.3, 1.4_

- [ ] 2.2 Implement MemoryTokenStorage
  - Create `gate/storage/memory.rs`
  - Use `Arc<RwLock<HashMap<String, TokenInfo>>>` for thread-safe storage
  - Implement `TokenStorage` trait
  - Add `new()` and `with_token()` constructors
  - Write unit tests for CRUD operations
  - _Requirements: 1.1_

- [ ] 2.3 Implement FileTokenStorage
  - Create `gate/storage/file.rs`
  - Support `~` home directory expansion
  - Use JSON format: `{ "provider_name": TokenInfo }`
  - Set 0600 permissions on Unix with `#[cfg(unix)]`
  - Create parent directories with 0700 permissions
  - Implement `default_path()` returning `~/.config/gate/tokens.json`
  - Write unit tests with tempfile
  - Test backward compatibility loading existing token files
  - _Requirements: 1.1, 1.5, 1.6_

- [ ] 2.4 Implement KeyringTokenStorage (feature-gated)
  - Create `gate/storage/keyring.rs` with `#[cfg(feature = "keyring")]`
  - Implement `is_available()` with write/read test and caching
  - Store JSON-serialized token in keyring
  - Use service name "gate-{provider}" for isolation
  - Handle keyring::Error appropriately
  - Write unit tests (skip when keyring unavailable)
  - _Requirements: 1.1, 1.5, 1.7_

- [ ] 2.5 Implement CallbackStorage for custom backends
  - Create `gate/storage/callback.rs`
  - Accept load/save/remove closures returning futures
  - Add type aliases for callback signatures
  - Add `FileSource` and `EnvSource` helper factories
  - Write unit tests with mock callbacks
  - _Requirements: 1.1_

### Phase 3: Auth

- [ ] 3. Implement OAuth authentication
- [ ] 3.1 Implement PKCE module
  - Create `gate/auth/pkce.rs`
  - Implement `Pkce::generate()` using sha2 and base64
  - Use 32 random bytes for verifier
  - Generate S256 challenge from verifier
  - Implement `Pkce::verify()` for testing
  - Write unit tests for generation and verification
  - _Requirements: 3.1_

- [ ] 3.2 Create OAuthConfig and OAuthProvider trait
  - Create `gate/auth/config.rs` with `OAuthConfig` struct
  - Add fields: client_id, client_secret, auth_url, token_url, redirect_uri, scopes, callback_port
  - Create `gate/providers/mod.rs` with `OAuthProvider` trait
  - Define methods: `provider_id()`, `oauth_config()`, `build_auth_url()`, `exchange_code()`, `refresh_token()`
  - Provide default implementations using config
  - _Requirements: 5.1, 5.2, 5.5, 5.6_

- [ ] 3.3 Implement OAuthFlowState
  - Add `OAuthFlowState` to `gate/auth/mod.rs`
  - Include code_verifier, code_challenge, state fields
  - Implement `new()` generating fresh PKCE and state
  - State can be same as verifier (43 chars) or separate random
  - _Requirements: 3.2_

- [ ] 3.4 Implement OAuthFlow struct
  - Create `gate/auth/mod.rs` with `OAuthFlow<S: TokenStorage>`
  - Store: storage, provider (Box<dyn OAuthProvider>), pending_state (Arc<RwLock>)
  - Implement `new()` constructor
  - Implement `start_authorization()` and `start_authorization_async()`
  - Build authorization URL with PKCE challenge and state
  - Store pending state for later validation
  - _Requirements: 3.1, 3.2_

- [ ] 3.5 Implement code exchange in OAuthFlow
  - Implement `exchange_code()` method
  - Validate state parameter against pending state
  - Return `AuthError::StateMismatch` on validation failure
  - Call provider's `exchange_code()` with code and verifier
  - Save token to storage on success
  - Clear pending state after exchange
  - Implement `exchange_code_with_verifier()` for external verifier
  - Write integration tests with mocked HTTP
  - _Requirements: 3.3, 3.4, 3.5_

- [ ] 3.6 Implement token retrieval and refresh
  - Implement `get_access_token()` in OAuthFlow
  - Check if token is expired or needs refresh
  - Call provider's `refresh_token()` when needed
  - Save refreshed token to storage
  - Return `AuthError::NotAuthenticated` if no token
  - Return `AuthError::InvalidGrant` if refresh fails
  - Implement `get_token()` returning full TokenInfo
  - _Requirements: 3.6, 3.7_

- [ ] 3.7 Implement auth status and logout
  - Implement `is_authenticated()` checking token exists
  - Implement `logout()` removing token and clearing pending state
  - Write unit tests for complete flow
  - _Requirements: 3.8, 3.9_

### Phase 4: Providers

- [ ] 4. Implement provider configurations
- [ ] 4.1 Implement ClaudeProvider
  - Create `gate/providers/claude.rs`
  - Implement `OAuthProvider` trait
  - Default config:
    - client_id: "9d1c250a-e61b-44d9-88ed-5944d1962f5e"
    - auth_url: "https://claude.ai/oauth/authorize"
    - token_url: "https://console.anthropic.com/v1/oauth/token"
    - redirect_uri: "https://console.anthropic.com/oauth/code/callback"
    - scopes: ["org:create_api_key", "user:profile", "user:inference"]
  - Add `code=true` parameter to auth URL
  - Use JSON body for token exchange (not form-encoded)
  - Write unit tests for URL building
  - _Requirements: 5.3_

- [ ] 4.2 Implement GeminiProvider
  - Create `gate/providers/gemini.rs`
  - Implement `OAuthProvider` trait
  - Default config from existing gemini constants
  - Use Google's OAuth endpoints
  - Include cloudcode scopes
  - Write unit tests for URL building
  - _Requirements: 5.4_

- [ ] 4.3 Create convenience constructors
  - Add `OAuthConfig::claude()` and `OAuthConfig::gemini()` factory methods
  - Add `ClaudeProvider::new()` and `GeminiProvider::new()`
  - Allow custom config overrides
  - Export providers from `gate/mod.rs`
  - _Requirements: 5.1, 5.3, 5.4_

### Phase 5: Integration

- [ ] 5. Wire up to application
- [ ] 5.1 Create type aliases for existing usage
  - In `gate/mod.rs`, create type aliases matching old module names if needed
  - E.g., `pub type ClaudeClient = GateClient<ClaudeProvider, FileTokenStorage>`
  - Export `TokenInfo`, `TokenStorage`, `Error`, `AuthError`
  - Ensure all public types are accessible from `gate::*`
  - _Requirements: 6.4, 6.5_

- [ ] 5.2 Update commands.rs for Claude OAuth
  - Locate Claude OAuth commands in `commands.rs`
  - Update imports to use `crate::gate::*`
  - Update function signatures if trait bounds changed
  - Test OAuth flow works end-to-end
  - _Requirements: 6.4_

- [ ] 5.3 Update commands.rs for Gemini OAuth
  - Locate Gemini OAuth commands in `commands.rs`
  - Update imports to use `crate::gate::*`
  - Update function signatures if needed
  - Test OAuth flow works end-to-end
  - _Requirements: 6.4_

- [ ] 5.4 Update LLM providers
  - Check `core/llm/providers/claude.rs` for gate usage
  - Check `core/llm/providers/gemini.rs` for gate usage
  - Update imports to unified gate
  - Verify API calls still work
  - _Requirements: 6.4_

### Phase 6: Cleanup

- [ ] 6. Remove old code and finalize
- [ ] 6.1 Deprecate old modules
  - Add `#[deprecated]` attributes to old module re-exports
  - Keep old modules for one release cycle (optional)
  - Update any remaining references
  - _Requirements: 6.3_

- [ ] 6.2 Delete old gate modules
  - Delete `src-tauri/src/claude/` directory
  - Delete `src-tauri/src/gemini/` directory
  - Remove old module declarations from lib.rs/main.rs
  - Run `cargo check` and fix any remaining references
  - _Requirements: N/A (cleanup)_

- [ ] 6.3 Final testing and documentation
  - Run full test suite: `cargo test`
  - Test OAuth flows manually for both providers
  - Verify backward compatibility with existing token files
  - Update any documentation referencing old modules
  - Measure code reduction (target: 30%+)
  - _Requirements: NFR-1, NFR-2, NFR-3, NFR-4_

---

## Task Dependencies

```
1.1 ──► 1.2 ──► 1.3 ──► 2.1 ──► 2.2
                        │       │
                        │       ├──► 2.3
                        │       │
                        │       ├──► 2.4
                        │       │
                        │       └──► 2.5
                        │
                        └──► 3.1 ──► 3.2 ──► 3.3 ──► 3.4 ──► 3.5 ──► 3.6 ──► 3.7
                                      │
                                      └──► 4.1 ──► 4.3
                                      │
                                      └──► 4.2 ──► 4.3
                                                   │
                                                   └──► 5.1 ──► 5.2 ──► 5.3 ──► 5.4
                                                                                  │
                                                                                  └──► 6.1 ──► 6.2 ──► 6.3
```

---

## Verification Checklist

Before marking implementation complete:

- [ ] All tests pass (`cargo test`)
- [ ] No compiler warnings (`cargo check`)
- [ ] Clippy clean (`cargo clippy`)
- [ ] OAuth flow works for Claude
- [ ] OAuth flow works for Gemini
- [ ] Existing tokens load correctly
- [ ] Token refresh works
- [ ] File permissions are correct (0600)
- [ ] Keyring works when available
- [ ] Code reduction ≥ 30%
- [ ] No regressions in LLM provider functionality
