You’ve bumped into the classic “semantic match but attribute mismatch” problem: the embedding model thinks *“sand beach”* is close enough, while the user really meant the **black** attribute as a hard constraint.

In practice, you cope by **splitting “meaning” from “filters”**:

* **Embeddings**: capture broad semantic intent (beach vibes).
* **Structured attributes**: enforce crisp constraints (black sand, not white).
* **Re-rank / veto layer**: if attributes conflict, down-rank or drop, even if the semantic score is high.

Below are concrete Rust patterns for **pre-processing**, **retrieval**, **post-processing**, and **embedding/model tuning** guidance.

---

## 1) Pre-processing: extract “hard” attributes into metadata

### A. Represent your items with searchable metadata

If you have images, run a vision model / captioner / tagger offline and store tags like:

* `sand_color: black | white | unknown`
* `beach: true`
* plus confidence scores

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SandColor {
    Black,
    White,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ImageMeta {
    pub sand_color: SandColor,
    pub sand_color_conf: f32, // 0..1
    pub tags: Vec<String>,    // e.g. ["beach", "ocean", "volcano"]
}

#[derive(Debug, Clone)]
pub struct Doc {
    pub id: String,
    pub text: String,          // caption / OCR / transcript / etc.
    pub embedding: Vec<f32>,   // your vector
    pub meta: ImageMeta,
}
```

### B. Parse the query for attribute constraints (cheap + effective)

Don’t overthink it: a small rule-based extractor gets you 80% quickly, and you can evolve it later.

```rust
#[derive(Debug, Clone)]
pub struct QueryConstraints {
    pub sand_color: Option<SandColor>,
    pub sand_color_min_conf: f32,
}

pub fn extract_constraints(query: &str) -> QueryConstraints {
    let q = query.to_lowercase();
    let sand_color = if q.contains("black sand") || q.contains("black-sand") {
        Some(SandColor::Black)
    } else if q.contains("white sand") || q.contains("white-sand") {
        Some(SandColor::White)
    } else {
        None
    };

    QueryConstraints {
        sand_color,
        sand_color_min_conf: 0.60, // tune this per your pipeline quality
    }
}
```

This is the “hybrid” in hybrid search: **structured extraction + semantic retrieval**.

---

## 2) Retrieval: use a filter gate before scoring (pre-filter)

If the user asks for black sand beach, do **not** even consider items confidently tagged as white sand.

```rust
pub fn passes_prefilter(doc: &Doc, c: &QueryConstraints) -> bool {
    match c.sand_color {
        None => true,
        Some(ref wanted) => {
            // If we confidently know it's the opposite color, exclude it.
            let conf = doc.meta.sand_color_conf;
            let color = &doc.meta.sand_color;

            if conf >= c.sand_color_min_conf {
                color == wanted
            } else {
                // low confidence -> keep; let post-processing decide
                true
            }
        }
    }
}
```

### Vector search + filter

If you’re using a vector DB (Qdrant, Milvus, Elasticsearch kNN), push this filter into the query.
But even in-memory, do:

1. prefilter candidates
2. compute cosine similarity
3. take top K

Cosine helper:

```rust
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut na = 0.0;
    let mut nb = 0.0;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 { return 0.0; }
    dot / (na.sqrt() * nb.sqrt())
}
```

---

## 3) Post-processing: soft penalties + hard veto (re-rank)

Even after pre-filtering, you may still have “unknown” or ambiguous cases. Do a re-rank where attribute matches can dominate popularity.

### A. Combine signals (semantic + popularity + attribute agreement)

Key trick: treat attribute mismatch as a **veto** or a **huge penalty**.

```rust
#[derive(Debug, Clone)]
pub struct Scored<'a> {
    pub doc: &'a Doc,
    pub semantic: f32,
    pub popularity: f32, // e.g. 0..1
    pub final_score: f32,
    pub excluded: bool,
}

