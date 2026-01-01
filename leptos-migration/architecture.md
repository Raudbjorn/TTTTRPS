# Leptos Migration Architecture

This document details the technical architecture for the Leptos-based frontend.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.0.0 |
| Created | 2026-01-01 |
| Status | Draft |

---

## 1. Technology Stack

### 1.1 Frontend Dependencies

```toml
# frontend/Cargo.toml - Target State
[package]
name = "ttrpg-assistant-frontend"
version = "0.1.0"
edition = "2021"

[dependencies]
# Core Framework
leptos = { version = "0.7", features = ["csr"] }
leptos_router = "0.7"

# WASM/Tauri Integration
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde-wasm-bindgen = "0.6"
js-sys = "0.3"
web-sys = { version = "0.3", features = [
    "Window",
    "Document",
    "Element",
    "HtmlElement",
    "console",
] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
gloo-timers = "0.3"
pulldown-cmark = "0.13"

# Logging
tracing = "0.1"
tracing-wasm = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

### 1.2 Build Tooling

**Option A: Trunk** (Recommended for Tauri)
```toml
# Trunk.toml
[build]
target = "index.html"
dist = "dist"

[watch]
watch = ["src", "index.html", "public"]

[[hooks]]
stage = "post_build"
command = "sh"
command_arguments = ["-c", "cp -r public/* dist/"]
```

**Option B: cargo-leptos**
```toml
# Cargo.toml [package.metadata.leptos]
[package.metadata.leptos]
output-name = "ttrpg-assistant"
site-root = "dist"
site-addr = "127.0.0.1:3030"
reload-port = 3001
browserquery = "defaults"
watch = true
```

### 1.3 Tauri Configuration Updates

```json
// src-tauri/tauri.conf.json changes
{
  "build": {
    "beforeDevCommand": "trunk serve --port 3030",
    "devUrl": "http://127.0.0.1:3030",
    "beforeBuildCommand": "trunk build --release",
    "frontendDist": "../frontend/dist"
  }
}
```

---

## 2. Application Structure

### 2.1 Directory Layout

```
frontend/
├── Cargo.toml              # Leptos dependencies
├── Trunk.toml              # Build configuration
├── index.html              # Entry HTML (updated)
├── public/
│   ├── tailwind.css        # Unchanged
│   ├── themes.css          # Unchanged
│   └── favicon.ico
├── src/
│   ├── main.rs             # App entry + routing
│   ├── app.rs              # Root App component
│   ├── bindings.rs         # Tauri IPC (types preserved)
│   ├── components/
│   │   ├── mod.rs
│   │   ├── design_system/
│   │   │   ├── mod.rs
│   │   │   ├── button.rs
│   │   │   ├── input.rs
│   │   │   ├── card.rs
│   │   │   ├── badge.rs
│   │   │   ├── select.rs
│   │   │   ├── modal.rs
│   │   │   └── loading.rs
│   │   ├── chat.rs
│   │   ├── settings.rs
│   │   ├── library.rs
│   │   ├── campaigns.rs
│   │   ├── session.rs
│   │   └── character.rs
│   └── utils/
│       ├── mod.rs
│       └── markdown.rs
└── tests/
    └── integration.rs
```

### 2.2 Entry Point

```rust
// src/main.rs
use leptos::prelude::*;
use leptos_router::*;

mod app;
mod bindings;
mod components;
mod utils;

use app::App;

fn main() {
    // Initialize logging
    tracing_wasm::set_as_global_default();

    // Mount app
    leptos::mount::mount_to_body(App);
}
```

### 2.3 App Component with Routing

```rust
// src/app.rs
use leptos::prelude::*;
use leptos_router::*;

use crate::components::{
    Chat, Settings, Library, Campaigns, Session, CharacterCreator
};

#[component]
pub fn App() -> impl IntoView {
    // Global theme signal
    let (theme, set_theme) = signal("fantasy".to_string());
    provide_context(theme);
    provide_context(set_theme);

    view! {
        <Router>
            <main class="app-container">
                <Routes fallback=|| "Page not found">
                    <Route path="/" view=Chat />
                    <Route path="/settings" view=Settings />
                    <Route path="/library" view=Library />
                    <Route path="/campaigns" view=Campaigns />
                    <Route path="/session/:campaign_id" view=Session />
                    <Route path="/character" view=CharacterCreator />
                </Routes>
            </main>
        </Router>
    }
}
```

---

## 3. State Management

### 3.1 Signal Patterns

```rust
// Dioxus → Leptos Signal Translation

// Local State
// Dioxus:  let mut count = use_signal(|| 0);
// Leptos:  let (count, set_count) = signal(0);

// Reading
// Dioxus:  {count}
// Leptos:  {count.get()} or {count()}

// Writing
// Dioxus:  count += 1;
// Leptos:  set_count.update(|n| *n += 1);

