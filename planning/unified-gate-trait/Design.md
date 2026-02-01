# Design: Unified Gate Trait

## Overview

This design document describes the architecture for unifying `claude` and `gemini` into a shared trait-based system. The core insight is that both implementations share ~70% of their code: OAuth 2.0 + PKCE flows, token storage, error handling, and client lifecycle. Provider-specific differences (API endpoints, token formats, request/response conversion) are isolated into trait implementations.

### Design Goals

- **Maximize code reuse**: Single implementations for OAuth, PKCE, storage, and error handling
- **Provider isolation**: Clean separation between shared infrastructure and provider-specific logic
- **Backward compatibility**: Existing tokens and API calls continue to work
- **Extensibility**: Adding new providers requires minimal new code
- **Type safety**: Invalid states are unrepresentable; the compiler catches misuse

### Key Design Decisions

- **Trait-based polymorphism over generics**: Use `dyn TokenStorage` and `dyn OAuthProvider` for runtime flexibility in Tauri commands
- **Unified module location**: New `gate` module at `src-tauri/src/gate/` with provider subdirectories
- **Composition over inheritance**: Providers compose shared components rather than inheriting
- **Feature flags for optional backends**: `keyring` feature gates system keyring support

## Architecture

### System Overview

```
gate/
├── mod.rs              # Re-exports, GateClient trait
├── error.rs            # Unified Error/AuthError types
├── token.rs            # TokenInfo model
├── storage/
│   ├── mod.rs          # TokenStorage trait
│   ├── file.rs         # FileTokenStorage
│   ├── memory.rs       # MemoryTokenStorage
│   ├── keyring.rs      # KeyringTokenStorage (feature-gated)
│   └── callback.rs     # CallbackStorage
├── auth/
│   ├── mod.rs          # OAuthFlow struct
│   ├── pkce.rs         # PKCE generation/verification
│   └── config.rs       # OAuthConfig types
├── providers/
│   ├── mod.rs          # OAuthProvider trait
│   ├── claude.rs       # Claude/Anthropic provider
│   └── gemini.rs       # Gemini/Google provider
└── client.rs           # Generic GateClient implementation
```

### Component Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        GateClient<P, S>                         │
│   (Generic client parameterized by Provider and Storage)        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────────┐    ┌──────────────────┐                   │
│  │   OAuthFlow<S>  │    │  OAuthProvider P │                   │
│  │  (Shared auth)  │    │  (Provider-spec) │                   │
│  └────────┬────────┘    └────────┬─────────┘                   │
│           │                      │                              │
│           ▼                      ▼                              │
│  ┌─────────────────┐    ┌──────────────────┐                   │
│  │  TokenStorage S │    │   OAuthConfig    │                   │
│  │ (Pluggable)     │    │ (Provider URLs)  │                   │
│  └─────────────────┘    └──────────────────┘                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
           │                       │
           ▼                       ▼
┌──────────────────┐    ┌──────────────────────┐
│  Storage Impls   │    │   Provider Impls     │
├──────────────────┤    ├──────────────────────┤
│ • FileStorage    │    │ • ClaudeProvider     │
│ • MemoryStorage  │    │ • GeminiProvider     │
│ • KeyringStorage │    │ • (Future providers) │
│ • CallbackStorage│    └──────────────────────┘
└──────────────────┘
```

### Technology Stack

| Layer | Technology | Rationale |
|-------|------------|-----------|
| Async runtime | tokio | Already used throughout the application |
| HTTP client | reqwest | Existing dependency, well-tested |
| Serialization | serde + serde_json | Standard Rust serialization |
| Traits | async_trait | Required for async trait methods |
| Error handling | thiserror | Derive-based error types |
| Time handling | chrono | Already used for timestamp operations |
| Crypto | sha2 + base64 | PKCE challenge generation |

## Components and Interfaces

### TokenStorage Trait

**Purpose**: Abstract interface for persisting OAuth tokens to various backends.

**Responsibilities**:
- Load tokens from storage
- Save tokens to storage
- Remove tokens from storage
- Check token existence (with efficient default)

**Interface**:
```rust
#[async_trait]
pub trait TokenStorage: Send + Sync {
    /// Load token for a provider, returns None if not found
    async fn load(&self, provider: &str) -> Result<Option<TokenInfo>>;

    /// Save token for a provider
    async fn save(&self, provider: &str, token: &TokenInfo) -> Result<()>;

    /// Remove token for a provider
    async fn remove(&self, provider: &str) -> Result<()>;

