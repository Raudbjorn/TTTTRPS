//! Notes Panel Component (TASK-017)
//!
//! Session notes panel with CRUD, tagging, search, and AI categorization.
//! Now integrated with Tauri backend for persistence and real AI categorization.

use leptos::prelude::*;
use leptos::ev;
use wasm_bindgen_futures::spawn_local;
use log::{info, error};

use crate::components::design_system::{Button, ButtonVariant, Card, CardHeader, CardBody, Badge, BadgeVariant, Input};
use crate::bindings::{
    self,
    SessionNote as BackendNote,
    NoteCategory as BackendCategory,
    CategorizationResponse,
};

// ============================================================================
// Note Types (Frontend versions)
// ============================================================================

/// Note category for display
#[derive(Debug, Clone, PartialEq)]
pub enum NoteCategory {
    General,
    Combat,
    Character,
    Location,
    Plot,
    Quest,
    Loot,
    Rules,
    Meta,
    Worldbuilding,
    Dialogue,
    Secret,
    Custom(String),
}

impl NoteCategory {
    pub fn all() -> Vec<Self> {
        vec![
            Self::General,
            Self::Combat,
            Self::Character,
            Self::Location,
            Self::Plot,
            Self::Quest,
            Self::Loot,
            Self::Rules,
            Self::Meta,
            Self::Worldbuilding,
            Self::Dialogue,
            Self::Secret,
        ]
    }

    /// Convert from backend category
    pub fn from_backend(cat: &BackendCategory) -> Self {
        match cat {
            BackendCategory::General => Self::General,
            BackendCategory::Combat => Self::Combat,
            BackendCategory::Character => Self::Character,
            BackendCategory::Location => Self::Location,
            BackendCategory::Plot => Self::Plot,
            BackendCategory::Quest => Self::Quest,
            BackendCategory::Loot => Self::Loot,
            BackendCategory::Rules => Self::Rules,
            BackendCategory::Meta => Self::Meta,
            BackendCategory::Worldbuilding => Self::Worldbuilding,
            BackendCategory::Dialogue => Self::Dialogue,
            BackendCategory::Secret => Self::Secret,
            BackendCategory::Custom(s) => Self::Custom(s.clone()),
        }
    }

    /// Convert to backend category string
    pub fn to_backend_string(&self) -> String {
        match self {
            Self::General => "general".to_string(),
            Self::Combat => "combat".to_string(),
            Self::Character => "character".to_string(),
            Self::Location => "location".to_string(),
            Self::Plot => "plot".to_string(),
            Self::Quest => "quest".to_string(),
            Self::Loot => "loot".to_string(),
            Self::Rules => "rules".to_string(),
            Self::Meta => "meta".to_string(),
            Self::Worldbuilding => "worldbuilding".to_string(),
            Self::Dialogue => "dialogue".to_string(),
            Self::Secret => "secret".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// Parse from a string (for AI response)
    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "general" => Self::General,
            "combat" => Self::Combat,
            "character" => Self::Character,
            "location" => Self::Location,
            "plot" => Self::Plot,
            "quest" => Self::Quest,
            "loot" => Self::Loot,
            "rules" => Self::Rules,
            "meta" => Self::Meta,
            "worldbuilding" => Self::Worldbuilding,
            "dialogue" => Self::Dialogue,
            "secret" => Self::Secret,
            _ => Self::Custom(s.to_string()),
        }
    }

