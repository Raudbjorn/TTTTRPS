//! Voice Profile Manager Component (TASK-004)
//!
//! Provides a UI for browsing, filtering, and selecting voice profiles.
//! Includes preset DM personas and support for NPC-to-profile linking.

use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::bindings::{
    list_voice_presets, list_voice_presets_by_tag, search_voice_profiles,
    get_voice_profiles_by_gender, get_voice_profiles_by_age,
    get_npc_voice_profile, link_voice_profile_to_npc,
    VoiceProfile, AgeRange, Gender,
};
use crate::components::design_system::{Badge, BadgeVariant, Input};

// ============================================================================
// Voice Profile Card
// ============================================================================

/// Display a single voice profile as a card
#[component]
pub fn VoiceProfileCard(
    #[prop(into)] profile: VoiceProfile,
    #[prop(optional)] on_select: Option<Callback<VoiceProfile>>,
    #[prop(default = false)] is_selected: bool,
) -> impl IntoView {
    let profile_clone = profile.clone();
    let profile_for_click = profile.clone();

    let handle_click = move |_| {
        if let Some(ref callback) = on_select {
            callback.run(profile_for_click.clone());
        }
    };

    let gender_badge_variant = match profile.metadata.gender {
        Gender::Male => BadgeVariant::Info,
        Gender::Female => BadgeVariant::Warning,
        Gender::Neutral => BadgeVariant::Default,
        Gender::NonBinary => BadgeVariant::Success,
    };

    let age_badge_variant = match profile.metadata.age_range {
        AgeRange::Child => BadgeVariant::Info,
        AgeRange::YoungAdult => BadgeVariant::Success,
        AgeRange::Adult => BadgeVariant::Default,
        AgeRange::MiddleAged => BadgeVariant::Warning,
        AgeRange::Elderly => BadgeVariant::Danger,
    };

    let selected_class = if is_selected {
        "ring-2 ring-purple-500 bg-zinc-800"
    } else {
        "hover:bg-zinc-800/50"
    };

    view! {
        <div
            class=format!("cursor-pointer rounded-lg border border-zinc-700 p-4 transition-all duration-200 bg-zinc-900 {}", selected_class)
            on:click=handle_click
        >
            <div class="flex items-start justify-between mb-2">
                <h3 class="font-semibold text-zinc-100">
                    {profile_clone.name.clone()}
                </h3>
                {if profile_clone.is_preset {
                    view! {
                        <Badge variant=BadgeVariant::Info>
                            "Preset"
                        </Badge>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>

            <div class="flex flex-wrap gap-2 mb-3">
                <Badge variant=gender_badge_variant>
                    {profile_clone.metadata.gender.display_name()}
                </Badge>
                <Badge variant=age_badge_variant>
                    {profile_clone.metadata.age_range.display_name()}
                </Badge>
            </div>

            {if let Some(ref desc) = profile_clone.metadata.description {
                view! {
                    <p class="text-sm text-zinc-400 mb-3 line-clamp-2">
                        {desc.clone()}
                    </p>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}

            <div class="flex flex-wrap gap-1">
                {profile_clone.metadata.personality_traits.iter().take(4).map(|trait_name| {
                    view! {
                        <span class="text-xs px-2 py-0.5 rounded-full bg-zinc-700 text-zinc-300">
                            {trait_name.clone()}
                        </span>
                    }
                }).collect_view()}
                {if profile_clone.metadata.personality_traits.len() > 4 {
                    view! {
                        <span class="text-xs text-zinc-500">
                            {format!("+{} more", profile_clone.metadata.personality_traits.len() - 4)}
                        </span>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
            </div>

            <div class="mt-3 flex flex-wrap gap-1">
                {profile_clone.metadata.tags.iter().map(|tag| {
                    view! {
                        <span class="text-xs px-2 py-0.5 rounded bg-purple-900/30 text-purple-300">
                            {"#"}{tag.clone()}
                        </span>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

// ============================================================================
// Voice Profile Grid
// ============================================================================

/// Display a grid of voice profiles
#[component]
pub fn VoiceProfileGrid(
    #[prop(into)] profiles: Signal<Vec<VoiceProfile>>,
    #[prop(optional)] on_select: Option<Callback<VoiceProfile>>,
    #[prop(optional)] selected_id: Option<Signal<Option<String>>>,
) -> impl IntoView {
    view! {
        <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {move || {
                let current_profiles = profiles.get();
                let selected = selected_id.map(|s| s.get()).flatten();

                if current_profiles.is_empty() {
                    view! {
                        <div class="col-span-full text-center py-8 text-zinc-500">
                            "No voice profiles found matching your criteria."
                        </div>
                    }.into_any()
                } else {
                    current_profiles.into_iter().map(|profile| {
                        let is_selected = selected.as_ref().map(|s| s == &profile.id).unwrap_or(false);
                        view! {
                            <VoiceProfileCard
                                profile=profile
                                on_select=on_select.clone()
                                is_selected=is_selected
                            />
                        }
                    }).collect_view().into_any()
                }
            }}
        </div>
    }
}

// ============================================================================
// Voice Profile Filters
// ============================================================================

/// Filter controls for voice profiles
#[component]
pub fn VoiceProfileFilters(
    search_query: RwSignal<String>,
    selected_gender: RwSignal<Option<Gender>>,
    selected_age: RwSignal<Option<AgeRange>>,
    selected_tag: RwSignal<Option<String>>,
    on_filter_change: Callback<()>,
) -> impl IntoView {
    let gender_value = RwSignal::new("".to_string());
    let age_value = RwSignal::new("".to_string());
    let tag_value = RwSignal::new("".to_string());

    // Handle gender change
    let on_filter_change_gender = on_filter_change.clone();
    Effect::new(move |_| {
        let val = gender_value.get();
        let gender = match val.as_str() {
            "male" => Some(Gender::Male),
            "female" => Some(Gender::Female),
            "neutral" => Some(Gender::Neutral),
            "nonbinary" => Some(Gender::NonBinary),
            _ => None,
        };
        selected_gender.set(gender);
        on_filter_change_gender.run(());
    });

    // Handle age change
    let on_filter_change_age = on_filter_change.clone();
    Effect::new(move |_| {
        let val = age_value.get();
        let age = match val.as_str() {
            "child" => Some(AgeRange::Child),
            "young_adult" => Some(AgeRange::YoungAdult),
            "adult" => Some(AgeRange::Adult),
            "middle_aged" => Some(AgeRange::MiddleAged),
            "elderly" => Some(AgeRange::Elderly),
            _ => None,
        };
        selected_age.set(age);
        on_filter_change_age.run(());
    });

    // Handle tag change
    let on_filter_change_tag = on_filter_change.clone();
    Effect::new(move |_| {
        let val = tag_value.get();
        let tag = if val.is_empty() { None } else { Some(val) };
        selected_tag.set(tag);
        on_filter_change_tag.run(());
    });

    view! {
        <div class="flex flex-wrap gap-4 mb-6">
            <div class="flex-1 min-w-[200px]">
                <label class="block text-sm text-zinc-400 mb-1">"Search"</label>
                <Input
                    value=search_query
                    placeholder="Search profiles..."
                />
            </div>

            <div class="w-40">
                <label class="block text-sm text-zinc-400 mb-1">"Gender"</label>
                <select
                    class="w-full bg-zinc-800 border border-zinc-700 rounded p-2 text-white focus:outline-none focus:ring-2 focus:ring-purple-500/50"
                    prop:value=move || gender_value.get()
                    on:change=move |ev| gender_value.set(event_target_value(&ev))
                >
                    <option value="">"All Genders"</option>
                    <option value="male">"Male"</option>
                    <option value="female">"Female"</option>
                    <option value="neutral">"Neutral"</option>
                    <option value="nonbinary">"Non-Binary"</option>
                </select>
            </div>

            <div class="w-40">
                <label class="block text-sm text-zinc-400 mb-1">"Age Range"</label>
                <select
                    class="w-full bg-zinc-800 border border-zinc-700 rounded p-2 text-white focus:outline-none focus:ring-2 focus:ring-purple-500/50"
                    prop:value=move || age_value.get()
                    on:change=move |ev| age_value.set(event_target_value(&ev))
                >
                    <option value="">"All Ages"</option>
                    <option value="child">"Child"</option>
                    <option value="young_adult">"Young Adult"</option>
                    <option value="adult">"Adult"</option>
                    <option value="middle_aged">"Middle-Aged"</option>
                    <option value="elderly">"Elderly"</option>
                </select>
            </div>

            <div class="w-40">
                <label class="block text-sm text-zinc-400 mb-1">"Category"</label>
                <select
                    class="w-full bg-zinc-800 border border-zinc-700 rounded p-2 text-white focus:outline-none focus:ring-2 focus:ring-purple-500/50"
                    prop:value=move || tag_value.get()
                    on:change=move |ev| tag_value.set(event_target_value(&ev))
                >
                    <option value="">"All Tags"</option>
                    <option value="fantasy">"Fantasy"</option>
                    <option value="horror">"Horror"</option>
                    <option value="scifi">"Sci-Fi"</option>
                    <option value="narrator">"Narrator"</option>
                    <option value="heroic">"Heroic"</option>
                    <option value="villain">"Villain"</option>
                    <option value="mystical">"Mystical"</option>
                </select>
            </div>
        </div>
    }
}

// ============================================================================
// Voice Profile Manager
// ============================================================================

/// Main voice profile manager component
#[component]
pub fn VoiceProfileManager(
    #[prop(optional)] npc_id: Option<String>,
    #[prop(optional)] on_profile_selected: Option<Callback<VoiceProfile>>,
) -> impl IntoView {
    // State signals
    let profiles = RwSignal::new(Vec::<VoiceProfile>::new());
    let is_loading = RwSignal::new(true);
    let error_message = RwSignal::new(Option::<String>::None);
    let selected_profile_id = RwSignal::new(Option::<String>::None);

    // Filter signals
    let search_query = RwSignal::new(String::new());
    let selected_gender = RwSignal::new(Option::<Gender>::None);
    let selected_age = RwSignal::new(Option::<AgeRange>::None);
    let selected_tag = RwSignal::new(Option::<String>::None);

    // NPC linking state
    let npc_profile_id = RwSignal::new(Option::<String>::None);
    let is_linking = RwSignal::new(false);
    let link_status = RwSignal::new(String::new());

    // Load profiles function
    let load_profiles = move || {
        is_loading.set(true);
        error_message.set(None);

        spawn_local(async move {
            let query = search_query.get();
            let gender = selected_gender.get();
            let age = selected_age.get();
            let tag = selected_tag.get();

            let result = if !query.is_empty() {
                // Search by query
                search_voice_profiles(query).await
            } else if let Some(g) = gender {
                // Filter by gender
                get_voice_profiles_by_gender(g.display_name().to_lowercase()).await
            } else if let Some(a) = age {
                // Filter by age
                let age_str = match a {
                    AgeRange::Child => "child",
                    AgeRange::YoungAdult => "young_adult",
                    AgeRange::Adult => "adult",
                    AgeRange::MiddleAged => "middle_aged",
                    AgeRange::Elderly => "elderly",
                };
                get_voice_profiles_by_age(age_str.to_string()).await
            } else if let Some(t) = tag {
                // Filter by tag
                list_voice_presets_by_tag(t).await
            } else {
                // Load all presets
                list_voice_presets().await
            };

            match result {
                Ok(p) => {
                    profiles.set(p);
                }
                Err(e) => {
                    error_message.set(Some(format!("Failed to load profiles: {}", e)));
                }
            }

            is_loading.set(false);
        });
    };

    // Load NPC's current profile if npc_id is provided
    let npc_id_clone = npc_id.clone();
    Effect::new(move |_| {
        if let Some(ref id) = npc_id_clone {
            let id = id.clone();
            spawn_local(async move {
                if let Ok(Some(profile_id)) = get_npc_voice_profile(id).await {
                    npc_profile_id.set(Some(profile_id.clone()));
                    selected_profile_id.set(Some(profile_id));
                }
            });
        }
    });

    // Initial load
    Effect::new(move |_| {
        load_profiles();
    });

    // Handle profile selection
    let npc_id_for_select = npc_id.clone();
    let on_profile_selected_clone = on_profile_selected.clone();
    let on_select = Callback::new(move |profile: VoiceProfile| {
        selected_profile_id.set(Some(profile.id.clone()));

        // If linking to NPC
        if let Some(ref npc) = npc_id_for_select {
            let npc = npc.clone();
            let profile_id = profile.id.clone();
            is_linking.set(true);
            link_status.set(String::new());

            spawn_local(async move {
                match link_voice_profile_to_npc(profile_id, npc).await {
                    Ok(()) => {
                        link_status.set("Profile linked successfully!".to_string());
                    }
                    Err(e) => {
                        link_status.set(format!("Failed to link profile: {}", e));
                    }
                }
                is_linking.set(false);
            });
        }

        // Call external callback if provided
        if let Some(ref callback) = on_profile_selected_clone {
            callback.run(profile);
        }
    });

    // Filter change handler
    let on_filter_change = Callback::new(move |_: ()| {
        load_profiles();
    });

    // Debounced search
    Effect::new(move |_| {
        let _query = search_query.get();
        // Trigger reload on search change
        load_profiles();
    });

    view! {
        <div class="space-y-6">
            // Header
            <div class="flex items-center justify-between">
                <div>
                    <h2 class="text-xl font-bold text-zinc-100">
                        "Voice Profiles"
                    </h2>
                    <p class="text-sm text-zinc-400">
                        {if npc_id.is_some() {
                            "Select a voice profile for this NPC"
                        } else {
                            "Browse and manage DM voice personas"
                        }}
                    </p>
                </div>

                {move || {
                    let count = profiles.get().len();
                    view! {
                        <Badge variant=BadgeVariant::Info>
                            {format!("{} profiles", count)}
                        </Badge>
                    }
                }}
            </div>

            // Link status message
            {move || {
                let status = link_status.get();
                if !status.is_empty() {
                    let is_error = status.starts_with("Failed");
                    let class = if is_error {
                        "bg-red-900/30 text-red-300"
                    } else {
                        "bg-green-900/30 text-green-300"
                    };
                    view! {
                        <div class=format!("rounded-lg p-3 text-sm {}", class)>
                            {status}
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Filters
            <VoiceProfileFilters
                search_query=search_query
                selected_gender=selected_gender
                selected_age=selected_age
                selected_tag=selected_tag
                on_filter_change=on_filter_change
            />

            // Error message
            {move || {
                if let Some(err) = error_message.get() {
                    view! {
                        <div class="rounded-lg bg-red-900/30 text-red-300 p-4">
                            {err}
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Loading state
            {move || {
                if is_loading.get() {
                    view! {
                        <div class="flex items-center justify-center py-12">
                            <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-purple-500"></div>
                            <span class="ml-3 text-zinc-400">
                                "Loading profiles..."
                            </span>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Profile grid
            {move || {
                if !is_loading.get() {
                    view! {
                        <VoiceProfileGrid
                            profiles=profiles.into()
                            on_select=Some(on_select.clone())
                            selected_id=Some(selected_profile_id.into())
                        />
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}
        </div>
    }
}

// ============================================================================
// Profile Selector (Compact Version)
// ============================================================================

/// A compact profile selector for inline use
#[component]
pub fn VoiceProfileSelector(
    #[prop(into)] value: RwSignal<Option<String>>,
    #[prop(optional)] on_change: Option<Callback<Option<VoiceProfile>>>,
    #[prop(default = "Select Voice Profile".to_string())] placeholder: String,
) -> impl IntoView {
    let profiles = RwSignal::new(Vec::<VoiceProfile>::new());
    let is_open = RwSignal::new(false);
    let selected_profile = RwSignal::new(Option::<VoiceProfile>::None);

    // Load presets on mount
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(p) = list_voice_presets().await {
                // Find currently selected profile
                let current_id = value.get();
                if let Some(id) = current_id {
                    if let Some(profile) = p.iter().find(|pr| pr.id == id) {
                        selected_profile.set(Some(profile.clone()));
                    }
                }
                profiles.set(p);
            }
        });
    });

    let on_change_clone = on_change.clone();
    let handle_select = move |profile: VoiceProfile| {
        value.set(Some(profile.id.clone()));
        selected_profile.set(Some(profile.clone()));
        is_open.set(false);

        if let Some(ref callback) = on_change_clone {
            callback.run(Some(profile));
        }
    };

    let on_change_clear = on_change.clone();
    let handle_clear = move |_| {
        value.set(None);
        selected_profile.set(None);

        if let Some(ref callback) = on_change_clear {
            callback.run(None);
        }
    };

    view! {
        <div class="relative">
            // Selected value display / button
            <button
                type="button"
                class="w-full flex items-center justify-between px-3 py-2 rounded-lg border border-zinc-600 bg-zinc-800 text-left hover:border-purple-400 focus:outline-none focus:ring-2 focus:ring-purple-500"
                on:click=move |_| is_open.update(|v| *v = !*v)
            >
                {move || {
                    if let Some(profile) = selected_profile.get() {
                        view! {
                            <div class="flex items-center gap-2">
                                <span class="font-medium text-zinc-100">
                                    {profile.name}
                                </span>
                                <Badge variant=BadgeVariant::Default>
                                    {profile.metadata.gender.display_name()}
                                </Badge>
                            </div>
                        }.into_any()
                    } else {
                        view! {
                            <span class="text-zinc-500">{placeholder.clone()}</span>
                        }.into_any()
                    }
                }}

                <svg class="w-4 h-4 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                </svg>
            </button>

            // Clear button
            {move || {
                if selected_profile.get().is_some() {
                    view! {
                        <button
                            type="button"
                            class="absolute right-8 top-1/2 -translate-y-1/2 text-zinc-400 hover:text-zinc-200"
                            on:click=handle_clear
                        >
                            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                            </svg>
                        </button>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}

            // Dropdown
            {move || {
                if is_open.get() {
                    view! {
                        <div class="absolute z-50 mt-1 w-full max-h-60 overflow-auto rounded-lg border border-zinc-700 bg-zinc-800 shadow-lg">
                            {profiles.get().into_iter().map(|profile| {
                                let p = profile.clone();
                                let p2 = profile.clone();
                                let is_selected = value.get().as_ref() == Some(&profile.id);
                                let selected_class = if is_selected {
                                    "bg-purple-900/30"
                                } else {
                                    "hover:bg-zinc-700"
                                };

                                view! {
                                    <button
                                        type="button"
                                        class=format!("w-full px-3 py-2 text-left {} transition-colors", selected_class)
                                        on:click=move |_| handle_select(p.clone())
                                    >
                                        <div class="font-medium text-zinc-100">
                                            {p2.name.clone()}
                                        </div>
                                        <div class="text-xs text-zinc-500 flex gap-2 mt-0.5">
                                            <span>{p2.metadata.gender.display_name()}</span>
                                            <span>"-"</span>
                                            <span>{p2.metadata.age_range.display_name()}</span>
                                        </div>
                                    </button>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }
            }}
        </div>
    }
}
