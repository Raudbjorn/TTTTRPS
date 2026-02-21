# Embedded Meilisearch Integration with RAG Pipeline

## Executive Summary

Integrate `meilisearch-lib` as an embedded search engine with full RAG (Retrieval-Augmented Generation) capabilities for TTRPG rulebook queries. This eliminates the need for an external Meilisearch process while providing AI-powered question answering over indexed documents.

### Current State

```
TTRPS App
    │
    ├── meilisearch-sdk (HTTP client)
    │       │
    │       └── HTTP ──▶ [meilisearch binary :7700]
    │                        └── spawned/managed by SidecarManager
    │
    └── Custom LLM integration (no RAG pipeline)
```

**Pain Points:**
- External process management (download, spawn, health-check, restart)
- Platform-specific sidecar binaries
- No integrated RAG pipeline for rulebook Q&A
- Manual context building for LLM queries

### Target State

```
TTRPS App
    │
    └── meilisearch-lib (embedded)
            │
            ├── Index/Document Operations
            │   └── Store TTRPG rulebook chunks
            │
            ├── Hybrid Search (Keyword + Semantic)
            │   └── BM25 + vector embeddings via milli
            │
            └── RAG Pipeline ← KEY FEATURE
                ├── chat_completion() / chat_completion_stream()
                ├── Automatic context retrieval
                ├── Liquid template formatting
                ├── Multi-provider LLM support
                └── Source citations
```

**Benefits:**
- Zero external processes
- Built-in RAG pipeline for rulebook Q&A
- Hybrid search (keyword + semantic) for context retrieval
- Multi-LLM support (Claude, GPT-4, Mistral, Ollama)
- Source citations in responses
- Streaming support

---

## Key Feature: RAG for TTRPG

```rust
// Configure RAG for your rulebook indexes
let chat_config = ChatConfig {
    source: ChatSource::Anthropic,
    api_key: env::var("ANTHROPIC_API_KEY")?,
    model: "claude-sonnet-4-20250514".to_string(),
    prompts: ChatPrompts {
        system: Some("You are a TTRPG rules expert. Answer using only the provided context.".into()),
        ..Default::default()
    },
    index_configs: HashMap::from([
        ("ttrpg_rules".to_string(), ChatIndexConfig {
            description: "D&D 5e rules and mechanics".to_string(),
            template: Some("Source: {{doc.source}} (p{{doc.page}})\n{{doc.content}}".to_string()),
            search_params: Some(ChatSearchParams {
                limit: Some(8),
                semantic_ratio: Some(0.7),  // 70% semantic, 30% keyword
                ..Default::default()
            }),
            ..Default::default()
        }),
    ]),
    ..Default::default()
};

meili.set_chat_config(Some(chat_config));

// Ask questions about your rulebooks
let response = meili.chat_completion(ChatRequest {
    messages: vec![Message::user("How does flanking work?")],
    index_uid: "ttrpg_rules".to_string(),
    stream: false,
}).await?;

println!("{}", response.content);
// "Flanking is an optional rule in the DMG (p.251)..."
println!("Sources: {:?}", response.sources);
// ["dmg-page-251", "phb-combat-chapter"]
```

---

## Specification Documents

| Document | Purpose |
|----------|---------|
| [Requirements.md](./Requirements.md) | User stories, functional requirements |
| [Design.md](./Design.md) | Technical architecture, component design |
| [Tasks.md](./Tasks.md) | Implementation tasks (~31 hours) |
| [API-Surface-Analysis.md](./API-Surface-Analysis.md) | SDK vs lib API comparison |

---

## Architecture Decision

**Decision**: Use meilisearch-lib as-is with full RAG pipeline

**Rationale**:
- RAG pipeline is the primary value for TTRPG rulebook queries
- Hybrid search (keyword + semantic) already integrated
- Multi-LLM provider support built-in
- Source citations included
- Would take weeks to reimplement what already exists

**What we considered but rejected**:
- Stripping chat module → Loses RAG functionality
- SurrealDB for vectors → Requires custom RAG implementation
- Custom RRF fusion → Already handled by meilisearch-lib

---

## Implementation Phases

1. **Dependency Setup** (~2 hours)
   - Add meilisearch-lib to workspace
   - Configure feature flags

2. **Sidecar Removal** (~4 hours)
   - Remove SidecarManager
   - Remove process spawning code
   - Update health checks

3. **Client Migration** (~12 hours)
   - Replace SearchClient with MeilisearchLib
   - Update all Tauri commands
   - Migrate settings/embedder configuration

4. **RAG Configuration** (~5 hours)
   - Configure ChatConfig for TTRPG indexes
   - Set up Liquid templates for document formatting
   - Configure hybrid search parameters

5. **Testing & Polish** (~8 hours)
   - Integration tests
   - Update frontend for streaming responses
   - Documentation

**Total Estimate**: ~31 hours (~4 working days)

---

## LLM Providers Supported

| Provider | Model Examples |
|----------|----------------|
| **Anthropic** | claude-sonnet-4-20250514, claude-3-haiku |
| **OpenAI** | gpt-4o, gpt-4-turbo, gpt-3.5-turbo |
| **Azure OpenAI** | Custom deployments |
| **Mistral** | mistral-large, mistral-medium |
| **vLLM** | Self-hosted models |

---

## Success Criteria

- [ ] App starts without spawning external processes
- [ ] RAG queries return relevant answers with source citations
- [ ] Hybrid search combines keyword + semantic results
- [ ] Streaming responses work for chat interface
- [ ] All existing search functionality preserved
- [ ] Works on Linux, Windows, macOS

---

## Related Documentation

- [meilisearch-lib source](/mnt/mrgr/dev/meili-dev/crates/meilisearch-lib/)
- [Previous meilisearch integration](../done/meilisearch-integration/)
- [Embeddings implementation](../done/embeddings/)