    pub fn display(&self) -> &str {
        match self {
            Self::General => "General",
            Self::Combat => "Combat",
            Self::Character => "Character",
            Self::Location => "Location",
            Self::Plot => "Plot",
            Self::Quest => "Quest",
            Self::Loot => "Loot",
            Self::Rules => "Rules",
            Self::Meta => "Meta",
            Self::Worldbuilding => "Worldbuilding",
            Self::Dialogue => "Dialogue",
            Self::Secret => "Secret",
            Self::Custom(s) => s,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::General => "file-text",
            Self::Combat => "swords",
            Self::Character => "user",
            Self::Location => "map-pin",
            Self::Plot => "book-open",
            Self::Quest => "target",
            Self::Loot => "package",
            Self::Rules => "book",
            Self::Meta => "settings",
            Self::Worldbuilding => "globe",
            Self::Dialogue => "message-square",
            Self::Secret => "lock",
            Self::Custom(_) => "tag",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::General => "#71717a",
            Self::Combat => "#ef4444",
            Self::Character => "#3b82f6",
            Self::Location => "#22c55e",
            Self::Plot => "#a855f7",
            Self::Quest => "#f59e0b",
            Self::Loot => "#eab308",
            Self::Rules => "#64748b",
            Self::Meta => "#6b7280",
            Self::Worldbuilding => "#06b6d4",
            Self::Dialogue => "#ec4899",
            Self::Secret => "#7c3aed",
            Self::Custom(_) => "#94a3b8",
        }
    }
}

/// A session note for display
#[derive(Debug, Clone, PartialEq)]
pub struct SessionNote {
    pub id: String,
    pub title: String,
    pub content: String,
    pub category: NoteCategory,
    pub tags: Vec<String>,
    pub is_pinned: bool,
    pub is_private: bool,
    pub created_at: String,
    pub updated_at: String,
    pub ai_summary: Option<String>,
}

impl SessionNote {
    /// Create a new note with defaults
    pub fn new(title: String, content: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            content,
            category: NoteCategory::General,
            tags: Vec::new(),
            is_pinned: false,
            is_private: false,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
            ai_summary: None,
        }
    }

    /// Convert from backend note
    pub fn from_backend(note: BackendNote) -> Self {
        Self {
            id: note.id,
            title: note.title,
            content: note.content,
            category: NoteCategory::from_backend(&note.category),
            tags: note.tags,
            is_pinned: note.is_pinned,
            is_private: note.is_private,
            created_at: note.created_at,
            updated_at: note.updated_at,
            ai_summary: None,
        }
    }

    /// Convert to backend note format for updates
    pub fn to_backend(&self, session_id: &str, campaign_id: &str) -> BackendNote {
        BackendNote {
            id: self.id.clone(),
            session_id: session_id.to_string(),
            campaign_id: campaign_id.to_string(),
            title: self.title.clone(),
            content: self.content.clone(),
            category: match &self.category {
                NoteCategory::General => BackendCategory::General,
                NoteCategory::Combat => BackendCategory::Combat,
                NoteCategory::Character => BackendCategory::Character,
                NoteCategory::Location => BackendCategory::Location,
                NoteCategory::Plot => BackendCategory::Plot,
                NoteCategory::Quest => BackendCategory::Quest,
                NoteCategory::Loot => BackendCategory::Loot,
                NoteCategory::Rules => BackendCategory::Rules,
                NoteCategory::Meta => BackendCategory::Meta,
                NoteCategory::Worldbuilding => BackendCategory::Worldbuilding,
                NoteCategory::Dialogue => BackendCategory::Dialogue,
                NoteCategory::Secret => BackendCategory::Secret,
                NoteCategory::Custom(s) => BackendCategory::Custom(s.clone()),
            },
            tags: self.tags.clone(),
            linked_entities: vec![],
            is_pinned: self.is_pinned,
            is_private: self.is_private,
            created_at: self.created_at.clone(),
            updated_at: self.updated_at.clone(),
        }
    }
}

// ============================================================================
// Notes Panel Component
// ============================================================================

