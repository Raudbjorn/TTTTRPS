# 16 — Audit, Analytics & Usage Tracking

**Gap addressed:** #13 (MISSING — not mentioned)

## Audit Logger (`core/audit.rs`)

In-memory `RwLock<VecDeque<AuditEvent>>`, capacity 10000, rotates oldest on overflow. Optionally mirrors to `tracing`. No persistent storage wired yet.

### Event Types

| Category | Events |
|----------|--------|
| Auth | ApiKeyAdded, ApiKeyRemoved, ApiKeyAccessed (all with `provider`) |
| Document | DocumentIngested (path, doc_type), DocumentDeleted (doc_id), DocumentSearched (query) |
| Campaign | CampaignCreated (id, name), CampaignDeleted (id), CampaignExported (id), CampaignImported (name) |
| Session | SessionStarted (session_id, campaign_id), SessionEnded (session_id) |
| LLM | LlmRequest (provider, model, tokens: u32), LlmError (provider, error) |
| Settings | SettingsChanged (setting, old_value, new_value) |
| Security | ValidationFailed (input_type, reason), RateLimitHit (endpoint) |
| System | ApplicationStarted, ApplicationShutdown, BackupCreated (path), BackupRestored (path) |
| Custom | category, action, details |

### Severity Levels (ordered)

`Info` < `Warning` < `Security` < `Critical`

### Query API

- `query(AuditQuery)` — time range, severity filter, event type filter
- `get_recent(count)`, `get_by_severity`, `get_security_events` (last 24h)
- `export_json`, `cleanup(days)`, `count()`

## Search Analytics (`core/search_analytics.rs`)

Two implementations:

### In-Memory (`SearchAnalytics`)
- `records: Vec<SearchRecord>` (max 100000, drains 10000 on overflow)
- `selections: Vec<ResultSelection>` (max 50000)
- `query_stats: HashMap<String, QueryStats>`
- Atomic counters: cache_hits, cache_misses, cache_time_saved_ms
- Rolling average for uncached query time (90% old + 10% new)

### Persistent (`DbSearchAnalytics`)
- SQLite-backed via `Database` trait
- Methods: record_search, record_search_selection, get_search_analytics_summary, get_popular_queries, get_cache_stats, get_trending_queries, get_zero_result_queries, get_click_distribution, cleanup_search_analytics

### Key Types

**SearchRecord:** id, query, result_count, clicked, clicked_index, execution_time_ms, search_type, from_cache, source_filter, campaign_id, timestamp

**QueryStats:** count, clicks, avg_results, avg_time_ms, last_searched, click_positions: HashMap<usize, u32>

**CacheStats:** hits, misses, hit_rate, avg_time_saved_ms, total_time_saved_ms, top_cached_queries

**AnalyticsSummary:** total_searches, zero_result_searches, click_through_rate, avg_results_per_search, avg_execution_time_ms, top_queries, failed_queries, cache_stats, by_search_type, period_start/end

## Usage / Cost Tracking (`core/usage/`)

### UsageTracker
- `records: Vec<UsageRecord>` (max 100000, drains 10000)
- `session_usage: SessionUsage`
- `budget_limits: HashMap<BudgetPeriodType, BudgetLimit>`

### Key Types

**UsageRecord:** id, timestamp, provider, model, input_tokens, output_tokens, cached_tokens, cost_usd, context

**BudgetPeriodType:** Hourly, Daily, Weekly, Monthly, Total

**BudgetLimit:** limit_usd (default 50.0), period, warning_threshold (0.8), critical_threshold (0.95), block_on_limit (false)

**BudgetAlertLevel:** Normal, Warning, Critical, Exceeded

**CostBreakdown:** total_cost_usd, by_provider (HashMap → ProviderCostDetails), by_model (HashMap → ModelCostDetails), period_start/end

**Note:** A second `UsageRecord` exists in `database/models/chat.rs` with different fields (`estimated_cost_usd` vs `cost_usd`, no `cached_tokens`). These are not unified.

## TUI Requirements

1. **Audit log viewer** — scrollable, filterable by severity/category/time
2. **Security events panel** — last 24h security events highlighted
3. **Search analytics dashboard** — 24h search count, cache hit %, top 5 queries, zero-result queries
4. **Cost dashboard** — session cost, provider breakdown, budget status with alert level
5. **Budget configuration** — set limits per period with warning/critical thresholds
6. **Cache stats** — hit rate, time saved, most cached queries