    /// Check if token exists (default: calls load)
    async fn exists(&self, provider: &str) -> Result<bool> {
        Ok(self.load(provider).await?.is_some())
    }

    /// Storage backend name for debugging
    fn name(&self) -> &str { "unknown" }
}
```

**Implementation Notes**:
- Provider parameter allows single storage for multiple providers
- Blanket impls for `Arc<T>` and `Box<T>` enable flexible ownership
- File storage uses JSON with `{ "provider_name": TokenInfo }` schema

### OAuthProvider Trait

**Purpose**: Encapsulate provider-specific OAuth configuration and behavior.

**Responsibilities**:
- Provide OAuth endpoint URLs
- Provide default scopes
- Customize token exchange/refresh if needed

**Interface**:
```rust
pub trait OAuthProvider: Send + Sync {
    /// Provider identifier (e.g., "anthropic", "google")
    fn provider_id(&self) -> &str;

    /// OAuth configuration for this provider
    fn oauth_config(&self) -> &OAuthConfig;

    /// Build authorization URL (default implementation uses config)
    fn build_auth_url(&self, pkce: &Pkce, state: &str) -> String {
        // Default implementation using oauth_config
    }

    /// Exchange code for tokens (can override for custom behavior)
    async fn exchange_code(
        &self,
        code: &str,
        verifier: &str,
    ) -> Result<TokenInfo> {
        // Default implementation using oauth_config
    }

    /// Refresh access token
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenInfo> {
        // Default implementation using oauth_config
    }
}
```

### OAuthConfig

**Purpose**: Hold OAuth endpoint configuration for a provider.

**Interface**:
```rust
#[derive(Debug, Clone)]
pub struct OAuthConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub auth_url: String,
    pub token_url: String,
    pub redirect_uri: String,
    pub scopes: Vec<String>,
    pub callback_port: Option<u16>,
}

impl OAuthConfig {
    pub fn claude() -> Self { /* Anthropic defaults */ }
    pub fn gemini() -> Self { /* Google defaults */ }
}
```

### OAuthFlow<S>

**Purpose**: Orchestrate the complete OAuth lifecycle using a storage backend.

**Responsibilities**:
- Generate PKCE challenges
- Build authorization URLs
- Exchange codes for tokens
- Refresh expired tokens
- Manage pending flow state

**Interface**:
```rust
pub struct OAuthFlow<S: TokenStorage> {
    storage: S,
    provider: Box<dyn OAuthProvider>,
    pending_state: Arc<RwLock<Option<OAuthFlowState>>>,
}

impl<S: TokenStorage> OAuthFlow<S> {
    pub fn new(storage: S, provider: impl OAuthProvider + 'static) -> Self;

    pub fn start_authorization(&self) -> Result<(String, OAuthFlowState)>;
    pub async fn start_authorization_async(&self) -> Result<(String, OAuthFlowState)>;

    pub async fn exchange_code(&self, code: &str, state: Option<&str>) -> Result<TokenInfo>;
    pub async fn exchange_code_with_verifier(
        &self, code: &str, verifier: &str,
        expected_state: Option<&str>, received_state: Option<&str>
    ) -> Result<TokenInfo>;

    pub async fn get_access_token(&self) -> Result<String>;
    pub async fn get_token(&self) -> Result<TokenInfo>;

    pub async fn is_authenticated(&self) -> Result<bool>;
    pub async fn logout(&self) -> Result<()>;

    pub fn storage(&self) -> &S;
    pub fn provider(&self) -> &dyn OAuthProvider;
}
```

### Pkce

**Purpose**: Generate and verify PKCE challenge/verifier pairs.

**Interface**:
```rust
#[derive(Debug, Clone)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
    pub method: &'static str,  // Always "S256"
}

impl Pkce {
    pub fn generate() -> Self;
    pub fn verify(verifier: &str, challenge: &str) -> bool;
}
```

### GateClient

**Purpose**: High-level client combining auth flow with provider-specific API calls.

**Interface**:
```rust
pub struct GateClient<P: OAuthProvider, S: TokenStorage> {
    flow: OAuthFlow<S>,
    http_client: reqwest::Client,
    _provider: PhantomData<P>,
}

impl<P: OAuthProvider, S: TokenStorage> GateClient<P, S> {
    pub fn new(storage: S, provider: P) -> Self;

    // Auth delegation
    pub fn start_authorization(&self) -> Result<(String, OAuthFlowState)>;
    pub async fn exchange_code(&self, code: &str, state: Option<&str>) -> Result<TokenInfo>;
    pub async fn get_access_token(&self) -> Result<String>;
    pub async fn is_authenticated(&self) -> Result<bool>;
    pub async fn logout(&self) -> Result<()>;

