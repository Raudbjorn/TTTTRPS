This is a classic problem in vector search known as the **"Lexical Gap"** or the loss of fine-grained details.

Dense embeddings (like those from CLIP, BERT, or E5) represent concepts. To an embedding model, "White Sand" and "Black Sand" are 95% identical vectors because they share the concepts: *Beach, Sand, Shore, Texture, Nature*. The color is a minor dimension in that high-dimensional space.

To solve this in Rust, you generally do **not** want to fine-tune the model immediately (which is hard and expensive). Instead, you use a **Hybrid Search** approach (Dense Vector + Sparse Keyword) and specific pre/post-processing hooks.

Here is a breakdown of how to handle this in Rust.

---

### 1. Pre-Processing: Metadata Extraction & Enrichment

Before you even generate the embedding, you should extract explicit features (like colors) to use as filters later. If you rely solely on the generated text description, you are at the mercy of the captioning model.

**Strategy:** Extract specific entities (Nouns, Adjectives) or use a "Color Histogram" on the image to tag the data.

```rust
use std::collections::HashSet;

// A simple struct representing your data before embedding
#[derive(Debug, Clone)]
struct Document {
    id: u64,
    raw_text: String,
    // Extracted metadata tags
    tags: HashSet<String>, 
}

impl Document {
    fn new(id: u64, text: &str) -> Self {
        let mut doc = Document {
            id,
            raw_text: text.to_string(),
            tags: HashSet::new(),
        };
        doc.enrich_metadata();
        doc
    }

    // Pre-processing step: Deterministic extraction
    // If the text mentions specific distinct attributes, lift them to metadata.
    fn enrich_metadata(&mut self) {
        let text_lower = self.raw_text.to_lowercase();
        
        // In a real app, use NLP tokenization or Regex here
        let distinctive_features = vec!["black", "white", "red", "blue", "wooden", "metal"];
        
        for feature in distinctive_features {
            if text_lower.contains(feature) {
                self.tags.insert(feature.to_string());
            }
        }
    }
}

fn main() {
    let doc = Document::new(1, "A beautiful scenic beach with black sand and jagged rocks.");
    println!("Tags: {:?}", doc.tags); 
    // Output: Tags: {"black"}
    // Now you have a hard filter available for search time.
}
```

---

### 2. The Search Strategy: Hybrid Search (The Real Solution)

This is the industry standard solution. You combine:
1.  **Dense Score (Cosine Similarity):** Captures "Is this a beach?"
2.  **Sparse Score (BM25 / Keyword Matching):** Captures "Does it contain the word BLACK?"

If you only use vectors, "White sand" scores 0.85 and "Black sand" scores 0.86.
If you add BM25, "Black sand" gets a massive boost because of the exact term match, while "White sand" gets penalized.

Here is a conceptual implementation of Hybrid Scoring in Rust:

```rust
use std::collections::HashMap;

// Mocking a vector type
type Vector = Vec<f32>;

#[derive(Debug)]
struct SearchResult {
    id: u64,
    score: f32,
}

struct HybridSearcher {
    // Maps Doc ID to Embedding
    dense_index: HashMap<u64, Vector>,
    // Maps Doc ID to Raw Text (for keyword scoring)
    corpus: HashMap<u64, String>,
}

impl HybridSearcher {
    fn cosine_similarity(a: &Vector, b: &Vector) -> f32 {
        let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        dot_product / (norm_a * norm_b)
    }

    // A very naive implementation of Keyword scoring (like TF-IDF/BM25)
    fn keyword_score(query: &str, doc_text: &str) -> f32 {
        let query_tokens: Vec<&str> = query.split_whitespace().collect();
        let doc_lower = doc_text.to_lowercase();
        
        let mut score = 0.0;
        for token in query_tokens {
            let t_lower = token.to_lowercase();
            // In reality, use BM25 here (penalize common words, boost rare words)
            if doc_lower.contains(&t_lower) {
                // Give a high weight to exact matches!
                score += 1.0; 
            }
        }
        score
    }

    pub fn search(&self, query_text: &str, query_vec: &Vector, alpha: f32) -> Vec<SearchResult> {
        let mut results = Vec::new();

        for (id, doc_vec) in &self.dense_index {
            let doc_text = self.corpus.get(id).unwrap();

            // 1. Semantic Score (The embedding logic)
            let dense_score = Self::cosine_similarity(query_vec, doc_vec);

            // 2. Lexical Score (The "Black Sand" fixer)
            let sparse_score = Self::keyword_score(query_text, doc_text);

            // 3. Hybrid Score (Weighted combination)
            // Alpha controls the balance. 0.5 = equal weight.
            // For "Black Sand", the sparse_score will be 0 for the white beach 
            // and 1 for the black beach, creating a massive gap.
            let hybrid_score = (alpha * dense_score) + ((1.0 - alpha) * sparse_score);

            results.push(SearchResult { id: *id, score: hybrid_score });
        }

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results
    }
}

fn main() {
    // ---------------------------------------------------------
    // Scenario: 
    // Doc 1: "White sand beach" (Vector is very close to query)
    // Doc 2: "Black sand beach" (Vector is very close to query)
    // Query: "Black sand beach"
    // ---------------------------------------------------------
    
    // Mock Vectors (simplified: index 0 is "beach-ness", index 1 is "color-val")
    let vec_query = vec![0.9, 0.9]; // High beach, High black
    let vec_white = vec![0.9, 0.1]; // High beach, Low black (White)
    let vec_black = vec![0.9, 0.95]; // High beach, High black

    let mut searcher = HybridSearcher {
        dense_index: HashMap::new(),
        corpus: HashMap::new(),
    };

    searcher.dense_index.insert(1, vec_white);
    searcher.corpus.insert(1, "Beautiful white sand beach".to_string());

    searcher.dense_index.insert(2, vec_black);
    searcher.corpus.insert(2, "Beautiful black sand beach".to_string());

    // Alpha 0.2 means we trust Keywords (0.8) more than vectors (0.2)
    // This is usually tunable based on user intent.
    let results = searcher.search("black sand beach", &vec_query, 0.3);

    println!("Search Results: {:#?}", results);
    // Result 2 will win decisively because it gets the Keyword Score boost.
}
```

