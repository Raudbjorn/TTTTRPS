# Component Mapping: Dioxus to Leptos

This document provides detailed translation guidance for each existing Dioxus component.

## Document Information

| Field | Value |
|-------|-------|
| Version | 1.0.0 |
| Created | 2026-01-01 |
| Status | Draft |

---

## 1. Component Inventory

### 1.1 Current Dioxus Components

| Component | File | LOC | Complexity |
|-----------|------|-----|------------|
| **Chat** | `chat.rs` | 369 | High |
| **Settings** | `settings.rs` | 981 | High |
| **Library** | `library.rs` | 457 | Medium |
| **Campaigns** | `campaigns.rs` | 271 | Medium |
| **Session** | `session.rs` | 452 | High |
| **CharacterCreator** | `character.rs` | 284 | Medium |
| **Design System** | `design_system.rs` | 365 | Low |
| **Campaign Details** | `campaign_details/*.rs` | ~300 | Medium |

**Total: ~3,479 LOC**

---

## 2. Design System Components

### 2.1 Button

**Dioxus (Current):**
```rust
#[derive(Clone, Copy, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
    Outline,
}

#[component]
pub fn Button(
    children: Element,
    #[props(default = ButtonVariant::Primary)]
    variant: ButtonVariant,
    #[props(default = false)]
    disabled: bool,
    onclick: Option<EventHandler<MouseEvent>>,
) -> Element {
    rsx! {
        button {
            class: "btn {variant.class()}",
            disabled: disabled,
            onclick: move |evt| {
                if let Some(handler) = &onclick {
                    handler.call(evt);
                }
            },
            {children}
        }
    }
}
```

**Leptos (Target):**
```rust
#[derive(Clone, Copy, PartialEq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Danger,
    Ghost,
    Outline,
}

#[component]
pub fn Button(
    children: Children,
    #[prop(default)] variant: ButtonVariant,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
    #[prop(optional)] on_click: Option<Callback<ev::MouseEvent>>,
) -> impl IntoView {
    view! {
        <button
            class=format!("btn {}", variant.class())
            disabled=move || disabled.get()
            on:click=move |ev| {
                if let Some(cb) = on_click.as_ref() {
                    cb.call(ev);
                }
            }
        >
            {children()}
        </button>
    }
}
```

**Key Changes:**
- `Element` → `Children`
- `#[props(default)]` → `#[prop(default)]`
- `EventHandler<MouseEvent>` → `Callback<ev::MouseEvent>`
- `rsx!` → `view!`
- `onclick:` → `on:click=`

---

### 2.2 Input

**Dioxus (Current):**
```rust
#[component]
pub fn Input(
    value: String,
    oninput: EventHandler<FormEvent>,
    #[props(default = "".to_string())]
    placeholder: String,
    #[props(default = false)]
    disabled: bool,
) -> Element {
    rsx! {
        input {
            class: "input",
            value: value,
            placeholder: placeholder,
            disabled: disabled,
            oninput: move |evt| oninput.call(evt),
        }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn Input(
    #[prop(into)] value: Signal<String>,
    #[prop(optional)] on_input: Option<Callback<String>>,
    #[prop(default = "")] placeholder: &'static str,
    #[prop(optional, into)] disabled: MaybeSignal<bool>,
) -> impl IntoView {
    view! {
        <input
            class="input"
            prop:value=move || value.get()
            placeholder=placeholder
            disabled=move || disabled.get()
            on:input=move |ev| {
                if let Some(cb) = on_input.as_ref() {
                    cb.call(event_target_value(&ev));
                }
            }
        />
    }
}
```

**Key Changes:**
- `value: String` → `value: Signal<String>` (reactive binding)
- `oninput:` → `on:input=`
- `value:` → `prop:value=` (for controlled inputs)
- String → `&'static str` for static props

---

### 2.3 Card / CardHeader / CardBody

**Dioxus (Current):**
```rust
#[component]
pub fn Card(children: Element) -> Element {
    rsx! {
        div { class: "card", {children} }
    }
}

#[component]
pub fn CardHeader(children: Element) -> Element {
    rsx! {
        div { class: "card-header", {children} }
    }
}

#[component]
pub fn CardBody(children: Element) -> Element {
    rsx! {
        div { class: "card-body", {children} }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn Card(children: Children) -> impl IntoView {
    view! { <div class="card">{children()}</div> }
}

#[component]
pub fn CardHeader(children: Children) -> impl IntoView {
    view! { <div class="card-header">{children()}</div> }
}

#[component]
pub fn CardBody(children: Children) -> impl IntoView {
    view! { <div class="card-body">{children()}</div> }
}
```

**Key Changes:**
- `Element` → `Children`
- `{children}` → `{children()}`
- `rsx!` → `view!`

---

### 2.4 Badge

**Dioxus (Current):**
```rust
#[derive(Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    Default,
    Success,
    Warning,
    Danger,
    Info,
}

#[component]
pub fn Badge(
    children: Element,
    #[props(default = BadgeVariant::Default)]
    variant: BadgeVariant,
) -> Element {
    rsx! {
        span { class: "badge {variant.class()}", {children} }
    }
}
```

