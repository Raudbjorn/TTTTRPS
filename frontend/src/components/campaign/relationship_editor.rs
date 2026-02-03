//! Relationship Editor Component
//!
//! Modal/panel for creating and editing entity relationships.

use crate::bindings::{
    create_entity_relationship, delete_entity_relationship, get_relationships_for_entity,
    list_entity_relationships, update_entity_relationship, EntityRelationship, RelationshipSummary,
};
use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// Relationship type options
const RELATIONSHIP_TYPES: &[(&str, &str)] = &[
    ("ally", "Ally"),
    ("enemy", "Enemy"),
    ("friend", "Friend"),
    ("rival", "Rival"),
    ("family", "Family"),
    ("romantic", "Romantic"),
    ("business", "Business"),
    ("political", "Political"),
    ("mentor", "Mentor"),
    ("student", "Student"),
    ("memberof", "Member Of"),
    ("leaderof", "Leader Of"),
    ("servantof", "Servant Of"),
    ("employerof", "Employer Of"),
    ("locatedat", "Located At"),
    ("ownedby", "Owned By"),
    ("worships", "Worships"),
    ("createdby", "Created By"),
    ("partof", "Part Of"),
    ("custom", "Custom"),
];

/// Entity type options
const ENTITY_TYPES: &[(&str, &str)] = &[
    ("npc", "NPC"),
    ("pc", "Player Character"),
    ("location", "Location"),
    ("faction", "Faction"),
    ("item", "Item"),
    ("event", "Event"),
    ("quest", "Quest"),
    ("deity", "Deity"),
    ("creature", "Creature"),
    ("custom", "Custom"),
];

/// Relationship strength options
const STRENGTH_OPTIONS: &[(&str, &str)] = &[
    ("weak", "Weak"),
    ("moderate", "Moderate"),
    ("strong", "Strong"),
    ("unbreakable", "Unbreakable"),
];

/// Form field component
#[component]
fn FormField(
    #[prop(into)] label: String,
    #[prop(optional)] help: Option<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="space-y-1">
            <label class="block text-sm font-medium text-zinc-300">{label}</label>
            {children()}
            {help.map(|h| view! {
                <p class="text-xs text-zinc-500">{h}</p>
            })}
        </div>
    }
}

