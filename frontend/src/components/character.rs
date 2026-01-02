//! Character Creator Component (TASK-018: Multi-System Character Generation)
//!
//! A comprehensive form for generating RPG characters with support for 10+ game systems.
//! Features system-specific options, class/race selection, and equipment generation.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::bindings::{
    generate_character_advanced, get_supported_systems, list_system_info,
    get_game_system_info, Character, CharacterAttributeValue, CharacterTrait,
    CharacterEquipment, CharacterBackground, GenerationOptions, GameSystemInfo,
};
use crate::components::design_system::{Button, ButtonVariant, Card, CardBody, CardHeader, Input};

/// Character Creator page component with multi-system support
#[component]
pub fn CharacterCreator() -> impl IntoView {
    // System selection state
    let systems: RwSignal<Vec<GameSystemInfo>> = RwSignal::new(Vec::new());
    let selected_system: RwSignal<Option<GameSystemInfo>> = RwSignal::new(None);
    let selected_system_id: RwSignal<String> = RwSignal::new("dnd5e".to_string());

    // Character options state
    let character_name: RwSignal<String> = RwSignal::new(String::new());
    let character_concept: RwSignal<String> = RwSignal::new(String::new());
    let selected_race: RwSignal<String> = RwSignal::new(String::new());
    let selected_class: RwSignal<String> = RwSignal::new(String::new());
    let selected_background: RwSignal<String> = RwSignal::new(String::new());
    let character_level: RwSignal<String> = RwSignal::new("1".to_string());

    // Generation options
    let include_equipment: RwSignal<bool> = RwSignal::new(true);
    let include_backstory: RwSignal<bool> = RwSignal::new(true);
    let random_stats: RwSignal<bool> = RwSignal::new(true);
    let backstory_length: RwSignal<String> = RwSignal::new("Medium".to_string());

    // Output state
    let generated_character: RwSignal<Option<Character>> = RwSignal::new(None);
    let is_generating: RwSignal<bool> = RwSignal::new(false);
    let status_message: RwSignal<String> = RwSignal::new(String::new());

    // Load supported systems on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match list_system_info().await {
                Ok(system_list) => {
                    systems.set(system_list.clone());
                    // Set default system (D&D 5e)
                    if let Some(default_system) = system_list.into_iter().find(|s| s.id == "dnd5e") {
                        selected_system.set(Some(default_system));
                    }
                }
                Err(e) => {
                    status_message.set(format!("Failed to load systems: {}", e));
                }
            }
        });
    });

    // Update selected system info when system ID changes
    let on_system_change = move |system_id: String| {
        selected_system_id.set(system_id.clone());
        // Reset race/class/background selection when system changes
        selected_race.set(String::new());
        selected_class.set(String::new());
        selected_background.set(String::new());

        spawn_local(async move {
            if let Ok(Some(info)) = get_game_system_info(system_id).await {
                selected_system.set(Some(info));
            }
        });
    };

    // Generate character handler
    let handle_generate = move |_: ev::MouseEvent| {
        is_generating.set(true);
        status_message.set("Generating character...".to_string());

        let system = selected_system_id.get();
        let name = character_name.get();
        let concept = character_concept.get();
        let race = selected_race.get();
        let class = selected_class.get();
        let background = selected_background.get();
        let level: u32 = character_level.get().parse().unwrap_or(1);
        let equipment = include_equipment.get();
        let backstory = include_backstory.get();
        let random = random_stats.get();
        let bs_length = backstory_length.get();

        spawn_local(async move {
            let options = GenerationOptions {
                system: Some(system),
                name: if name.is_empty() { None } else { Some(name) },
                concept: if concept.is_empty() { None } else { Some(concept) },
                race: if race.is_empty() { None } else { Some(race) },
                character_class: if class.is_empty() { None } else { Some(class) },
                background: if background.is_empty() { None } else { Some(background) },
                level: Some(level),
                point_buy: None,
                random_stats: random,
                include_equipment: equipment,
                include_backstory: backstory,
                backstory_length: Some(bs_length),
                theme: None,
                campaign_setting: None,
            };

            match generate_character_advanced(options).await {
                Ok(character) => {
                    generated_character.set(Some(character));
                    status_message.set("Character generated successfully!".to_string());
                }
                Err(e) => {
                    status_message.set(format!("Generation failed: {}", e));
                }
            }
            is_generating.set(false);
        });
    };

    // Clear character handler
    let clear_character = move |_: ev::MouseEvent| {
        generated_character.set(None);
        character_name.set(String::new());
        character_concept.set(String::new());
        selected_race.set(String::new());
        selected_class.set(String::new());
        selected_background.set(String::new());
        status_message.set(String::new());
    };

    view! {
        <div class="p-8 bg-gray-900 text-white min-h-screen font-sans">
            <div class="max-w-6xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-8">
                    <div class="flex items-center gap-4">
                        <a href="/" class="text-gray-400 hover:text-white transition-colors">
                            "<- Back"
                        </a>
                        <h1 class="text-3xl font-bold">"Character Generator"</h1>
                    </div>
                    <Show when=move || generated_character.get().is_some()>
                        <Button
                            variant=ButtonVariant::Secondary
                            on_click=clear_character
                        >
                            "New Character"
                        </Button>
                    </Show>
                </div>

                // Status message
                <Show when=move || !status_message.get().is_empty()>
                    <div class="mb-6 p-4 bg-gray-800 rounded-lg border border-gray-700">
                        {move || status_message.get()}
                    </div>
                </Show>

                <div class="grid grid-cols-1 xl:grid-cols-2 gap-8">
                    // Left Panel: Generation Form
                    <div class="space-y-6">
                        // System Selection Card
                        <Card>
                            <CardHeader>
                                <h2 class="text-xl font-semibold">"Game System"</h2>
                            </CardHeader>
                            <CardBody>
                                <div class="space-y-4">
                                    // System dropdown
                                    <div>
                                        <label class="block text-sm text-gray-400 mb-2">"Select System"</label>
                                        <select
                                            class="w-full p-3 bg-gray-700 rounded-lg border border-gray-600 focus:border-purple-500 outline-none transition-colors"
                                            on:change=move |e| {
                                                on_system_change(event_target_value(&e));
                                            }
                                        >
                                            <For
                                                each=move || systems.get()
                                                key=|s| s.id.clone()
                                                children=move |system| {
                                                    let id = system.id.clone();
                                                    let name = system.name.clone();
                                                    let id_clone = id.clone();
                                                    let is_selected = move || selected_system_id.get() == id_clone;
                                                    view! {
                                                        <option value=id.clone() selected=is_selected>
                                                            {name}
                                                        </option>
                                                    }
                                                }
                                            />
                                        </select>
                                    </div>

                                    // System description
                                    <Show when=move || selected_system.get().is_some()>
                                        <div class="p-3 bg-gray-800 rounded-lg">
                                            <p class="text-gray-300 text-sm">
                                                {move || selected_system.get().map(|s| s.description).unwrap_or_default()}
                                            </p>
                                            <Show when=move || selected_system.get().map(|s| s.has_levels).unwrap_or(false)>
                                                <p class="text-gray-400 text-xs mt-2">
                                                    "Max Level: "
                                                    {move || selected_system.get().and_then(|s| s.max_level).map(|l| l.to_string()).unwrap_or_else(|| "N/A".to_string())}
                                                </p>
                                            </Show>
                                        </div>
                                    </Show>
                                </div>
                            </CardBody>
                        </Card>

                        // Character Options Card
                        <Card>
                            <CardHeader>
                                <h2 class="text-xl font-semibold">"Character Options"</h2>
                            </CardHeader>
                            <CardBody>
                                <div class="space-y-4">
                                    // Name
                                    <div>
                                        <label class="block text-sm text-gray-400 mb-2">"Name (optional)"</label>
                                        <Input
                                            value=character_name
                                            placeholder="Leave blank for random name"
                                        />
                                    </div>

                                    // Concept
                                    <div>
                                        <label class="block text-sm text-gray-400 mb-2">"Character Concept (optional)"</label>
                                        <Input
                                            value=character_concept
                                            placeholder="e.g., Grizzled veteran, Mysterious wanderer"
                                        />
                                    </div>

                                    // Race selection
                                    <div>
                                        <label class="block text-sm text-gray-400 mb-2">"Race/Ancestry"</label>
                                        <select
                                            class="w-full p-3 bg-gray-700 rounded-lg border border-gray-600 focus:border-purple-500 outline-none"
                                            on:change=move |e| selected_race.set(event_target_value(&e))
                                        >
                                            <option value="">"Random"</option>
                                            <For
                                                each=move || selected_system.get().map(|s| s.races).unwrap_or_default()
                                                key=|r| r.clone()
                                                children=move |race| {
                                                    view! {
                                                        <option value=race.clone()>{race.clone()}</option>
                                                    }
                                                }
                                            />
                                        </select>
                                    </div>

                                    // Class selection
                                    <div>
                                        <label class="block text-sm text-gray-400 mb-2">"Class/Playbook"</label>
                                        <select
                                            class="w-full p-3 bg-gray-700 rounded-lg border border-gray-600 focus:border-purple-500 outline-none"
                                            on:change=move |e| selected_class.set(event_target_value(&e))
                                        >
                                            <option value="">"Random"</option>
                                            <For
                                                each=move || selected_system.get().map(|s| s.classes).unwrap_or_default()
                                                key=|c| c.clone()
                                                children=move |class| {
                                                    view! {
                                                        <option value=class.clone()>{class.clone()}</option>
                                                    }
                                                }
                                            />
                                        </select>
                                    </div>

                                    // Background selection
                                    <div>
                                        <label class="block text-sm text-gray-400 mb-2">"Background"</label>
                                        <select
                                            class="w-full p-3 bg-gray-700 rounded-lg border border-gray-600 focus:border-purple-500 outline-none"
                                            on:change=move |e| selected_background.set(event_target_value(&e))
                                        >
                                            <option value="">"Random"</option>
                                            <For
                                                each=move || selected_system.get().map(|s| s.backgrounds).unwrap_or_default()
                                                key=|b| b.clone()
                                                children=move |bg| {
                                                    view! {
                                                        <option value=bg.clone()>{bg.clone()}</option>
                                                    }
                                                }
                                            />
                                        </select>
                                    </div>

                                    // Level (only if system has levels)
                                    <Show when=move || selected_system.get().map(|s| s.has_levels).unwrap_or(true)>
                                        <div>
                                            <label class="block text-sm text-gray-400 mb-2">"Level"</label>
                                            <input
                                                type="number"
                                                class="w-full p-3 bg-gray-700 rounded-lg border border-gray-600 focus:border-purple-500 outline-none"
                                                min="1"
                                                max=move || selected_system.get().and_then(|s| s.max_level).unwrap_or(20).to_string()
                                                prop:value=move || character_level.get()
                                                on:input=move |e| character_level.set(event_target_value(&e))
                                            />
                                        </div>
                                    </Show>
                                </div>
                            </CardBody>
                        </Card>

                        // Generation Options Card
                        <Card>
                            <CardHeader>
                                <h2 class="text-xl font-semibold">"Generation Options"</h2>
                            </CardHeader>
                            <CardBody>
                                <div class="space-y-4">
                                    // Random stats toggle
                                    <div class="flex items-center gap-3">
                                        <input
                                            type="checkbox"
                                            class="w-5 h-5 rounded bg-gray-700 border-gray-600 text-purple-500 focus:ring-purple-500"
                                            prop:checked=move || random_stats.get()
                                            on:change=move |_| random_stats.update(|v| *v = !*v)
                                        />
                                        <label class="text-gray-300">"Random stat generation"</label>
                                    </div>

                                    // Include equipment toggle
                                    <div class="flex items-center gap-3">
                                        <input
                                            type="checkbox"
                                            class="w-5 h-5 rounded bg-gray-700 border-gray-600 text-purple-500 focus:ring-purple-500"
                                            prop:checked=move || include_equipment.get()
                                            on:change=move |_| include_equipment.update(|v| *v = !*v)
                                        />
                                        <label class="text-gray-300">"Generate starting equipment"</label>
                                    </div>

                                    // Include backstory toggle
                                    <div class="flex items-center gap-3">
                                        <input
                                            type="checkbox"
                                            class="w-5 h-5 rounded bg-gray-700 border-gray-600 text-purple-500 focus:ring-purple-500"
                                            prop:checked=move || include_backstory.get()
                                            on:change=move |_| include_backstory.update(|v| *v = !*v)
                                        />
                                        <label class="text-gray-300">"Generate backstory"</label>
                                    </div>

                                    // Backstory length
                                    <Show when=move || include_backstory.get()>
                                        <div>
                                            <label class="block text-sm text-gray-400 mb-2">"Backstory Length"</label>
                                            <select
                                                class="w-full p-3 bg-gray-700 rounded-lg border border-gray-600 focus:border-purple-500 outline-none"
                                                on:change=move |e| backstory_length.set(event_target_value(&e))
                                            >
                                                <option value="Brief">"Brief (50-100 words)"</option>
                                                <option value="Medium" selected>"Medium (150-300 words)"</option>
                                                <option value="Detailed">"Detailed (400-600 words)"</option>
                                            </select>
                                        </div>
                                    </Show>

                                    // Generate button
                                    <Button
                                        variant=ButtonVariant::Primary
                                        on_click=handle_generate
                                        disabled=is_generating.get()
                                        loading=is_generating.get()
                                        class="w-full mt-4 bg-purple-600 hover:bg-purple-500 py-3 text-lg"
                                    >
                                        {move || if is_generating.get() { "Generating..." } else { "Generate Character" }}
                                    </Button>
                                </div>
                            </CardBody>
                        </Card>
                    </div>

                    // Right Panel: Generated Character Display
                    <div>
                        <Card class="sticky top-8">
                            <CardHeader>
                                <h2 class="text-xl font-semibold">"Generated Character"</h2>
                            </CardHeader>
                            <CardBody>
                                <Show
                                    when=move || generated_character.get().is_some()
                                    fallback=move || view! {
                                        <div class="text-center py-16 text-gray-500">
                                            <div class="text-5xl mb-4">"D20"</div>
                                            <p class="text-lg">"No character generated yet"</p>
                                            <p class="text-sm mt-2">"Configure options and click Generate"</p>
                                        </div>
                                    }
                                >
                                    <CharacterDisplay character=generated_character />
                                </Show>
                            </CardBody>
                        </Card>
                    </div>
                </div>

                // System Quick Reference
                <Card class="mt-8">
                    <CardHeader>
                        <h2 class="text-xl font-semibold">"Supported Systems"</h2>
                    </CardHeader>
                    <CardBody>
                        <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
                            <For
                                each=move || systems.get()
                                key=|s| s.id.clone()
                                children=move |system| {
                                    let id = system.id.clone();
                                    let name = system.name.clone();
                                    let is_selected = {
                                        let id = id.clone();
                                        move || selected_system_id.get() == id
                                    };
                                    view! {
                                        <button
                                            class=move || format!(
                                                "p-3 rounded-lg border transition-all text-sm text-left {}",
                                                if is_selected() {
                                                    "bg-purple-600 border-purple-500 text-white"
                                                } else {
                                                    "bg-gray-800 border-gray-700 text-gray-400 hover:border-gray-600"
                                                }
                                            )
                                            on:click={
                                                let id = id.clone();
                                                move |_| on_system_change(id.clone())
                                            }
                                        >
                                            {name}
                                        </button>
                                    }
                                }
                            />
                        </div>
                    </CardBody>
                </Card>
            </div>
        </div>
    }
}

