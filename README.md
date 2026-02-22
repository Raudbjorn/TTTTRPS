# TTTTRPS - AI-Powered TTRPG Assistant (TUI)

A terminal-based application for Game Masters running tabletop RPG sessions, powered by multiple LLM backends and built entirely in Rust with [ratatui](https://ratatui.rs/).

The extra T stands for TUI. Forked from [TTTRPS](https://github.com/Raudbjorn/TTTRPS) (Tauri+Leptos desktop app) — same core engine, terminal interface.

## Features

- **Multi-LLM Support**: Claude, Gemini, OpenAI, and local Ollama models
- **Semantic Search**: Hybrid search (vector + BM25) across your rulebooks
- **Campaign Management**: Track campaigns, sessions, and world state
- **Combat Tracker**: Initiative tracking, HP management, conditions
- **Character Generation**: Multi-system support (D&D 5e, Pathfinder, Call of Cthulhu, etc.)
- **NPC Generator**: Procedurally generated NPCs with personality traits
- **Voice Synthesis**: ElevenLabs, OpenAI TTS, and local providers
- **Document Ingestion**: PDF and EPUB parsing with intelligent chunking
- **Secure Storage**: API keys stored in system keyring

## Architecture

```
TTTTRPS/
├── src/
│   ├── main.rs            # TUI entry point (ratatui)
│   ├── lib.rs             # Library root
│   ├── core/              # Core business logic
│   │   ├── llm/           # LLM providers & routing
│   │   ├── search/        # Embedded search (Meilisearch + SurrealDB)
│   │   ├── storage/       # SurrealDB unified storage
│   │   ├── voice/         # Voice synthesis & queue
│   │   ├── campaign/      # Campaign versioning, world state
│   │   ├── personality/   # DM personality system
│   │   └── preprocess/    # Query preprocessing (typo + synonyms)
│   ├── database/          # SQLite with SQLx migrations
│   ├── ingestion/         # Document extraction (PDF, EPUB, DOCX)
│   └── oauth/             # OAuth flows (Claude, Gemini, Copilot)
├── config/                # App configuration files
├── resources/             # Bundled resources (dictionaries, templates)
└── tests/                 # Integration & unit tests
```

## Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain, edition 2021)
- Optional: [Tesseract OCR](https://github.com/tesseract-ocr/tesseract) for scanned PDF extraction
- Optional: [Ollama](https://ollama.ai/) for local LLM inference

## Installation

```bash
git clone https://github.com/Raudbjorn/TTTTRPS.git
cd TTTTRPS
cargo build --release
```

## Usage

```bash
# Run the TUI
cargo run

# Or run the release binary directly
./target/release/ttttrps
```

### Build Commands

```bash
cargo run                  # Development mode
cargo build --release      # Production build
cargo check                # Type check
cargo test --lib           # Run unit tests
cargo test                 # Run all tests
```

## Configuration

### LLM Providers

**Claude (Anthropic)**
- Get API key from: https://console.anthropic.com/
- Models: claude-3-5-sonnet, claude-3-haiku

**Gemini (Google)**
- Get API key from: https://aistudio.google.com/
- Models: gemini-1.5-pro, gemini-1.5-flash

**Ollama (Local)**
- Install: https://ollama.ai/
- Run: `ollama run llama3.2`
- No API key required

**OpenAI**
- Get API key from: https://platform.openai.com/
- Models: gpt-4o, gpt-4-turbo

### Voice Synthesis

- **ElevenLabs** - API key from https://elevenlabs.io/
- **OpenAI TTS** - Uses your OpenAI API key
- **Local**: Chatterbox, GPT-SoVITS, XTTS-v2, Fish Speech, Piper

## Data Storage

- **SurrealDB**: `~/.local/share/ttrpg-assistant/surrealdb/` (RocksDB-backed)
- **Legacy SQLite**: `~/.local/share/ttrpg-assistant/ttrpg_assistant.db`
- **API Keys**: System keyring via `keyring` crate
- **Dictionaries**: `~/.local/share/ttrpg-assistant/ttrpg_corpus.txt`

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- [ratatui](https://ratatui.rs/) - Terminal UI framework
- [Meilisearch](https://meilisearch.com/) - Search engine (embedded)
- [SurrealDB](https://surrealdb.com/) - Multi-model database
- [kreuzberg](https://github.com/kreuzberg-dev/kreuzberg) - Document extraction