/// Entity picker component
#[component]
fn EntityPicker(
    #[prop(into)] label: String,
    entity_id: RwSignal<String>,
    entity_type: RwSignal<String>,
    entity_name: RwSignal<String>,
    #[prop(optional)] available_entities: Option<Vec<(String, String, String)>>, // (id, name, type)
) -> impl IntoView {
    let handle_type_change = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&evt);
        entity_type.set(target.value());
    };

    let handle_name_change = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&evt);
        entity_name.set(target.value());
    };

    let handle_entity_select = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&evt);
        let value = target.value();
        if !value.is_empty() {
            entity_id.set(value);
        }
    };

    view! {
        <div class="space-y-2 p-4 bg-zinc-800/50 rounded-lg">
            <div class="text-sm font-medium text-zinc-400 mb-2">{label}</div>

            // Entity type selector
            <div class="grid grid-cols-2 gap-2">
                <div>
                    <label class="block text-xs text-zinc-500 mb-1">"Type"</label>
                    <select
                        class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none"
                        on:change=handle_type_change
                    >
                        {ENTITY_TYPES.iter().map(|(value, label)| {
                            view! {
                                <option value=*value selected=move || entity_type.get() == *value>
                                    {*label}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>

                <div>
                    <label class="block text-xs text-zinc-500 mb-1">"Name"</label>
                    <input
                        type="text"
                        placeholder="Entity name..."
                        class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                        prop:value=move || entity_name.get()
                        on:input=handle_name_change
                    />
                </div>
            </div>

            // Quick select from available entities
            {available_entities.map(|entities| {
                let filtered = entities.clone();
                view! {
                    <div>
                        <label class="block text-xs text-zinc-500 mb-1">"Or select existing"</label>
                        <select
                            class="w-full px-3 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white text-sm focus:border-purple-500 focus:outline-none"
                            on:change=handle_entity_select
                        >
                            <option value="">"Choose..."</option>
                            {filtered.into_iter().map(|(id, name, etype)| {
                                view! {
                                    <option value=id.clone()>
                                        {format!("{} ({})", name, etype)}
                                    </option>
                                }
                            }).collect_view()}
                        </select>
                    </div>
                }
            })}
        </div>
    }
}

/// Relationship summary card
#[component]
fn RelationshipCard(
    relationship: RelationshipSummary,
    on_edit: Callback<String>,
    on_delete: Callback<String>,
) -> impl IntoView {
    let rel_id = relationship.id.clone();
    let rel_id_delete = rel_id.clone();

    let type_color = match relationship.relationship_type.as_str() {
        "ally" | "friend" => "bg-emerald-900/50 text-emerald-300",
        "enemy" | "rival" => "bg-red-900/50 text-red-300",
        "family" => "bg-violet-900/50 text-violet-300",
        "romantic" => "bg-pink-900/50 text-pink-300",
        "business" => "bg-amber-900/50 text-amber-300",
        "political" => "bg-blue-900/50 text-blue-300",
        _ => "bg-zinc-700 text-zinc-300",
    };

    let handle_edit = move |_: ev::MouseEvent| {
        on_edit.run(rel_id.clone());
    };

    let handle_delete = move |_: ev::MouseEvent| {
        on_delete.run(rel_id_delete.clone());
    };

    view! {
        <div class="bg-zinc-800 border border-zinc-700 rounded-lg p-4 hover:border-zinc-600 transition-colors">
            <div class="flex items-start justify-between">
                <div class="flex-1">
                    // Source -> Target
                    <div class="flex items-center gap-2 mb-2">
                        <span class="text-sm font-medium text-white">{relationship.source_name}</span>
                        <span class="text-zinc-500">"->"</span>
                        <span class="text-sm font-medium text-white">{relationship.target_name}</span>
                    </div>

                    // Type and strength
                    <div class="flex items-center gap-2">
                        <span class=format!("px-2 py-0.5 text-xs rounded-full {}", type_color)>
                            {relationship.relationship_type}
                        </span>
                        <span class="text-xs text-zinc-500">{relationship.strength}</span>
                        {(!relationship.is_active).then(|| view! {
                            <span class="px-2 py-0.5 text-xs bg-zinc-700 text-zinc-400 rounded-full">"Inactive"</span>
                        })}
                    </div>

                    // Entity types
                    <div class="mt-2 text-xs text-zinc-500">
                        {format!("{} - {}", relationship.source_type, relationship.target_type)}
                    </div>
                </div>

                // Actions
                <div class="flex gap-1">
                    <button
                        class="p-2 text-zinc-500 hover:text-white transition-colors"
                        on:click=handle_edit
                    >
                        "Edit"
                    </button>
                    <button
                        class="p-2 text-zinc-500 hover:text-red-400 transition-colors"
                        on:click=handle_delete
                    >
                        "X"
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Create/Edit relationship modal
#[component]
fn RelationshipModal(
    campaign_id: String,
    existing: Option<EntityRelationship>,
    on_save: Callback<EntityRelationship>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let is_edit = existing.is_some();
    let title = if is_edit {
        "Edit Relationship"
    } else {
        "Create Relationship"
    };

    // Form state
    let source_id = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.source_id.clone())
            .unwrap_or_default(),
    );
    let source_type = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.source_type.clone())
            .unwrap_or_else(|| "npc".to_string()),
    );
    let source_name = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.source_name.clone())
            .unwrap_or_default(),
    );

    let target_id = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.target_id.clone())
            .unwrap_or_default(),
    );
    let target_type = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.target_type.clone())
            .unwrap_or_else(|| "npc".to_string()),
    );
    let target_name = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.target_name.clone())
            .unwrap_or_default(),
    );

    let relationship_type = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.relationship_type.clone())
            .unwrap_or_else(|| "ally".to_string()),
    );
    let strength = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.strength.clone())
            .unwrap_or_else(|| "moderate".to_string()),
    );
    let description = RwSignal::new(
        existing
            .as_ref()
            .map(|e| e.description.clone())
            .unwrap_or_default(),
    );
    let is_active = RwSignal::new(existing.as_ref().map(|e| e.is_active).unwrap_or(true));
    let is_known = RwSignal::new(existing.as_ref().map(|e| e.is_known).unwrap_or(true));

    let is_saving = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let existing_id = existing.as_ref().map(|e| e.id.clone());

    let handle_save = {
        let campaign_id = campaign_id.clone();
        move |_: ev::MouseEvent| {
            // Validate
            if source_name.get().is_empty() || target_name.get().is_empty() {
                error.set(Some("Source and target names are required".to_string()));
                return;
            }

            is_saving.set(true);
            error.set(None);

            let cid = campaign_id.clone();
            let existing_id = existing_id.clone();
            let on_save = on_save.clone();

            spawn_local(async move {
                let result = if let Some(rel_id) = existing_id {
                    // Update existing
                    let rel = EntityRelationship {
                        id: rel_id,
                        campaign_id: cid.clone(),
                        source_id: source_id.get(),
                        source_type: source_type.get(),
                        source_name: source_name.get(),
                        target_id: target_id.get(),
                        target_type: target_type.get(),
                        target_name: target_name.get(),
                        relationship_type: relationship_type.get(),
                        strength: strength.get(),
                        is_active: is_active.get(),
                        is_known: is_known.get(),
                        description: description.get(),
                        started_at: None,
                        ended_at: None,
                        tags: vec![],
                        metadata: std::collections::HashMap::new(),
                        created_at: String::new(),
                        updated_at: String::new(),
                    };
                    update_entity_relationship(rel.clone()).await.map(|_| rel)
                } else {
                    // Create new
                    create_entity_relationship(
                        cid,
                        source_id.get(),
                        source_type.get(),
                        source_name.get(),
                        target_id.get(),
                        target_type.get(),
                        target_name.get(),
                        relationship_type.get(),
                        Some(strength.get()),
                        Some(description.get()),
                    )
                    .await
                };

                match result {
                    Ok(rel) => {
                        on_save.run(rel);
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }

                is_saving.set(false);
            });
        }
    };

    let handle_cancel = move |_: ev::MouseEvent| {
        on_cancel.run(());
    };

    view! {
        <div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
            <div class="bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl w-full max-w-2xl max-h-[90vh] overflow-y-auto">
                // Header
                <div class="flex items-center justify-between px-6 py-4 border-b border-zinc-800">
                    <h2 class="text-lg font-bold text-white">{title}</h2>
                    <button
                        class="p-2 text-zinc-400 hover:text-white transition-colors"
                        on:click=handle_cancel.clone()
                    >
                        "X"
                    </button>
                </div>

                // Content
                <div class="p-6 space-y-6">
                    // Error
                    {move || error.get().map(|e| view! {
                        <div class="p-4 bg-red-900/20 border border-red-800 rounded-lg text-red-400 text-sm">
                            {e}
                        </div>
                    })}

                    // Source Entity
                    <EntityPicker
                        label="Source Entity"
                        entity_id=source_id
                        entity_type=source_type
                        entity_name=source_name
                    />

                    // Relationship Type
                    <div class="grid grid-cols-2 gap-4">
                        <FormField label="Relationship Type">
                            <select
                                class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                                on:change=move |evt| {
                                    let target = event_target::<web_sys::HtmlSelectElement>(&evt);
                                    relationship_type.set(target.value());
                                }
                            >
                                {RELATIONSHIP_TYPES.iter().map(|(value, label)| {
                                    view! {
                                        <option value=*value selected=move || relationship_type.get() == *value>
                                            {*label}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                        </FormField>

                        <FormField label="Strength">
                            <select
                                class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                                on:change=move |evt| {
                                    let target = event_target::<web_sys::HtmlSelectElement>(&evt);
                                    strength.set(target.value());
                                }
                            >
                                {STRENGTH_OPTIONS.iter().map(|(value, label)| {
                                    view! {
                                        <option value=*value selected=move || strength.get() == *value>
                                            {*label}
                                        </option>
                                    }
                                }).collect_view()}
                            </select>
                        </FormField>
                    </div>

                    // Target Entity
                    <EntityPicker
                        label="Target Entity"
                        entity_id=target_id
                        entity_type=target_type
                        entity_name=target_name
                    />

                    // Description
                    <FormField label="Description" help="Optional notes about this relationship".to_string()>
                        <textarea
                            rows=3
                            class="w-full px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none resize-none"
                            placeholder="Describe the nature of this relationship..."
                            prop:value=move || description.get()
                            on:input=move |evt| {
                                let target = event_target::<web_sys::HtmlTextAreaElement>(&evt);
                                description.set(target.value());
                            }
                        />
                    </FormField>

                    // Flags
                    <div class="flex gap-6">
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                class="w-4 h-4 rounded border-zinc-600 bg-zinc-800 text-purple-600 focus:ring-purple-500 focus:ring-offset-0"
                                prop:checked=move || is_active.get()
                                on:change=move |evt| {
                                    let target = event_target::<web_sys::HtmlInputElement>(&evt);
                                    is_active.set(target.checked());
                                }
                            />
                            <span class="text-sm text-zinc-300">"Active relationship"</span>
                        </label>

                        <label class="flex items-center gap-2 cursor-pointer">
                            <input
                                type="checkbox"
                                class="w-4 h-4 rounded border-zinc-600 bg-zinc-800 text-purple-600 focus:ring-purple-500 focus:ring-offset-0"
                                prop:checked=move || is_known.get()
                                on:change=move |evt| {
                                    let target = event_target::<web_sys::HtmlInputElement>(&evt);
                                    is_known.set(target.checked());
                                }
                            />
                            <span class="text-sm text-zinc-300">"Known to players"</span>
                        </label>
                    </div>
                </div>

                // Footer
                <div class="flex justify-end gap-3 px-6 py-4 border-t border-zinc-800">
                    <button
                        class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                        on:click=handle_cancel
                    >
                        "Cancel"
                    </button>
                    <button
                        class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors disabled:opacity-50"
                        disabled=move || is_saving.get()
                        on:click=handle_save
                    >
                        {move || if is_saving.get() { "Saving..." } else { "Save Relationship" }}
                    </button>
                </div>
            </div>
        </div>
    }
}

/// Main relationship editor component
#[component]
pub fn RelationshipEditor(
    /// Campaign ID
    campaign_id: String,
    /// Optional entity to filter by
    #[prop(optional)]
    entity_id: Option<String>,
    /// Callback when relationship is created/updated
    #[prop(optional)]
    _on_change: Option<Callback<()>>,
) -> impl IntoView {
    let relationships = RwSignal::new(Vec::<RelationshipSummary>::new());
    let is_loading = RwSignal::new(true);
    let show_modal = RwSignal::new(false);
    let editing_relationship = RwSignal::new(Option::<EntityRelationship>::None);
    let filter_type = RwSignal::new(String::new());
    let search_query = RwSignal::new(String::new());

    // Load relationships
    let campaign_id_load = campaign_id.clone();
    let entity_id_load = entity_id.clone();
    let load_relationships = move || {
        let cid = campaign_id_load.clone();
        let eid = entity_id_load.clone();
        spawn_local(async move {
            is_loading.set(true);

            let result = if let Some(entity_id) = eid {
                get_relationships_for_entity(cid, entity_id)
                    .await
                    .map(|rels| {
                        rels.into_iter()
                            .map(|r| RelationshipSummary {
                                id: r.id,
                                source_id: r.source_id,
                                source_name: r.source_name,
                                source_type: r.source_type,
                                target_id: r.target_id,
                                target_name: r.target_name,
                                target_type: r.target_type,
                                relationship_type: r.relationship_type,
                                strength: r.strength,
                                is_active: r.is_active,
                            })
                            .collect()
                    })
            } else {
                list_entity_relationships(cid).await
            };

            if let Ok(rels) = result {
                relationships.set(rels);
            }

            is_loading.set(false);
        });
    };

    Effect::new({
        let load = load_relationships.clone();
        move || {
            load();
        }
    });

    let handle_create = move |_: ev::MouseEvent| {
        editing_relationship.set(None);
        show_modal.set(true);
    };

    let _campaign_id_edit = campaign_id.clone();
    let handle_edit = Callback::new(move |_rel_id: String| {
        // For simplicity, we'll just open the modal without loading the full relationship
        // In a real app, you'd fetch the full EntityRelationship here
        editing_relationship.set(None);
        show_modal.set(true);
    });

    let campaign_id_delete = campaign_id.clone();
    let load_after_delete = load_relationships.clone();
    let handle_delete = Callback::new(move |rel_id: String| {
        let cid = campaign_id_delete.clone();
        let load = load_after_delete.clone();
        spawn_local(async move {
            if delete_entity_relationship(cid, rel_id).await.is_ok() {
                load();
            }
        });
    });

    let campaign_id_modal = campaign_id.clone();
    let load_after_save = load_relationships.clone();
    let handle_save = Callback::new(move |_rel: EntityRelationship| {
        show_modal.set(false);
        editing_relationship.set(None);
        load_after_save();
    });

    let handle_cancel = Callback::new(move |_: ()| {
        show_modal.set(false);
        editing_relationship.set(None);
    });

    let handle_search = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlInputElement>(&evt);
        search_query.set(target.value());
    };

    let handle_filter = move |evt: ev::Event| {
        let target = event_target::<web_sys::HtmlSelectElement>(&evt);
        filter_type.set(target.value());
    };

    view! {
        <div class="space-y-4">
            // Header
            <div class="flex flex-col md:flex-row md:items-center gap-4">
                // Search
                <div class="flex-1">
                    <input
                        type="text"
                        placeholder="Search relationships..."
                        class="w-full px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-white placeholder-zinc-500 focus:border-purple-500 focus:outline-none"
                        on:input=handle_search
                    />
                </div>

                // Filter
                <select
                    class="px-4 py-2 bg-zinc-900 border border-zinc-800 rounded-lg text-white focus:border-purple-500 focus:outline-none"
                    on:change=handle_filter
                >
                    <option value="">"All Types"</option>
                    {RELATIONSHIP_TYPES.iter().map(|(value, label)| {
                        view! {
                            <option value=*value>{*label}</option>
                        }
                    }).collect_view()}
                </select>

                // Create button
                <button
                    class="px-4 py-2 bg-purple-600 hover:bg-purple-500 text-white rounded-lg transition-colors whitespace-nowrap"
                    on:click=handle_create
                >
                    "+ Add Relationship"
                </button>
            </div>

            // Relationships list
            {move || {
                if is_loading.get() {
                    view! {
                        <div class="text-center py-12 text-zinc-500">"Loading relationships..."</div>
                    }.into_any()
                } else {
                    let query = search_query.get().to_lowercase();
                    let rel_type = filter_type.get();

                    let filtered: Vec<_> = relationships.get()
                        .into_iter()
                        .filter(|r| {
                            (query.is_empty() ||
                                r.source_name.to_lowercase().contains(&query) ||
                                r.target_name.to_lowercase().contains(&query)) &&
                            (rel_type.is_empty() || r.relationship_type == rel_type)
                        })
                        .collect();

                    if filtered.is_empty() {
                        view! {
                            <div class="text-center py-12">
                                <div class="text-zinc-500 mb-4">"No relationships found"</div>
                                <button
                                    class="px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                                    on:click=handle_create.clone()
                                >
                                    "Create First Relationship"
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <div class="space-y-3">
                                {filtered.into_iter().map(|rel| {
                                    view! {
                                        <RelationshipCard
                                            relationship=rel
                                            on_edit=handle_edit
                                            on_delete=handle_delete
                                        />
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                }
            }}

            // Modal
            {move || show_modal.get().then(|| view! {
                <RelationshipModal
                    campaign_id=campaign_id_modal.clone()
                    existing=editing_relationship.get()
                    on_save=handle_save
                    on_cancel=handle_cancel
                />
            })}
        </div>
    }
}
