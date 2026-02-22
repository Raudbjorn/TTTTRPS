# 17 — Configuration & Settings

**Gap addressed:** #19 (MISSING — no segment)

## Overview

Configuration is split across multiple structs with different storage strategies. Only `AppConfig` persists to disk (TOML). All others are code-constructed or derive from runtime state.

## AppConfig (`src/config.rs`) — TOML file

**Path:** `~/.config/ttttrps/config.toml`

```toml
[tui]
tick_rate_ms = 50       # Event loop tick rate
mouse_enabled = false   # Mouse support toggle
theme = "default"       # Color theme name

[data]
# data_dir = "/custom/path"  # Override XDG default
# Default: ~/.local/share/ttrpg-assistant
```

## Runtime Config Structs

### StorageConfig (`core/storage/surrealdb.rs`)
| Field | Default | Notes |
|-------|---------|-------|
| namespace | `"ttrpg"` | SurrealDB namespace |
| database | `"main"` | SurrealDB database name |
| default_vector_dimensions | `768` | HNSW index dimensions |

### RouterConfig (`core/llm/router/config.rs`)
| Field | Default | Notes |
|-------|---------|-------|
| request_timeout | 120s | Per-request timeout |
| enable_fallback | true | Try next provider on failure |
| health_check_interval | 60s | Provider health probe interval |
| routing_strategy | Priority | Priority / CostOptimized / LatencyOptimized / RoundRobin / Random |
| max_retries | 1 | Retry count per provider |
| monthly_budget | None | Optional budget cap |
| daily_budget | None | Optional budget cap |
| stream_chunk_timeout | 30s | Timeout between stream chunks |

### EmbeddingConfig (`core/search/embeddings.rs`)
| Field | Default | Notes |
|-------|---------|-------|
| provider | `"ollama"` | Embedding provider |
| model | `"nomic-embed-text"` | Model name |
| endpoint | `"http://localhost:11434"` | Provider endpoint |
| api_key | None | Optional auth |
| dimensions | 768 | Vector dimensions |
| batch_size | 32 | Batch embedding size |

### Other Config Structs

| Struct | Location | Purpose |
|--------|----------|---------|
| PreprocessConfig | `core/preprocess/config.rs` | Typo correction + synonym expansion settings |
| TypoConfig | `core/preprocess/config.rs` | SymSpell parameters |
| SynonymConfig | `core/preprocess/config.rs` | Synonym expansion rules |
| HybridSearchConfig | `core/storage/search.rs` | Semantic/keyword weights, score normalization |
| RagConfig | `core/storage/rag.rs` + `core/rag/config.rs` | TTRPG-specific RAG settings (two definitions) |
| RelationshipManagerConfig | `core/campaign/relationships.rs` | auto_create_inverse, max relationships |
| QueueConfig | `core/voice/queue/types.rs` | Max concurrent jobs, batch size, timeout |
| TranscriptionConfig | `core/transcription.rs` | Speech-to-text settings |

## Credential Storage

API keys stored in system keyring via `keyring` crate (not in config files):
- Claude (Anthropic)
- Gemini (Google)
- OpenAI
- Ollama (no key needed)
- Mistral, Azure OpenAI, vLLM

OAuth tokens: `FileTokenStorage` at `~/.local/share/ttrpg-assistant/tokens.json` (0600 perms)

## TUI Requirements

1. **Config editor** — edit AppConfig TOML fields (tick rate, mouse, theme)
2. **LLM provider settings** — routing strategy, timeouts, budget caps
3. **Embedding config** — provider/model/endpoint selection
4. **Credential manager** — add/remove API keys (masked display)
5. **Data directory** — show path, size, option to change
6. **Search tuning** — hybrid search weights, score normalization method