/// Main notes panel component
#[component]
pub fn NotesPanel(
    /// Session ID
    session_id: Signal<String>,
    /// Campaign ID
    campaign_id: Signal<String>,
    /// Notes data
    notes: RwSignal<Vec<SessionNote>>,
    /// Callback when note is created
    #[prop(optional)]
    on_note_created: Option<Callback<SessionNote>>,
    /// Callback when note is updated
    #[prop(optional)]
    on_note_updated: Option<Callback<SessionNote>>,
    /// Callback when note is deleted
    #[prop(optional)]
    on_note_deleted: Option<Callback<String>>,
) -> impl IntoView {
    // UI state
    let search_query = RwSignal::new(String::new());
    let selected_category = RwSignal::new(Option::<NoteCategory>::None);
    let show_pinned_only = RwSignal::new(false);
    let show_editor = RwSignal::new(false);
    let editing_note_id = RwSignal::new(Option::<String>::None);
    let is_loading = RwSignal::new(false);
    let error_message = RwSignal::new(Option::<String>::None);

    // Editor state
    let editor_title = RwSignal::new(String::new());
    let editor_content = RwSignal::new(String::new());
    let editor_category = RwSignal::new(NoteCategory::General);
    let editor_tags = RwSignal::new(String::new());
    let editor_is_pinned = RwSignal::new(false);
    let editor_is_private = RwSignal::new(false);

    // AI categorization state
    let is_categorizing = RwSignal::new(false);
    let ai_suggestions = RwSignal::new(Option::<(NoteCategory, Vec<String>)>::None);

    // Load notes from backend when session changes
    Effect::new(move |_| {
        let sid = session_id.get();
        if sid.is_empty() {
            return;
        }

        is_loading.set(true);
        error_message.set(None);

        spawn_local(async move {
            match bindings::list_session_notes(sid).await {
                Ok(backend_notes) => {
                    let frontend_notes: Vec<SessionNote> = backend_notes
                        .into_iter()
                        .map(SessionNote::from_backend)
                        .collect();
                    notes.set(frontend_notes);
                    info!("Loaded {} notes for session", notes.get().len());
                }
                Err(e) => {
                    error!("Failed to load notes: {}", e);
                    error_message.set(Some(format!("Failed to load notes: {}", e)));
                }
            }
            is_loading.set(false);
        });
    });

    // Filtered notes
    let filtered_notes = Memo::new(move |_| {
        let all_notes = notes.get();
        let query = search_query.get().to_lowercase();
        let category = selected_category.get();
        let pinned_only = show_pinned_only.get();

        all_notes
            .into_iter()
            .filter(|n| {
                // Filter by search
                if !query.is_empty() {
                    if !n.title.to_lowercase().contains(&query)
                        && !n.content.to_lowercase().contains(&query)
                        && !n.tags.iter().any(|t| t.to_lowercase().contains(&query))
                    {
                        return false;
                    }
                }

                // Filter by category
                if let Some(ref cat) = category {
                    if &n.category != cat {
                        return false;
                    }
                }

                // Filter by pinned
                if pinned_only && !n.is_pinned {
                    return false;
                }

                true
            })
            .collect::<Vec<_>>()
    });

    // Open editor for new note
    let open_new_note = move |_: ev::MouseEvent| {
        editing_note_id.set(None);
        editor_title.set(String::new());
        editor_content.set(String::new());
        editor_category.set(NoteCategory::General);
        editor_tags.set(String::new());
        editor_is_pinned.set(false);
        editor_is_private.set(false);
        ai_suggestions.set(None);
        show_editor.set(true);
    };

    // Open editor for existing note
    let open_edit_note = move |note: SessionNote| {
        editing_note_id.set(Some(note.id.clone()));
        editor_title.set(note.title.clone());
        editor_content.set(note.content.clone());
        editor_category.set(note.category.clone());
        editor_tags.set(note.tags.join(", "));
        editor_is_pinned.set(note.is_pinned);
        editor_is_private.set(note.is_private);
        ai_suggestions.set(None);
        show_editor.set(true);
    };

    // Save note (calls backend)
    let save_note = move |_: ev::MouseEvent| {
        let title = editor_title.get();
        let content = editor_content.get();
        let sid = session_id.get();
        let cid = campaign_id.get();

        if title.is_empty() {
            return;
        }

        let tags: Vec<String> = editor_tags.get()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let category_str = editor_category.get().to_backend_string();
        let is_pinned = editor_is_pinned.get();
        let is_private = editor_is_private.get();
        let is_new = editing_note_id.get().is_none();
        let existing_id = editing_note_id.get();

        // Close editor immediately for responsiveness
        show_editor.set(false);

        spawn_local(async move {
            if is_new {
                // Create new note via backend
                match bindings::create_session_note(
                    sid.clone(),
                    cid.clone(),
                    title.clone(),
                    content.clone(),
                    Some(category_str),
                    Some(tags.clone()),
                    Some(is_pinned),
                    Some(is_private),
                ).await {
                    Ok(backend_note) => {
                        let frontend_note = SessionNote::from_backend(backend_note);
                        notes.update(|all| all.push(frontend_note.clone()));
                        if let Some(callback) = on_note_created {
                            callback.run(frontend_note);
                        }
                        info!("Created new note");
                    }
                    Err(e) => {
                        error!("Failed to create note: {}", e);
                        error_message.set(Some(format!("Failed to create note: {}", e)));
                    }
                }
            } else if let Some(note_id) = existing_id {
                // Update existing note
                let note = SessionNote {
                    id: note_id,
                    title,
                    content,
                    category: NoteCategory::from_string(&category_str),
                    tags,
                    is_pinned,
                    is_private,
                    created_at: chrono::Utc::now().to_rfc3339(),
                    updated_at: chrono::Utc::now().to_rfc3339(),
                    ai_summary: None,
                };
                let backend_note = note.to_backend(&sid, &cid);

                match bindings::update_session_note(backend_note).await {
                    Ok(updated) => {
                        let frontend_note = SessionNote::from_backend(updated);
                        notes.update(|all| {
                            if let Some(pos) = all.iter().position(|n| n.id == frontend_note.id) {
                                all[pos] = frontend_note.clone();
                            }
                        });
                        if let Some(callback) = on_note_updated {
                            callback.run(frontend_note);
                        }
                        info!("Updated note");
                    }
                    Err(e) => {
                        error!("Failed to update note: {}", e);
                        error_message.set(Some(format!("Failed to update note: {}", e)));
                    }
                }
            }
        });
    };

    // Delete note (calls backend)
    let delete_note = move |note_id: String| {
        let nid = note_id.clone();

        // Optimistic update
        notes.update(|all| {
            all.retain(|n| n.id != note_id);
        });

        spawn_local(async move {
            match bindings::delete_session_note(nid.clone()).await {
                Ok(_) => {
                    if let Some(callback) = on_note_deleted {
                        callback.run(nid);
                    }
                    info!("Deleted note");
                }
                Err(e) => {
                    error!("Failed to delete note: {}", e);
                    error_message.set(Some(format!("Failed to delete note: {}", e)));
                }
            }
        });
    };

    // Request AI categorization (calls backend LLM)
    let request_ai_categorization = move |_: ev::MouseEvent| {
        let title = editor_title.get();
        let content = editor_content.get();

        if title.is_empty() && content.is_empty() {
            return;
        }

        is_categorizing.set(true);

        spawn_local(async move {
            match bindings::categorize_note_ai(title, content).await {
                Ok(response) => {
                    let suggested_category = NoteCategory::from_string(&response.suggested_category);
                    let suggested_tags = response.suggested_tags;
                    ai_suggestions.set(Some((suggested_category, suggested_tags)));
                    info!("AI categorization complete: {} (confidence: {:.0}%)",
                        response.suggested_category, response.confidence * 100.0);
                }
                Err(e) => {
                    // Fall back to simple keyword-based categorization
                    error!("AI categorization failed, using fallback: {}", e);
                    let content_lower = content.to_lowercase();
                    let suggested_category = if content_lower.contains("combat") || content_lower.contains("fight") || content_lower.contains("attack") {
                        NoteCategory::Combat
                    } else if content_lower.contains("npc") || content_lower.contains("character") {
                        NoteCategory::Character
                    } else if content_lower.contains("location") || content_lower.contains("place") || content_lower.contains("tavern") {
                        NoteCategory::Location
                    } else if content_lower.contains("quest") || content_lower.contains("mission") {
                        NoteCategory::Quest
                    } else if content_lower.contains("loot") || content_lower.contains("treasure") || content_lower.contains("gold") {
                        NoteCategory::Loot
                    } else if content_lower.contains("plot") || content_lower.contains("story") {
                        NoteCategory::Plot
                    } else {
                        NoteCategory::General
                    };

                    let mut suggested_tags = Vec::new();
                    if content_lower.contains("dragon") { suggested_tags.push("dragon".to_string()); }
                    if content_lower.contains("magic") { suggested_tags.push("magic".to_string()); }
                    if content_lower.contains("sword") { suggested_tags.push("weapon".to_string()); }

                    ai_suggestions.set(Some((suggested_category, suggested_tags)));
                }
            }
            is_categorizing.set(false);
        });
    };

    // Apply AI suggestions
    let apply_ai_suggestions = move |_: ev::MouseEvent| {
        if let Some((category, tags)) = ai_suggestions.get() {
            editor_category.set(category);
            let current_tags = editor_tags.get();
            let new_tags = if current_tags.is_empty() {
                tags.join(", ")
            } else {
                format!("{}, {}", current_tags, tags.join(", "))
            };
            editor_tags.set(new_tags);
            ai_suggestions.set(None);
        }
    };

    view! {
        <Card class="notes-panel h-full flex flex-col">
            // Header with search and actions
            <CardHeader class="flex flex-col gap-3">
                <div class="flex items-center justify-between">
                    <h3 class="font-bold text-zinc-200 flex items-center gap-2">
                        <svg class="w-5 h-5 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                        </svg>
                        "Session Notes"
                    </h3>
                    <Button
                        variant=ButtonVariant::Primary
                        class="px-3 py-1.5 bg-purple-600 hover:bg-purple-500 text-white text-sm font-medium"
                        on_click=open_new_note
                    >
                        <span class="flex items-center gap-1">
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4"/>
                            </svg>
                            "New Note"
                        </span>
                    </Button>
                </div>

                // Search bar
                <div class="flex gap-2">
                    <div class="flex-1 relative">
                        <input
                            type="text"
                            placeholder="Search notes..."
                            class="w-full pl-9 pr-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                            prop:value=move || search_query.get()
                            on:input=move |ev| search_query.set(event_target_value(&ev))
                        />
                        <svg class="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-zinc-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"/>
                        </svg>
                    </div>
                    <button
                        class=move || format!(
                            "px-3 py-2 rounded-lg border transition-colors {}",
                            if show_pinned_only.get() {
                                "bg-yellow-900/30 border-yellow-600/50 text-yellow-400"
                            } else {
                                "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-white"
                            }
                        )
                        on:click=move |_| show_pinned_only.update(|v| *v = !*v)
                        aria-label="Toggle pinned notes only"
                    >
                        <svg class="w-4 h-4" fill="currentColor" viewBox="0 0 24 24">
                            <path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"/>
                            <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
                        </svg>
                    </button>
                </div>

                // Category filters
                <div class="flex flex-wrap gap-1">
                    <button
                        class=move || format!(
                            "px-2 py-1 text-xs rounded transition-colors {}",
                            if selected_category.get().is_none() {
                                "bg-purple-600/30 text-purple-300 border border-purple-500/50"
                            } else {
                                "bg-zinc-800 text-zinc-400 border border-zinc-700 hover:text-white"
                            }
                        )
                        on:click=move |_| selected_category.set(None)
                    >
                        "All"
                    </button>
                    {NoteCategory::all().into_iter().take(6).map(|cat| {
                        let cat_clone = cat.clone();
                        let cat_display = cat.display().to_string();
                        let cat_color = cat.color();

                        view! {
                            <button
                                class=move || format!(
                                    "px-2 py-1 text-xs rounded transition-colors border {}",
                                    if selected_category.get().as_ref() == Some(&cat_clone) {
                                        "bg-opacity-30 border-opacity-50"
                                    } else {
                                        "bg-zinc-800 border-zinc-700 text-zinc-400 hover:text-white"
                                    }
                                )
                                style:background-color=move || {
                                    if selected_category.get().as_ref() == Some(&cat_clone) {
                                        format!("{}30", cat_color)
                                    } else {
                                        String::new()
                                    }
                                }
                                style:border-color=move || {
                                    if selected_category.get().as_ref() == Some(&cat_clone) {
                                        format!("{}50", cat_color)
                                    } else {
                                        String::new()
                                    }
                                }
                                style:color=move || {
                                    if selected_category.get().as_ref() == Some(&cat_clone) {
                                        cat_color.to_string()
                                    } else {
                                        String::new()
                                    }
                                }
                                on:click={
                                    let cat_for_click = cat.clone();
                                    move |_| selected_category.set(Some(cat_for_click.clone()))
                                }
                            >
                                {cat_display}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </CardHeader>

            // Notes list
            <CardBody class="flex-1 overflow-y-auto p-0">
                <Show
                    when=move || !filtered_notes.get().is_empty()
                    fallback=|| view! {
                        <div class="py-12 text-center">
                            <svg class="w-12 h-12 mx-auto text-zinc-600 mb-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"/>
                            </svg>
                            <p class="text-zinc-500">"No notes yet"</p>
                            <p class="text-sm text-zinc-600">"Create your first note to get started"</p>
                        </div>
                    }
                >
                    <div class="divide-y divide-zinc-700/50">
                        <For
                            each=move || filtered_notes.get()
                            key=|note| note.id.clone()
                            children=move |note| {
                                let note_clone = note.clone();
                                let note_id = note.id.clone();
                                view! {
                                    <NoteCard
                                        note=note
                                        on_edit=Callback::new(move |_| open_edit_note(note_clone.clone()))
                                        on_delete=Callback::new({
                                            let id = note_id.clone();
                                            move |_| delete_note(id.clone())
                                        })
                                    />
                                }
                            }
                        />
                    </div>
                </Show>
            </CardBody>

            // Note Editor Modal
            <Show when=move || show_editor.get()>
                <div class="fixed inset-0 bg-black/70 flex items-center justify-center z-50 p-4">
                    <div class="w-full max-w-2xl bg-zinc-900 rounded-xl border border-zinc-700 shadow-2xl overflow-hidden max-h-[90vh] flex flex-col">
                        // Editor header
                        <div class="flex items-center justify-between px-6 py-4 border-b border-zinc-700 bg-zinc-800/50">
                            <h3 class="text-lg font-bold text-zinc-100">
                                {move || if editing_note_id.get().is_some() { "Edit Note" } else { "New Note" }}
                            </h3>
                            <button
                                class="p-1 text-zinc-400 hover:text-white transition-colors rounded-lg hover:bg-zinc-700"
                                on:click=move |_| show_editor.set(false)
                            >
                                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"/>
                                </svg>
                            </button>
                        </div>

                        // Editor body
                        <div class="flex-1 overflow-y-auto p-6 space-y-4">
                            // Title
                            <div>
                                <label class="block text-sm font-medium text-zinc-400 mb-1">"Title"</label>
                                <input
                                    type="text"
                                    class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                                    placeholder="Note title..."
                                    prop:value=move || editor_title.get()
                                    on:input=move |ev| editor_title.set(event_target_value(&ev))
                                />
                            </div>

                            // Content
                            <div>
                                <label class="block text-sm font-medium text-zinc-400 mb-1">"Content"</label>
                                <textarea
                                    class="w-full h-40 px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none resize-none"
                                    placeholder="Write your notes here..."
                                    prop:value=move || editor_content.get()
                                    on:input=move |ev| editor_content.set(event_target_value(&ev))
                                />
                            </div>

                            // Category and Tags row
                            <div class="grid grid-cols-2 gap-4">
                                <div>
                                    <label class="block text-sm font-medium text-zinc-400 mb-1">"Category"</label>
                                    <select
                                        class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                                        on:change=move |ev| {
                                            let value = event_target_value(&ev);
                                            let category = NoteCategory::all()
                                                .into_iter()
                                                .find(|c| c.display() == value)
                                                .unwrap_or(NoteCategory::General);
                                            editor_category.set(category);
                                        }
                                    >
                                        {NoteCategory::all().into_iter().map(|cat| {
                                            let display = cat.display().to_string();
                                            let is_selected = move || editor_category.get() == cat;
                                            view! {
                                                <option value=display.clone() selected=is_selected>
                                                    {display}
                                                </option>
                                            }
                                        }).collect_view()}
                                    </select>
                                </div>
                                <div>
                                    <label class="block text-sm font-medium text-zinc-400 mb-1">"Tags (comma separated)"</label>
                                    <input
                                        type="text"
                                        class="w-full px-4 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                                        placeholder="tag1, tag2, tag3"
                                        prop:value=move || editor_tags.get()
                                        on:input=move |ev| editor_tags.set(event_target_value(&ev))
                                    />
                                </div>
                            </div>

                            // Options row
                            <div class="flex items-center gap-6">
                                <label class="flex items-center gap-2 cursor-pointer">
                                    <input
                                        type="checkbox"
                                        class="w-4 h-4 rounded bg-zinc-800 border-zinc-600 text-purple-600 focus:ring-purple-500"
                                        prop:checked=move || editor_is_pinned.get()
                                        on:change=move |ev| editor_is_pinned.set(event_target_checked(&ev))
                                    />
                                    <span class="text-sm text-zinc-300">"Pin note"</span>
                                </label>
                                <label class="flex items-center gap-2 cursor-pointer">
                                    <input
                                        type="checkbox"
                                        class="w-4 h-4 rounded bg-zinc-800 border-zinc-600 text-purple-600 focus:ring-purple-500"
                                        prop:checked=move || editor_is_private.get()
                                        on:change=move |ev| editor_is_private.set(event_target_checked(&ev))
                                    />
                                    <span class="text-sm text-zinc-300">"Private (GM only)"</span>
                                </label>
                            </div>

                            // AI Categorization
                            <div class="p-4 bg-zinc-800/50 rounded-lg border border-zinc-700/50">
                                <div class="flex items-center justify-between mb-3">
                                    <h4 class="text-sm font-medium text-zinc-300 flex items-center gap-2">
                                        <svg class="w-4 h-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"/>
                                        </svg>
                                        "AI Suggestions"
                                    </h4>
                                    <Button
                                        variant=ButtonVariant::Ghost
                                        class="px-3 py-1 bg-purple-600/20 text-purple-400 text-xs"
                                        on_click=request_ai_categorization
                                    >
                                        {move || if is_categorizing.get() { "Analyzing..." } else { "Get Suggestions" }}
                                    </Button>
                                </div>

                                <Show when=move || ai_suggestions.get().is_some()>
                                    {move || {
                                        let (category, tags) = ai_suggestions.get().unwrap_or((NoteCategory::General, vec![]));
                                        view! {
                                            <div class="space-y-2">
                                                <div class="flex items-center gap-2">
                                                    <span class="text-xs text-zinc-500">"Suggested category:"</span>
                                                    <span
                                                        class="text-xs px-2 py-0.5 rounded"
                                                        style:background-color=format!("{}20", category.color())
                                                        style:color=category.color()
                                                    >
                                                        {category.display()}
                                                    </span>
                                                </div>
                                                {if !tags.is_empty() {
                                                    Some(view! {
                                                        <div class="flex items-center gap-2">
                                                            <span class="text-xs text-zinc-500">"Suggested tags:"</span>
                                                            {tags.iter().map(|tag| view! {
                                                                <span class="text-xs px-2 py-0.5 bg-zinc-700 text-zinc-300 rounded">
                                                                    {tag.clone()}
                                                                </span>
                                                            }).collect_view()}
                                                        </div>
                                                    })
                                                } else {
                                                    None
                                                }}
                                                <Button
                                                    variant=ButtonVariant::Secondary
                                                    class="w-full mt-2 py-1 text-xs"
                                                    on_click=apply_ai_suggestions
                                                >
                                                    "Apply Suggestions"
                                                </Button>
                                            </div>
                                        }
                                    }}
                                </Show>
                            </div>
                        </div>

                        // Editor footer
                        <div class="flex justify-end gap-2 px-6 py-4 border-t border-zinc-700 bg-zinc-800/30">
                            <Button
                                variant=ButtonVariant::Ghost
                                class="px-4 py-2 bg-zinc-700 text-zinc-300"
                                on_click=move |_: ev::MouseEvent| show_editor.set(false)
                            >
                                "Cancel"
                            </Button>
                            <Button
                                variant=ButtonVariant::Primary
                                class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white font-medium"
                                on_click=save_note
                            >
                                "Save Note"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>
        </Card>
    }
}

// ============================================================================
// Note Card Component
// ============================================================================

/// Individual note card
#[component]
fn NoteCard(
    note: SessionNote,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
) -> impl IntoView {
    let category_color = note.category.color();
    let is_expanded = RwSignal::new(false);

    view! {
        <div class="p-4 hover:bg-zinc-800/30 transition-colors">
            // Header row
            <div class="flex items-start justify-between gap-2 mb-2">
                <div class="flex items-center gap-2">
                    {if note.is_pinned {
                        Some(view! {
                            <svg class="w-4 h-4 text-yellow-400" fill="currentColor" viewBox="0 0 24 24">
                                <path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"/>
                                <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
                            </svg>
                        })
                    } else {
                        None
                    }}
                    <span
                        class="text-xs font-medium px-2 py-0.5 rounded"
                        style:background-color=format!("{}20", category_color)
                        style:color=category_color
                    >
                        {note.category.display().to_string()}
                    </span>
                    {if note.is_private {
                        Some(view! {
                            <span class="text-xs px-2 py-0.5 bg-purple-900/30 text-purple-400 rounded">
                                "Private"
                            </span>
                        })
                    } else {
                        None
                    }}
                </div>
                <span class="text-xs text-zinc-500">
                    {note.updated_at.clone()}
                </span>
            </div>

            // Title
            <h4
                class="font-medium text-zinc-200 mb-1 cursor-pointer hover:text-white"
                on:click=move |_| is_expanded.update(|v| *v = !*v)
            >
                {note.title.clone()}
            </h4>

            // Content preview or full
            <p class=move || format!(
                "text-sm text-zinc-400 {}",
                if is_expanded.get() { "" } else { "line-clamp-2" }
            )>
                {note.content.clone()}
            </p>

            // Tags
            {if !note.tags.is_empty() {
                Some(view! {
                    <div class="flex flex-wrap gap-1 mt-2">
                        {note.tags.iter().map(|tag| view! {
                            <span class="text-xs px-2 py-0.5 bg-zinc-800 text-zinc-400 rounded">
                                {format!("#{}", tag)}
                            </span>
                        }).collect_view()}
                    </div>
                })
            } else {
                None
            }}

            // Actions (shown on expand)
            <Show when=move || is_expanded.get()>
                <div class="flex justify-end gap-2 mt-3 pt-3 border-t border-zinc-700/50">
                    <button
                        class="px-3 py-1 text-xs text-zinc-400 hover:text-white transition-colors"
                        on:click=move |_| on_edit.run(())
                    >
                        "Edit"
                    </button>
                    <button
                        class="px-3 py-1 text-xs text-red-400 hover:text-red-300 transition-colors"
                        on:click=move |_| on_delete.run(())
                    >
                        "Delete"
                    </button>
                </div>
            </Show>
        </div>
    }
}