**Leptos (Target):**
```rust
#[derive(Clone, Copy, PartialEq, Default)]
pub enum BadgeVariant {
    #[default]
    Default,
    Success,
    Warning,
    Danger,
    Info,
}

#[component]
pub fn Badge(
    children: Children,
    #[prop(default)] variant: BadgeVariant,
) -> impl IntoView {
    view! {
        <span class=format!("badge {}", variant.class())>
            {children()}
        </span>
    }
}
```

---

### 2.5 Select

**Dioxus (Current):**
```rust
#[component]
pub fn Select(
    value: String,
    options: Vec<(String, String)>,  // (value, label)
    onchange: EventHandler<FormEvent>,
) -> Element {
    rsx! {
        select {
            class: "select",
            value: value,
            onchange: move |evt| onchange.call(evt),
            for (val, label) in options.iter() {
                option { value: "{val}", selected: *val == value, "{label}" }
            }
        }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn Select(
    #[prop(into)] value: Signal<String>,
    options: Vec<(String, String)>,
    #[prop(optional)] on_change: Option<Callback<String>>,
) -> impl IntoView {
    view! {
        <select
            class="select"
            on:change=move |ev| {
                let val = event_target_value(&ev);
                if let Some(cb) = on_change.as_ref() {
                    cb.call(val);
                }
            }
        >
            <For
                each=move || options.clone()
                key=|(val, _)| val.clone()
                children=move |(val, label)| {
                    let selected = value.get() == val;
                    view! {
                        <option value=val.clone() selected=selected>
                            {label}
                        </option>
                    }
                }
            />
        </select>
    }
}
```

**Key Changes:**
- `for ... in` → `<For each=... />`
- Event value extraction: `event_target_value(&ev)`

---

### 2.6 Modal