pub fn rerank<'a>(
    docs: impl IntoIterator<Item = (&'a Doc, f32 /*semantic*/, f32 /*popularity*/)> ,
    c: &QueryConstraints,
) -> Vec<Scored<'a>> {
    let mut out = Vec::new();

    for (doc, semantic, popularity) in docs {
        let mut excluded = false;
        let mut score = 0.0;

        // base mixture (tune weights)
        score += 0.75 * semantic + 0.25 * popularity;

        // attribute logic
        if let Some(ref wanted) = c.sand_color {
            let conf = doc.meta.sand_color_conf;
            let color = &doc.meta.sand_color;

            if conf >= c.sand_color_min_conf && color != wanted {
                // hard veto
                excluded = true;
                score = f32::NEG_INFINITY;
            } else if conf >= c.sand_color_min_conf && color == wanted {
                // boost
                score += 0.25;
            } else {
                // unknown/low-conf: mild penalty so confident matches rise
                score -= 0.10;
            }
        }

        out.push(Scored { doc, semantic, popularity, final_score: score, excluded });
    }

    out.sort_by(|a, b| b.final_score.partial_cmp(&a.final_score).unwrap_or(std::cmp::Ordering::Equal));
    out
}
```

This gives you the behavior you want:

* white-sand result can be #1 semantically and still be kicked out
* black-sand result with decent semantic match will float up

---

## 4) Make the query embedding “attribute aware” (without tuning the model)

You usually **don’t** tune the base embedding model for this. You instead:

* extract constraints and enforce them structurally
* optionally expand the text you embed to emphasize the attribute

Example: rewrite query text before embedding:

```rust
pub fn canonicalize_query_for_embedding(query: &str, c: &QueryConstraints) -> String {
    let mut q = query.to_string();
    if let Some(SandColor::Black) = c.sand_color {
        // reinforce: include synonyms and “not white” as a biasing hint
        q.push_str(" | black volcanic sand, basalt sand, not white sand");
    }
    if let Some(SandColor::White) = c.sand_color {
        q.push_str(" | white sand, pale sand, not black volcanic sand");
    }
    q
}
```

This helps the embedding retrieval pull in the right neighborhood, but it’s still not a guarantee. The guarantee comes from your veto/filters.

---

## 5) If you *do* tune: what to tune (and what not to)

Tuning can help, but the highest ROI is usually **not** “teach the embedding model colors.” Colors are exactly the kind of crisp attribute that embeddings blur.

If you want to tune anyway, tune for **ranking behavior**:

* Create training pairs/triplets:

  * Query: “black sand beach”
  * Positive: items with black sand
  * Hard negative: visually similar beaches with white sand
* Use contrastive learning (triplet loss / InfoNCE style)

But even then: you *still* keep the filter/veto layer because:

* vision tagging can be wrong
* captions can lie
* users use tricky phrasing (“jet-black sand”, “dark volcanic shore”)

So tuning is an enhancement, not your safety rail.

---

## 6) Practical architecture that behaves nicely

A robust pipeline looks like this:

1. **Offline ingestion**

* generate embedding from text/caption/OCR
* run taggers/classifiers → structured meta + confidences

2. **Query-time**

* extract constraints from query
* optionally canonicalize query text for embedding
* vector search (top N)
* prefilter + rerank using constraints
* (optional) final cross-encoder / LLM reranker for top 20

That last LLM/cross-encoder step is how big search engines do “sanity checks,” but your simple veto layer already catches the worst mismatch.

---

### Small thing I fixed/improved while answering

I treated “black sand” as an explicit **constraint signal** rather than “just more words for the embedding,” because embeddings will always sometimes ignore attributes when the surrounding semantics are strong.

If you tell me what you’re using for storage/search (in-memory, Qdrant, Elasticsearch, Postgres+pgvector) and how you’re producing image tags (captioner? CLIP labels? custom classifier?), I can tailor the Rust examples to that exact stack and show how to express the filter in the query itself.

Yep — there are a bunch of repos that tackle this exact failure mode (high semantic score, wrong *attribute*) via **metadata filtering**, **hybrid search**, and/or **reranking**. Here are solid, inspectable examples (with the “what to steal” called out).

### Repos focused on the *right* mitigation patterns

**Qdrant (payload filters = hard constraints)**

* **qdrant/qdrant** — core DB supports attaching JSON “payload” to vectors and filtering queries using those payload values (exactly how you’d enforce `sand_color=black`). ([GitHub][1])
* **qdrant/examples** — grab-and-go demos showing hybrid-ish search patterns and payload filtering usage in practice. ([GitHub][2])

**Elasticsearch (kNN + filters + semantic reranking)**

* **elastic/elasticsearch-labs** — notebook: *semantic reranking* (take top-N candidates, rerank with a stronger model). This is the canonical “post-processing fix” when vector similarity is too mushy. ([GitHub][3])

**Hybrid search + reranking (general RAG patterns you can port)**

* **edumunozsala/langchain-rag-techniques** — notebook: hybrid/fusion + reranking ideas (even if you’re not using LangChain, the heuristics translate). ([GitHub][4])
* **yuniko-software/bge-m3-qdrant-sample** — hybrid search + reranking with Qdrant, using a model that produces multiple vector types; useful if you’re doing “semantic + lexical + something else.” ([GitHub][5])
* **shanojpillai/qdrant-rag-pro** — “production-ish” RAG that explicitly calls out hybrid search + metadata filtering. ([GitHub][6])
* **chatterjeesaurabh/Contextual-RAG-System-with-Hybrid-Search-and-Reranking** — straightforward hybrid (BM25 + vector) + rerank pipeline. ([GitHub][7])

### Repos on the *multimodal / attribute* side (more researchy, but relevant)

**CLIP retrieval systems (you’ll still need filters, but this is the ecosystem)**

* **rom1504/clip-retrieval** — widely used CLIP embedding + retrieval system; good reference for building/serving multimodal indexes and candidate sets (then you add your attribute veto/filters on top). ([GitHub][8])

**Attribute-aware / composed image retrieval (query specifies an attribute change)**

* **haokunwen/Awesome-Composed-Image-Retrieval** — curated list of “composed image retrieval” work, much of which is literally “keep everything the same but change the attribute” (your black-vs-white sand vibe). Great for finding specific implementations/datasets. ([GitHub][9])

**Image retrieval reranking by decomposing query into components**

* **freshfish15/CoTRR** — listwise reranking for image retrieval with query deconstruction into semantic components (a fancier version of “treat color as a required facet”). ([GitHub][10])

---

### How to use these as “attribute mismatch antidotes”

If you’re scanning these repos for the exact trick, search within them for keywords like:

* `payload`, `filter`, `must`, `metadata`, `facet`
* `rerank`, `cross-encoder`, `listwise`, `re-ranking`
* `hybrid`, `bm25`, `fusion`, `rrf` (reciprocal rank fusion)

The reliable recipe you’ll see repeated:

1. **Candidate retrieval** via vectors (and often BM25 too)
2. **Metadata pre-filter** (hard constraints)
3. **Rerank** the survivors (soft scoring + penalties)

---

*Footnote (new/improved thing):* I biased the list toward repos where the mitigation is **visible in code paths** (payload filtering / reranking pipelines), rather than papers-only repos, because “I can grep this and steal the pattern” beats “I can cite this.”

[1]: https://github.com/qdrant/qdrant?utm_source=chatgpt.com "GitHub - qdrant/qdrant: Qdrant - High-performance, ..."
[2]: https://github.com/qdrant/examples?utm_source=chatgpt.com "A collection of examples and tutorials for Qdrant vector ..."
[3]: https://github.com/elastic/elasticsearch-labs/blob/main/notebooks/search/12-semantic-reranking-elastic-rerank.ipynb?utm_source=chatgpt.com "12-semantic-reranking-elastic-rerank.ipynb"
[4]: https://github.com/edumunozsala/langchain-rag-techniques/blob/main/Rerank-Fusion-Ensemble-Hybrid-Search.ipynb?utm_source=chatgpt.com "Rerank-Fusion-Ensemble-Hybrid-Search.ipynb"
[5]: https://github.com/yuniko-software/bge-m3-qdrant-sample?utm_source=chatgpt.com "BGE-M3 Qdrant sample. Hybrid search & reranking"
[6]: https://github.com/shanojpillai/qdrant-rag-pro?utm_source=chatgpt.com "shanojpillai/qdrant-rag-pro: Building a Production-Ready ..."
[7]: https://github.com/chatterjeesaurabh/Contextual-RAG-System-with-Hybrid-Search-and-Reranking?utm_source=chatgpt.com "Contextual-RAG-System-with-Hybrid-Search-and-Reranking"
[8]: https://github.com/rom1504/clip-retrieval?utm_source=chatgpt.com "Compute CLIP Embeddings & Build CLIP Retrieval System"
[9]: https://github.com/haokunwen/Awesome-Composed-Image-Retrieval?utm_source=chatgpt.com "haokunwen/Awesome-Composed-Image-Retrieval"
[10]: https://github.com/freshfish15/CoTRR?utm_source=chatgpt.com "Chain-of-Thought Re-Ranking for Image Retrieval (CoTRR)"