---

### 3. Post-Processing: Re-ranking with Cross-Encoders

If you cannot change your indexing strategy (Strategy 2), you can use **Re-ranking**.

1.  Retrieve the top 50 candidates using your Embedding (which might include white beaches).
2.  Pass those 50 pairs (Query + Doc) into a **Cross-Encoder**.
3.  A Cross-Encoder (like a BERT classifier) looks at both texts *simultaneously* and outputs a similarity score. It is much smarter than embeddings at understanding negation and adjectives.

**Rust implementation concept (using `rust-bert` crate logic):**

```rust
// Pseudo-code for Post-Processing / Re-ranking
// Requires 'rust-bert' or 'candle-core'

struct ReRanker {
    // This would actually be a loaded BERT model
}

impl ReRanker {
    // This is computationally expensive, so only run on top 20-50 results
    fn predict_relevance(&self, query: &str, document: &str) -> f32 {
        // A Cross-Encoder sees: "[CLS] black sand beach [SEP] white sand beach [SEP]"
        // It notices the contradiction immediately.
        
        // Mock return:
        if query.contains("black") && !document.contains("black") {
            return 0.1; // Penalize heavily
        }
        0.9
    }
}

fn post_process_results(query: &str, rough_results: Vec<SearchResult>, docs: &HashMap<u64, String>) -> Vec<SearchResult> {
    let re_ranker = ReRanker {}; // Load model
    
    let mut refined_results = Vec::new();

    for res in rough_results {
        let doc_text = docs.get(&res.id).unwrap();
        
        // Rerank!
        let semantic_precision_score = re_ranker.predict_relevance(query, doc_text);
        
        refined_results.push(SearchResult {
            id: res.id,
            score: semantic_precision_score // Or blend this with original score
        });
    }

    refined_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    refined_results
}
```

---

### 4. About the Model Generation (Do I tune?)

**Question:** "Do I tune the model?"

**Short Answer:** Probably not.

**Long Answer:**
Training embedding models (Contrastive Learning) requires triplets of data: `(Anchor, Positive, Negative)`.
*   Anchor: "Black sand beach"
*   Positive: Image of black beach.
*   Negative: Image of white beach.

Unless you have a dataset of millions of these "hard negatives" (items that look similar but are actually different), fine-tuning CLIP/BERT is very difficult and prone to overfitting.

**Better Alternative (SPLADE):**
Instead of tuning a dense model, generate **Sparse Embeddings** (SPLADE).
SPLADE generates a vector that is the size of the vocabulary (30k dimensions), but mostly zeros. It learns to "expand" terms.
*   Input: "Black sand"
*   SPLADE Output: `{"black": 2.5, "dark": 1.2, "volcanic": 0.8, "sand": 1.5}`

This solves your problem at the generation level because the dimension for "black" is explicit. In Rust, you can run SPLADE models using `candle`.

### Summary of Recommendation

1.  **Do not rely on Dense Vectors alone.** They blur the line between antonyms (black/white, hot/cold).
2.  **Implement Hybrid Search.** Store your embeddings in a vector database (like Qdrant, Weaviate, or pgvector) but *also* keep a full-text index (Tantivy is the best Rust library for this).
3.  **Query Logic:**
    *   Query = "Black sand beach"
    *   Vector Search -> Top 100 (Gets all beaches).
    *   Keyword Search (Tantivy/BM25) -> Top 100 (Gets documents with word "Black").
    *   Merge results (RRF - Reciprocal Rank Fusion).
    *   The intersection (Semantic Beach + Keyword Black) rises to the top.