    // HTTP with auth header
    pub async fn request(&self, method: Method, url: &str) -> RequestBuilder;
}
```

## Data Models

### TokenInfo

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    /// Token type (e.g., "oauth", "bearer")
    #[serde(rename = "type", default = "default_token_type")]
    pub token_type: String,

    /// OAuth access token
    #[serde(alias = "access")]
    pub access_token: String,

    /// OAuth refresh token
    #[serde(alias = "refresh")]
    pub refresh_token: String,

    /// Unix timestamp when token expires
    #[serde(alias = "expires")]
    pub expires_at: i64,
}

impl TokenInfo {
    pub fn new(access_token: String, refresh_token: String, expires_in: i64) -> Self;
    pub fn is_expired(&self) -> bool;
    pub fn needs_refresh(&self) -> bool;  // Within 5 minutes of expiry
    pub fn time_until_expiry(&self) -> i64;
}
```

**Validation Rules**:
- `access_token` must not be empty
- `refresh_token` must not be empty
- `expires_at` must be positive Unix timestamp

**Backward Compatibility**:
- `#[serde(alias = "access")]` supports claude's `access` field
- `#[serde(alias = "expires")]` supports claude's `expires` field
- Default `token_type` handles missing field

### OAuthFlowState

```rust
#[derive(Debug, Clone)]
pub struct OAuthFlowState {
    pub code_verifier: String,
    pub code_challenge: String,
    pub state: String,
}

impl OAuthFlowState {
    pub fn new() -> Self;  // Generates fresh PKCE and state
}
```

## Error Handling

### Error Enum

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("API error ({status}): {message}")]
    Api {
        status: u16,
        message: String,
        retry_after: Option<Duration>,
    },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn config(msg: impl Into<String>) -> Self;
    pub fn storage(msg: impl Into<String>) -> Self;
    pub fn api(status: u16, message: impl Into<String>, retry_after: Option<Duration>) -> Self;

    pub fn is_rate_limit(&self) -> bool;
    pub fn is_auth_error(&self) -> bool;
    pub fn requires_reauth(&self) -> bool;
    pub fn retry_after(&self) -> Option<Duration>;
}
```

### AuthError Enum

```rust
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Not authenticated - please complete OAuth flow")]
    NotAuthenticated,

    #[error("Token expired - please re-authenticate")]
    TokenExpired,

    #[error("Invalid grant - refresh token is invalid")]
    InvalidGrant,

    #[error("OAuth state mismatch - possible CSRF attack")]
    StateMismatch,

    #[error("OAuth flow cancelled")]
    Cancelled,

    #[error("PKCE verification failed")]
    PkceVerificationFailed,
}
```

| Category | Error Variant | User Action |
|----------|--------------|-------------|
| Auth | NotAuthenticated | Start OAuth flow |
| Auth | TokenExpired | Re-authenticate |
| Auth | InvalidGrant | Re-authenticate |
| Auth | StateMismatch | Retry OAuth flow |
| API | 401 | Re-authenticate |
| API | 429 | Wait and retry |
| API | 5xx | Retry later |
| Storage | * | Check permissions/config |

## Testing Strategy

### Unit Testing

**Target**: 80% coverage of core logic

- `token.rs`: Test expiration logic, serialization, backward compatibility
- `pkce.rs`: Test generation, verification, challenge algorithm
- `storage/*.rs`: Test each backend with mock filesystem/keyring
- `auth/mod.rs`: Test flow state management, URL building
- `error.rs`: Test error conversions, helper methods

### Integration Testing

- OAuth flow with mock HTTP responses (wiremock)
- Storage roundtrip tests (save → load → verify)
- Token refresh with mocked endpoints
- State validation (CSRF protection)

### Backward Compatibility Testing

- Load existing claude token files
- Load existing gemini token files
- Verify API call compatibility with existing provider code

## Migration Strategy

### Phase 1: Create Unified Module
- Build new `gate/` module alongside existing modules
- No changes to existing code

### Phase 2: Wire Up Providers
- Create `ClaudeProvider` using unified traits
- Create `GeminiProvider` using unified traits
- Existing code continues to use old modules

### Phase 3: Migrate Consumers
- Update `commands.rs` to use new gate types
- Update `core/llm/providers/` to use new clients
- Keep old modules as deprecated

### Phase 4: Remove Old Code
- Delete `claude/` and `gemini/` directories
- Remove deprecated re-exports
- Final cleanup