/// Character display component for showing generated character details
#[component]
fn CharacterDisplay(character: RwSignal<Option<Character>>) -> impl IntoView {
    view! {
        <div class="space-y-6 max-h-[calc(100vh-300px)] overflow-y-auto pr-2">
            // Header with name and basic info
            <div class="border-b border-gray-700 pb-4">
                <h3 class="text-2xl font-bold text-purple-400">
                    {move || character.get().map(|c| c.name.clone()).unwrap_or_default()}
                </h3>
                <p class="text-gray-400 mt-1">
                    {move || character.get().map(|c| c.concept.clone()).unwrap_or_default()}
                </p>
                <div class="flex flex-wrap gap-2 mt-3">
                    <Badge variant=BadgeVariant::Blue>
                        {move || character.get().map(|c| c.system.clone()).unwrap_or_default()}
                    </Badge>
                    {move || character.get().and_then(|c| c.race.clone()).map(|race| view! {
                        <Badge variant=BadgeVariant::Green>{race}</Badge>
                    })}
                    {move || character.get().and_then(|c| c.character_class.clone()).map(|class| view! {
                        <Badge variant=BadgeVariant::Purple>{class}</Badge>
                    })}
                    <Badge variant=BadgeVariant::Yellow>
                        {move || format!("Level {}", character.get().map(|c| c.level).unwrap_or(1))}
                    </Badge>
                </div>
            </div>

            // Attributes
            <div>
                <h4 class="font-semibold text-gray-300 mb-3">"Attributes"</h4>
                <div class="grid grid-cols-3 gap-2">
                    {move || {
                        character.get()
                            .map(|c| c.attributes.clone())
                            .unwrap_or_default()
                            .into_iter()
                            .map(|(name, attr)| {
                                let modifier = attr.modifier;
                                let mod_class = if modifier >= 0 { "text-green-400" } else { "text-red-400" };
                                let mod_text = if modifier >= 0 { format!("+{}", modifier) } else { format!("{}", modifier) };
                                view! {
                                    <div class="p-3 bg-gray-700 rounded-lg text-center">
                                        <div class="text-xs text-gray-400 uppercase tracking-wide">{name}</div>
                                        <div class="text-xl font-bold">{attr.base}</div>
                                        <div class=format!("text-xs {}", mod_class)>{mod_text}</div>
                                    </div>
                                }
                            })
                            .collect::<Vec<_>>()
                    }}
                </div>
            </div>

            // Skills
            {move || {
                let skills = character.get().map(|c| c.skills.clone()).unwrap_or_default();
                if skills.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div>
                            <h4 class="font-semibold text-gray-300 mb-3">"Skills"</h4>
                            <div class="flex flex-wrap gap-2">
                                {skills.into_iter().map(|(name, value)| {
                                    let display = if value >= 0 { format!("{} +{}", name, value) } else { format!("{} {}", name, value) };
                                    view! {
                                        <span class="px-2 py-1 bg-gray-700 rounded text-sm">{display}</span>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    })
                }
            }}

            // Traits
            {move || {
                let traits = character.get().map(|c| c.traits.clone()).unwrap_or_default();
                if traits.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div>
                            <h4 class="font-semibold text-gray-300 mb-3">"Traits & Abilities"</h4>
                            <div class="space-y-2">
                                {traits.into_iter().map(|t| view! {
                                    <div class="p-3 bg-gray-700 rounded-lg">
                                        <div class="flex items-center gap-2">
                                            <span class="font-medium text-purple-300">{t.name}</span>
                                            <span class="text-xs text-gray-500 px-2 py-0.5 bg-gray-800 rounded">{t.trait_type}</span>
                                        </div>
                                        <p class="text-sm text-gray-400 mt-1">{t.description}</p>
                                        {t.mechanical_effect.map(|effect| view! {
                                            <p class="text-xs text-blue-400 mt-1 italic">{effect}</p>
                                        })}
                                    </div>
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    })
                }
            }}

            // Equipment
            {move || {
                let equipment = character.get().map(|c| c.equipment.clone()).unwrap_or_default();
                if equipment.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div>
                            <h4 class="font-semibold text-gray-300 mb-3">"Equipment"</h4>
                            <div class="space-y-2">
                                {equipment.into_iter().map(|item| view! {
                                    <div class="p-3 bg-gray-700 rounded-lg">
                                        <div class="flex items-center gap-2">
                                            <span class="font-medium">{item.name}</span>
                                            <span class="text-xs text-gray-500 px-2 py-0.5 bg-gray-800 rounded">{item.category}</span>
                                        </div>
                                        <p class="text-sm text-gray-400 mt-1">{item.description}</p>
                                        {(!item.stats.is_empty()).then(|| view! {
                                            <div class="flex flex-wrap gap-1 mt-1">
                                                {item.stats.into_iter().map(|(k, v)| view! {
                                                    <span class="text-xs text-gray-500">{format!("{}: {}", k, v)}</span>
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        })}
                                    </div>
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    })
                }
            }}

            // Background
            {move || {
                let background = character.get().map(|c| c.background.clone()).unwrap_or_default();
                if background.origin.is_empty() && background.motivation.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div>
                            <h4 class="font-semibold text-gray-300 mb-3">"Background"</h4>
                            <div class="p-3 bg-gray-700 rounded-lg space-y-2">
                                {(!background.origin.is_empty()).then(|| view! {
                                    <div>
                                        <span class="text-xs text-gray-500 uppercase">"Origin: "</span>
                                        <span class="text-gray-300">{background.origin.clone()}</span>
                                    </div>
                                })}
                                {background.occupation.clone().map(|occ| view! {
                                    <div>
                                        <span class="text-xs text-gray-500 uppercase">"Occupation: "</span>
                                        <span class="text-gray-300">{occ}</span>
                                    </div>
                                })}
                                {(!background.motivation.is_empty()).then(|| view! {
                                    <div>
                                        <span class="text-xs text-gray-500 uppercase">"Motivation: "</span>
                                        <span class="text-gray-300">{background.motivation.clone()}</span>
                                    </div>
                                })}
                                {(!background.connections.is_empty()).then(|| view! {
                                    <div>
                                        <span class="text-xs text-gray-500 uppercase">"Connections: "</span>
                                        <span class="text-gray-300">{background.connections.join(", ")}</span>
                                    </div>
                                })}
                            </div>
                        </div>
                    })
                }
            }}

            // Backstory
            {move || {
                character.get().and_then(|c| c.backstory.clone()).map(|backstory| view! {
                    <div>
                        <h4 class="font-semibold text-gray-300 mb-3">"Backstory"</h4>
                        <div class="p-3 bg-gray-700 rounded-lg">
                            <p class="text-gray-300 text-sm whitespace-pre-wrap leading-relaxed">{backstory}</p>
                        </div>
                    </div>
                })
            }}

            // Notes
            {move || {
                let notes = character.get().map(|c| c.notes.clone()).unwrap_or_default();
                if notes.is_empty() {
                    None
                } else {
                    Some(view! {
                        <div>
                            <h4 class="font-semibold text-gray-300 mb-3">"Notes"</h4>
                            <p class="text-gray-400 text-sm">{notes}</p>
                        </div>
                    })
                }
            }}
        </div>
    }
}

/// Badge variant styles
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum BadgeVariant {
    #[default]
    Gray,
    Blue,
    Purple,
    Green,
    Red,
    Yellow,
}

impl BadgeVariant {
    fn class(&self) -> &'static str {
        match self {
            BadgeVariant::Gray => "bg-gray-700 text-gray-300",
            BadgeVariant::Blue => "bg-blue-900 text-blue-300",
            BadgeVariant::Purple => "bg-purple-900 text-purple-300",
            BadgeVariant::Green => "bg-green-900 text-green-300",
            BadgeVariant::Red => "bg-red-900 text-red-300",
            BadgeVariant::Yellow => "bg-yellow-900 text-yellow-300",
        }
    }
}

/// A styled badge/tag component
#[component]
fn Badge(
    /// The visual variant of the badge
    #[prop(default = BadgeVariant::Gray)]
    variant: BadgeVariant,
    /// Badge content
    children: Children,
) -> impl IntoView {
    let class = format!("text-xs px-2 py-1 rounded {}", variant.class());
    view! {
        <span class=class>{children()}</span>
    }
}