// Derived
// Dioxus:  let doubled = use_memo(move || count() * 2);
// Leptos:  let doubled = Memo::new(move |_| count.get() * 2);
```

### 3.2 Context Pattern

```rust
// Global Theme Context
#[component]
pub fn App() -> impl IntoView {
    let (theme, set_theme) = signal("fantasy".to_string());
    provide_context(theme);
    provide_context(set_theme);
    // ...
}

// Consuming Context
#[component]
pub fn ThemeSelector() -> impl IntoView {
    let theme = expect_context::<ReadSignal<String>>();
    let set_theme = expect_context::<WriteSignal<String>>();

    view! {
        <select on:change=move |ev| {
            set_theme.set(event_target_value(&ev));
        }>
            <option value="fantasy" selected=move || theme.get() == "fantasy">
                "Fantasy"
            </option>
            // ...
        </select>
    }
}
```

### 3.3 Resource Pattern (Async Data)

```rust
// Dioxus use_resource → Leptos Resource

// Leptos async data fetching
#[component]
pub fn CampaignList() -> impl IntoView {
    let campaigns = Resource::new(
        || (),  // source signal (unit = fetch once)
        |_| async move {
            list_campaigns().await.unwrap_or_default()
        }
    );

    view! {
        <Suspense fallback=|| view! { <LoadingSpinner /> }>
            {move || campaigns.get().map(|data| {
                view! {
                    <For
                        each=move || data.clone()
                        key=|c| c.id.clone()
                        children=|campaign| view! {
                            <CampaignCard campaign=campaign />
                        }
                    />
                }
            })}
        </Suspense>
    }
}
```

---

## 4. Tauri IPC Integration

### 4.1 Bindings Structure (Preserved)

```rust
// src/bindings.rs - Structure unchanged, types preserved

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Types remain exactly the same
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub id: String,
    pub name: String,
    pub system: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

// Command wrappers remain the same
pub async fn list_campaigns() -> Result<Vec<Campaign>, String> {
    let result = invoke("list_campaigns", JsValue::NULL).await;
    serde_wasm_bindgen::from_value(result)
        .map_err(|e| e.to_string())
}
```

### 4.2 Using Bindings in Components

```rust
// Leptos component calling Tauri commands
#[component]
pub fn Chat() -> impl IntoView {
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (input, set_input) = signal(String::new());
    let (loading, set_loading) = signal(false);

    let send_message = move |_| {
        let message = input.get();
        if message.is_empty() { return; }

        set_loading.set(true);
        set_input.set(String::new());

        spawn_local(async move {
            match chat(ChatRequestPayload { message, ..Default::default() }).await {
                Ok(response) => {
                    set_messages.update(|msgs| {
                        msgs.push(ChatMessage::user(message.clone()));
                        msgs.push(ChatMessage::assistant(response.response));
                    });
                }
                Err(e) => {
                    // Handle error
                    log::error!("Chat error: {}", e);
                }
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="chat-container">
            <MessageList messages=messages />
            <div class="input-area">
                <Input
                    value=input
                    on_change=move |v| set_input.set(v)
                    placeholder="Type a message..."
                />
                <Button on_click=send_message disabled=loading>
                    {move || if loading.get() { "Sending..." } else { "Send" }}
                </Button>
            </div>
        </div>
    }
}
```

---

## 5. Component Patterns

### 5.1 Design System Button

```rust
// src/components/design_system/button.rs

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Outline,
}

impl ButtonVariant {
    fn class(&self) -> &'static str {
        match self {
            Self::Primary => "btn-primary",
            Self::Secondary => "btn-secondary",
            Self::Danger => "btn-danger",
            Self::Ghost => "btn-ghost",
            Self::Outline => "btn-outline",
        }
    }
}

#[component]
pub fn Button(
    #[prop(into)] children: Children,
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] disabled: MaybeSignal<bool>,
    #[prop(optional)] on_click: Option<Callback<ev::MouseEvent>>,
) -> impl IntoView {
    let variant = variant.unwrap_or(ButtonVariant::Primary);

    view! {
        <button
            class=format!("btn {}", variant.class())
            disabled=move || disabled.get()
            on:click=move |ev| {
                if let Some(handler) = on_click.as_ref() {
                    handler.call(ev);
                }
            }
        >
            {children()}
        </button>
    }
}
```

### 5.2 Design System Input

```rust
// src/components/design_system/input.rs

#[component]
pub fn Input(
    #[prop(into)] value: Signal<String>,
    #[prop(optional)] on_change: Option<Callback<String>>,
    #[prop(optional)] placeholder: &'static str,
    #[prop(optional)] disabled: MaybeSignal<bool>,
    #[prop(optional)] input_type: &'static str,
) -> impl IntoView {
    let input_type = input_type.unwrap_or("text");

    view! {
        <input
            type=input_type
            class="input"
            placeholder=placeholder
            disabled=move || disabled.get()
            prop:value=move || value.get()
            on:input=move |ev| {
                let val = event_target_value(&ev);
                if let Some(handler) = on_change.as_ref() {
                    handler.call(val);
                }
            }
        />
    }
}
```

### 5.3 Modal Pattern

```rust
// src/components/design_system/modal.rs