**Dioxus (Current):**
```rust
#[component]
pub fn Modal(
    open: bool,
    onclose: EventHandler<()>,
    title: String,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }

    rsx! {
        div { class: "modal-backdrop", onclick: move |_| onclose.call(()),
            div { class: "modal-content", onclick: move |evt| evt.stop_propagation(),
                div { class: "modal-header",
                    h2 { "{title}" }
                    button { class: "modal-close", onclick: move |_| onclose.call(()), "×" }
                }
                div { class: "modal-body", {children} }
            }
        }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn Modal(
    #[prop(into)] open: Signal<bool>,
    on_close: Callback<()>,
    title: String,
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

**Key Changes:**
- `if !open { return rsx!{} }` → `<Show when=...>`
- `onclick:` → `on:click=`

---

### 2.7 LoadingSpinner

**Dioxus (Current):**
```rust
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div { class: "loading-spinner" }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn LoadingSpinner() -> impl IntoView {
    view! { <div class="loading-spinner"></div> }
}
```

---

### 2.8 TypingIndicator

**Dioxus (Current):**
```rust
#[component]
pub fn TypingIndicator() -> Element {
    rsx! {
        div { class: "typing-indicator",
            span { class: "dot" }
            span { class: "dot" }
            span { class: "dot" }
        }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn TypingIndicator() -> impl IntoView {
    view! {
        <div class="typing-indicator">
            <span class="dot"></span>
            <span class="dot"></span>
            <span class="dot"></span>
        </div>
    }
}
```

---

### 2.9 Markdown

**Dioxus (Current):**
```rust
#[component]
pub fn Markdown(content: String) -> Element {
    let html = use_memo(move || {
        let parser = pulldown_cmark::Parser::new(&content);
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);
        html_output
    });

    rsx! {
        div { class: "markdown-content", dangerous_inner_html: "{html}" }
    }
}
```

**Leptos (Target):**
```rust
#[component]
pub fn Markdown(#[prop(into)] content: MaybeSignal<String>) -> impl IntoView {
    let html = Memo::new(move |_| {
        let text = content.get();
        let parser = pulldown_cmark::Parser::new(&text);
        let mut html_output = String::new();
        pulldown_cmark::html::push_html(&mut html_output, parser);
        html_output
    });

    view! {
        <div class="markdown-content" inner_html=move || html.get()></div>
    }
}
```

**Key Changes:**
- `use_memo` → `Memo::new`
- `dangerous_inner_html:` → `inner_html=`

---

## 3. Page Components

### 3.1 Chat Component (High Complexity)

**Pattern Translation:**

```rust
// Dioxus structure
#[component]
pub fn Chat() -> Element {
    let mut messages = use_signal(|| vec![]);
    let mut input = use_signal(|| String::new());
    let mut loading = use_signal(|| false);

    let send = move |_| {
        let msg = input();
        if msg.is_empty() { return; }
        loading.set(true);

        spawn(async move {
            match chat(payload).await {
                Ok(response) => messages.write().push(response),
                Err(e) => { /* handle */ }
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "chat",
            for msg in messages.read().iter() {
                MessageBubble { message: msg.clone() }
            }
            // ...
        }
    }
}

// Leptos equivalent
#[component]
pub fn Chat() -> impl IntoView {
    let (messages, set_messages) = signal(Vec::<Message>::new());
    let (input, set_input) = signal(String::new());
    let (loading, set_loading) = signal(false);

    let send = move |_| {
        let msg = input.get();
        if msg.is_empty() { return; }
        set_loading.set(true);

        spawn_local(async move {
            match chat(payload).await {
                Ok(response) => {
                    set_messages.update(|msgs| msgs.push(response));
                }
                Err(e) => { /* handle */ }
            }
            set_loading.set(false);
        });
    };

    view! {
        <div class="chat">
            <For
                each=move || messages.get()
                key=|msg| msg.id.clone()
                children=|msg| view! { <MessageBubble message=msg /> }
            />
            // ...
        </div>
    }
}
```

---

### 3.2 Settings Component (High Complexity)

**Key Patterns:**

1. **Multiple sections with local state:**
```rust
// Leptos
let (llm_provider, set_llm_provider) = signal(String::new());
let (voice_provider, set_voice_provider) = signal(String::new());
let (theme, set_theme) = expect_context::<(ReadSignal<String>, WriteSignal<String>)>();
```

2. **Form submission:**
```rust
let save_settings = move |_| {
    spawn_local(async move {
        if let Err(e) = configure_llm(LLMSettings { ... }).await {
            // Show error
        }
    });
};
```

3. **Async data loading:**
```rust
let models = Resource::new(
    move || llm_provider.get(),
    |provider| async move {
        list_provider_models(&provider).await.unwrap_or_default()
    }
);
```

---

### 3.3 Session Component (High Complexity)

**Combat state pattern:**
```rust
#[component]
pub fn Session(campaign_id: String) -> impl IntoView {
    let campaign = Resource::new(
        move || campaign_id.clone(),
        |id| async move { get_campaign(&id).await.ok() }
    );

    let (combat, set_combat) = signal(None::<CombatState>);

    // Combat actions
    let next_turn = move |_| {
        spawn_local(async move {
            if let Some(state) = combat.get() {
                if let Ok(new_state) = next_turn_cmd(&state.id).await {
                    set_combat.set(Some(new_state));
                }
            }
        });
    };

    view! {
        <Suspense fallback=|| view! { <LoadingSpinner /> }>
            {move || campaign.get().flatten().map(|c| view! {
                <div class="session">
                    <SessionHeader campaign=c.clone() />
                    <Show when=move || combat.get().is_some()>
                        <CombatTracker
                            combat=combat
                            on_next_turn=next_turn
                        />
                    </Show>
                </div>
            })}
        </Suspense>
    }
}
```

---

## 4. Common Patterns Reference

### 4.1 Conditional Rendering

| Pattern | Dioxus | Leptos |
|---------|--------|--------|
| Simple if | `if cond { rsx!{} }` | `<Show when=move \|\| cond>` |
| If-else | `if cond { } else { }` | `<Show when=... fallback=\|\| view!{}>` |
| Match | `match val { }` | `{move \|\| match val.get() { }}` |

### 4.2 List Rendering

| Pattern | Dioxus | Leptos |
|---------|--------|--------|
| Simple | `for item in items { }` | `<For each=... key=... children=...>` |
| With index | `for (i, item) in items.iter().enumerate()` | `<For ... let:index>` |

### 4.3 Event Handling

| Event | Dioxus | Leptos |
|-------|--------|--------|
| Click | `onclick: move \|_\| {}` | `on:click=move \|_\| {}` |
| Input | `oninput: move \|e\| {}` | `on:input=move \|e\| {}` |
| Change | `onchange: move \|e\| {}` | `on:change=move \|e\| {}` |
| Submit | `onsubmit: move \|e\| {}` | `on:submit=move \|e\| {}` |
| Key | `onkeydown: move \|e\| {}` | `on:keydown=move \|e\| {}` |

### 4.4 Async Operations

| Pattern | Dioxus | Leptos |
|---------|--------|--------|
| Spawn async | `spawn(async { })` | `spawn_local(async { })` |
| Resource | `use_resource(\|\| async { })` | `Resource::new(\|\| (), \|_\| async { })` |
| Effect | `use_effect(\|\| { })` | `Effect::new(\|_\| { })` |

---

## 5. Migration Checklist per Component

### Generic Checklist

- [ ] Replace `rsx!` with `view!`
- [ ] Convert `Element` to `impl IntoView`
- [ ] Convert `use_signal` to `signal()`
- [ ] Convert signal reads from `signal()` to `signal.get()`
- [ ] Convert signal writes from `signal.set()` to `set_signal.set()`
- [ ] Replace `spawn()` with `spawn_local()`
- [ ] Convert event handlers (`onclick:` → `on:click=`)
- [ ] Update props syntax (`#[props(...)]` → `#[prop(...)]`)
- [ ] Convert `for` loops to `<For>` component
- [ ] Convert `if` conditionals to `<Show>` component
- [ ] Update context patterns
- [ ] Test functionality

---

## Related Documents

- [overview.md](./overview.md) - Migration rationale
- [architecture.md](./architecture.md) - Technical architecture
- [tasks.md](./tasks.md) - Implementation tasks

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0.0 | 2026-01-01 | Initial component mapping |
