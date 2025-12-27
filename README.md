# Sidecar DM - AI-Powered TTRPG Assistant

A desktop application for Game Masters running tabletop RPG sessions, powered by multiple LLM backends and built entirely in Rust.

## Features

- **Multi-LLM Support**: Claude, Gemini, OpenAI, and local Ollama models
- **Semantic Search**: Hybrid search (vector + BM25) across your rulebooks
- **Campaign Management**: Track campaigns, sessions, and world state
- **Combat Tracker**: Initiative tracking, HP management, conditions
- **Character Generation**: Multi-system support (D&D 5e, Pathfinder, Call of Cthulhu, etc.)
- **NPC Generator**: Procedurally generated NPCs with personality traits
- **Voice Synthesis**: ElevenLabs and Ollama TTS integration
- **Document Ingestion**: PDF and EPUB parsing with intelligent chunking
- **Secure Storage**: API keys stored in system keyring

## Architecture

```
TTTRPS/
├── frontend/          # Dioxus WASM frontend
│   └── src/
│       ├── components/   # UI components
│       └── bindings.rs   # Tauri IPC wrappers
├── src-tauri/         # Rust backend
│   └── src/
│       ├── core/         # Core business logic
│       │   ├── llm/      # LLM providers
│       │   ├── vector_store.rs
│       │   ├── keyword_search.rs
│       │   └── ...
│       ├── database/     # SQLite with migrations
│       ├── ingestion/    # Document parsers
│       └── commands.rs   # Tauri command handlers
└── build.sh           # Build script
```

## Prerequisites

### All Platforms
- [Rust](https://rustup.rs/) (stable toolchain)
- WASM target: `rustup target add wasm32-unknown-unknown`

### Linux (Arch)
```bash
paru -S webkit2gtk-4.1 gtk3 libappindicator-gtk3
```

### macOS
Xcode Command Line Tools

### Windows
Visual Studio Build Tools with "Desktop development with C++"

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/TTTRPS.git
cd TTTRPS
```

2. Install CLI tools:
```bash
cargo install dioxus-cli
cargo install tauri-cli
```

3. Build and run:
```bash
./build.sh dev    # Development mode
# or
./build.sh build --release  # Production build
```

## Configuration

### LLM Providers

Configure your preferred LLM provider in the Settings panel:

**Claude (Anthropic)**
- Get API key from: https://console.anthropic.com/
- Recommended models: claude-3-5-sonnet, claude-3-haiku

**Gemini (Google)**
- Get API key from: https://aistudio.google.com/
- Recommended models: gemini-1.5-pro, gemini-1.5-flash

**Ollama (Local)**
- Install: https://ollama.ai/
- Run: `ollama run llama3.2` or your preferred model
- No API key required

### Voice Synthesis

**ElevenLabs**
- Get API key from: https://elevenlabs.io/

**Ollama TTS**
- Uses local Ollama installation
- No additional setup required

## Usage

### Document Ingestion
1. Navigate to Library
2. Drag and drop PDF/EPUB files
3. Wait for processing and indexing

### Campaign Management
1. Go to Campaigns
2. Create a new campaign with system selection
3. Start a session to begin tracking

### Combat Tracker
1. In an active session, start combat
2. Add combatants with initiative rolls
3. Track HP, conditions, and turns

### Character Generation
1. Go to Characters
2. Select game system
3. Configure options (class, level, backstory)
4. Generate!

## Build Commands

```bash
./build.sh dev              # Start development server
./build.sh build            # Build debug version
./build.sh build --release  # Build optimized release
./build.sh test             # Run all tests
./build.sh check            # Run cargo check + clippy
./build.sh clean            # Clean build artifacts
./build.sh help             # Show help
```

## Data Storage

- **Database**: `~/.local/share/ttrpg-assistant/ttrpg_assistant.db`
- **Vector Store**: `~/.local/share/ttrpg-assistant/lancedb/`
- **Backups**: `~/.local/share/ttrpg-assistant/backups/`
- **Cache**: `~/.cache/ttrpg-assistant/`

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Enter | Send message |
| Shift+Enter | New line in message |
| Ctrl+K | Quick search |

## Troubleshooting

### "LLM not configured"
Configure an API key in Settings for your preferred provider.

### "Vector store not initialized"
The app creates the vector store on first document ingestion.

### AppImage doesn't run
```bash
chmod +x ttrpg-assistant.AppImage
./ttrpg-assistant.AppImage --no-sandbox
```

### WebKitGTK errors on Linux
Ensure webkit2gtk-4.1 is installed (not 4.0):
```bash
paru -S webkit2gtk-4.1
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `./build.sh check`
5. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) - Desktop framework
- [Dioxus](https://dioxuslabs.com/) - React-like UI framework
- [LanceDB](https://lancedb.com/) - Vector database
- [Tantivy](https://github.com/quickwit-oss/tantivy) - Search engine