#[component]
pub fn Modal(
    #[prop(into)] open: Signal<bool>,
    #[prop(into)] on_close: Callback<()>,
    #[prop(into)] title: String,
    children: Children,
) -> impl IntoView {
    view! {
        <Show when=move || open.get()>
            <div class="modal-backdrop" on:click=move |_| on_close.call(())>
                <div class="modal-content" on:click=|ev| ev.stop_propagation()>
                    <div class="modal-header">
                        <h2>{title.clone()}</h2>
                        <button class="modal-close" on:click=move |_| on_close.call(())>
                            "×"
                        </button>
                    </div>
                    <div class="modal-body">
                        {children()}
                    </div>
                </div>
            </div>
        </Show>
    }
}
```

---

## 6. Event Handling

### 6.1 Tauri Events (Streaming, Progress)

```rust
// Listening to Tauri events in Leptos

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"])]
    async fn listen(event: &str, handler: &Closure<dyn Fn(JsValue)>) -> JsValue;
}

#[component]
pub fn DocumentIngestion() -> impl IntoView {
    let (progress, set_progress) = signal(0.0);
    let (stage, set_stage) = signal(String::new());

    // Setup event listener on mount
    Effect::new(move |_| {
        let closure = Closure::new(move |event: JsValue| {
            if let Ok(payload) = serde_wasm_bindgen::from_value::<IngestionProgress>(event) {
                set_progress.set(payload.progress);
                set_stage.set(payload.stage);
            }
        });

        spawn_local(async move {
            listen("ingestion-progress", &closure).await;
            closure.forget(); // Keep alive
        });
    });

    view! {
        <div class="progress-container">
            <div class="progress-bar" style=move || format!("width: {}%", progress.get() * 100.0) />
            <span class="progress-stage">{stage}</span>
        </div>
    }
}
```

---

## 7. Routing

### 7.1 Route Definitions

```rust
// Leptos Router setup

use leptos_router::*;

#[derive(Clone, Routable, PartialEq)]
pub enum AppRoutes {
    #[route("/")]
    Chat,
    #[route("/settings")]
    Settings,
    #[route("/library")]
    Library,
    #[route("/campaigns")]
    Campaigns,
    #[route("/session/:campaign_id")]
    Session { campaign_id: String },
    #[route("/character")]
    CharacterCreator,
}
```

### 7.2 Navigation

```rust
// Programmatic navigation
#[component]
pub fn CampaignCard(campaign: Campaign) -> impl IntoView {
    let navigate = use_navigate();

    let start_session = move |_| {
        navigate(&format!("/session/{}", campaign.id), Default::default());
    };

    view! {
        <div class="campaign-card">
            <h3>{&campaign.name}</h3>
            <Button on_click=start_session>"Start Session"</Button>
        </div>
    }
}
```

---

## 8. Build & Development

### 8.1 Development Workflow

```bash
# Terminal 1: Frontend dev server
cd frontend
trunk serve --port 3030

# Terminal 2: Tauri dev mode
cd src-tauri
cargo tauri dev
```

### 8.2 Production Build

```bash
# Build frontend
cd frontend
trunk build --release

# Build Tauri app
cargo tauri build
```

### 8.3 Testing

```rust
// frontend/tests/integration.rs
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_button_renders() {
    // Component tests
}
```

---

## 9. Migration Considerations

### 9.1 Key Syntax Differences

| Aspect | Dioxus | Leptos |
|--------|--------|--------|
| Component macro | `#[component]` | `#[component]` |
| View macro | `rsx! { }` | `view! { }` |
| Signal create | `use_signal(\|\| val)` | `signal(val)` |
| Signal read | `count()` or `{count}` | `count.get()` |
| Signal write | `count += 1` | `set_count.update(\|n\| *n += 1)` |
| Async spawn | `spawn(async { })` | `spawn_local(async { })` |
| Resource | `use_resource(\|\| async { })` | `Resource::new(\|\| (), \|_\| async { })` |
| Effect | `use_effect(\|\| { })` | `Effect::new(\|_\| { })` |
| Context provide | `use_context_provider(\|\| val)` | `provide_context(val)` |
| Context consume | `use_context::<T>()` | `expect_context::<T>()` |
| Conditional | `if cond { rsx!{} }` | `<Show when=\|\| cond>` |
| List render | `for item in items { rsx!{} }` | `<For each=\|\| items ...>` |

### 9.2 Attribute Differences

| Dioxus | Leptos |
|--------|--------|
| `onclick` | `on:click` |
| `oninput` | `on:input` |
| `class: "foo"` | `class="foo"` |
| `disabled: true` | `disabled=true` |

---

## Related Documents

- [overview.md](./overview.md) - Migration rationale
- [component-mapping.md](./component-mapping.md) - Detailed component translations
- [tasks.md](./tasks.md) - Implementation tasks

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-01 | Initial architecture document |
