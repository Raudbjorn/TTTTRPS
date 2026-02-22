# 05 — OAuth & Credentials

**Gap addressed:** #15 (OAuth underspecified)

## Provider-Specific Auth Flows

### Claude (Anthropic) — `oauth/claude/`

| Aspect | Detail |
|--------|--------|
| Flow | PKCE-only (no client secret) |
| Encoding | JSON request bodies |
| Redirect | Remote OAuth redirect URL |
| Token format | Composite: `refresh_token|project_id|managed_project_id` |
| Refresh | Automatic with stored refresh token |

### Gemini (Google) — `oauth/gemini/`

| Aspect | Detail |
|--------|--------|
| Flow | PKCE + client secret |
| Encoding | Form-encoded bodies |
| Redirect | `http://localhost:8888` (local callback server) |
| Extra | Project discovery and management (create/list GCP projects) |
| Token format | Composite with project metadata |

### Copilot (GitHub) — `oauth/copilot/`

| Aspect | Detail |
|--------|--------|
| Flow | RFC 8628 Device Code (polling-based) |
| Redirect | None — user visits URL and enters code |
| Polling | Interval-based token polling with backoff |
| Display | Device code + verification URL shown to user |

## Token Storage Backends

| Backend | Use Case | Location |
|---------|----------|----------|
| `FileTokenStorage` | Production | `~/.local/share/ttrpg-assistant/tokens.json` (0600 perms) |
| `KeyringTokenStorage` | System keyring | OS-native (macOS Keychain, Linux Secret Service, Windows Credential Manager) |
| `MemoryTokenStorage` | Testing | In-memory HashMap |
| `CallbackStorage` | Integration tests | Custom closure-based |

API keys (non-OAuth) stored in system keyring via `keyring` crate.

## Error Recovery

- `requires_reauth()` → full OAuth flow needed (expired/revoked)
- `is_recoverable()` → safe to retry (network transient)
- `is_rate_limit()` → HTTP 429 with `retry_after()` duration
- `is_auth_error()` → 401/403 requiring credential refresh

## TUI Requirements

1. **Provider setup wizard** — per-provider auth flow:
   - Claude: Display auth URL → open browser → wait for callback
   - Gemini: Display auth URL → local callback server on :8888 → project selection
   - Copilot: Display device code + URL → poll until authorized
2. **Credential status** — show per-provider: authenticated (yes/no), token expiry, last refresh
3. **API key management** — for non-OAuth providers (OpenAI, Ollama, etc.): secure input, keyring storage
4. **Token refresh** — manual trigger for expired tokens
5. **Error display** — auth failures with recovery guidance (re-auth, check network, etc.)
