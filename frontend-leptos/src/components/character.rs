//! Character Creator Component
//!
//! A form for generating RPG characters with configurable options.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::bindings::{generate_character, get_supported_systems, Character, GenerationOptions};
use crate::components::design_system::{Button, ButtonVariant, Card, CardBody, CardHeader, Input};

/// Character Creator page component
#[component]
pub fn CharacterCreator() -> impl IntoView {
    // State signals
    let systems: RwSignal<Vec<String>> = RwSignal::new(Vec::new());
    let selected_system: RwSignal<String> = RwSignal::new("D&D 5e".to_string());
    let character_name: RwSignal<String> = RwSignal::new(String::new());
    let character_type: RwSignal<String> = RwSignal::new(String::new());
    let character_level: RwSignal<String> = RwSignal::new("1".to_string());
    let include_backstory: RwSignal<bool> = RwSignal::new(true);

    let generated_character: RwSignal<Option<Character>> = RwSignal::new(None);
    let is_generating: RwSignal<bool> = RwSignal::new(false);
    let status_message: RwSignal<String> = RwSignal::new(String::new());

    // Load supported systems on mount
    Effect::new(move |_| {
        spawn_local(async move {
            if let Ok(list) = get_supported_systems().await {
                systems.set(list);
            }
        });
    });

    // Generate character handler
    let handle_generate = move |_: ev::MouseEvent| {
        is_generating.set(true);
        status_message.set("Generating character...".to_string());

        let system = selected_system.get();
        let name = character_name.get();
        let ctype = character_type.get();
        let level: u32 = character_level.get().parse().unwrap_or(1);
        let backstory = include_backstory.get();

        spawn_local(async move {
            let options = GenerationOptions {
                system,
                character_type: if ctype.is_empty() { None } else { Some(ctype) },
                level: Some(level),
                name: if name.is_empty() { None } else { Some(name) },
                include_backstory: backstory,
            };

            match generate_character(options).await {
                Ok(character) => {
                    generated_character.set(Some(character));
                    status_message.set("Character generated!".to_string());
                }
                Err(e) => {
                    status_message.set(format!("Error: {}", e));
                }
            }
            is_generating.set(false);
        });
    };

    // Clear character handler
    let clear_character = move |_: ev::MouseEvent| {
        generated_character.set(None);
        character_name.set(String::new());
        character_type.set(String::new());
        status_message.set(String::new());
    };

    // Toggle backstory handler
    let toggle_backstory = move |_: ev::Event| {
        include_backstory.update(|v| *v = !*v);
    };

    view! {
        <div class="p-8 bg-gray-900 text-white min-h-screen font-sans">
            <div class="max-w-4xl mx-auto">
                // Header
                <div class="flex items-center justify-between mb-8">
                    <div class="flex items-center">
                        <a href="/" class="mr-4 text-gray-400 hover:text-white">"<- Chat"</a>
                        <h1 class="text-2xl font-bold">"Character Generator"</h1>
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
                    <div class="mb-4 p-3 bg-gray-800 rounded text-sm">
                        {move || status_message.get()}
                    </div>
                </Show>

                <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                    // Generator Form
                    <Card>
                        <CardHeader>
                            <h2 class="text-xl font-semibold">"Generation Options"</h2>
                        </CardHeader>
                        <CardBody>
                            <div class="space-y-4">
                                // System selection
                                <div>
                                    <label class="block text-sm text-gray-400 mb-1">"Game System"</label>
                                    <select
                                        class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 outline-none"
                                        on:change=move |e| {
                                            selected_system.set(event_target_value(&e));
                                        }
                                    >
                                        <For
                                            each=move || systems.get()
                                            key=|s| s.clone()
                                            children=move |system| {
                                                let system_clone = system.clone();
                                                let is_selected = move || selected_system.get() == system_clone;
                                                view! {
                                                    <option
                                                        value=system.clone()
                                                        selected=is_selected
                                                    >
                                                        {system.clone()}
                                                    </option>
                                                }
                                            }
                                        />
                                        // Fallback options if systems haven't loaded
                                        <Show when=move || systems.get().is_empty()>
                                            <option value="D&D 5e">"D&D 5e"</option>
                                            <option value="Pathfinder 2e">"Pathfinder 2e"</option>
                                            <option value="Call of Cthulhu">"Call of Cthulhu"</option>
                                        </Show>
                                    </select>
                                </div>

                                // Character name (optional)
                                <div>
                                    <label class="block text-sm text-gray-400 mb-1">"Character Name (optional)"</label>
                                    <Input
                                        value=character_name
                                        placeholder="Leave blank for random name"
                                    />
                                </div>

                                // Character type/class (optional)
                                <div>
                                    <label class="block text-sm text-gray-400 mb-1">"Class/Type (optional)"</label>
                                    <Input
                                        value=character_type
                                        placeholder="e.g., Fighter, Wizard, Investigator"
                                    />
                                </div>

                                // Level
                                <div>
                                    <label class="block text-sm text-gray-400 mb-1">"Level"</label>
                                    <input
                                        type="number"
                                        class="w-full p-2 bg-gray-700 rounded border border-gray-600 focus:border-blue-500 outline-none"
                                        min="1"
                                        max="20"
                                        prop:value=move || character_level.get()
                                        on:input=move |e| character_level.set(event_target_value(&e))
                                    />
                                </div>

                                // Backstory toggle
                                <div class="flex items-center gap-2">
                                    <input
                                        type="checkbox"
                                        class="w-4 h-4"
                                        prop:checked=move || include_backstory.get()
                                        on:change=toggle_backstory
                                    />
                                    <label class="text-sm text-gray-400">"Generate backstory"</label>
                                </div>

                                // Generate button
                                <Button
                                    variant=ButtonVariant::Primary
                                    on_click=handle_generate
                                    disabled=is_generating.get()
                                    loading=is_generating.get()
                                    class="w-full mt-4 bg-purple-600 hover:bg-purple-500"
                                >
                                    {move || if is_generating.get() { "Generating..." } else { "Generate Character" }}
                                </Button>
                            </div>
                        </CardBody>
                    </Card>

                    // Character Display
                    <Card>
                        <CardHeader>
                            <h2 class="text-xl font-semibold">"Generated Character"</h2>
                        </CardHeader>
                        <CardBody>
                            <Show
                                when=move || generated_character.get().is_some()
                                fallback=move || view! {
                                    <div class="text-center py-12 text-gray-500">
                                        <p>"No character generated yet"</p>
                                        <p class="text-sm mt-2">"Configure options and click Generate"</p>
                                    </div>
                                }
                            >
                                <CharacterDisplay character=generated_character />
                            </Show>
                        </CardBody>
                    </Card>
                </div>

                // Tips
                <Card class="mt-6">
                    <CardBody>
                        <h3 class="font-semibold mb-2">"Tips"</h3>
                        <ul class="text-sm text-gray-400 space-y-1">
                            <li>"Leave name and class blank for fully random generation"</li>
                            <li>"Generated characters use system-appropriate stats and skills"</li>
                            <li>"Backstories are procedurally generated based on class and system"</li>
                            <li>"You can regenerate as many times as you like"</li>
                        </ul>
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
        <div class="space-y-4">
            // Header with name and badges
            <div class="border-b border-gray-700 pb-4">
                <h3 class="text-2xl font-bold text-purple-400">
                    {move || character.get().map(|c| c.name.clone()).unwrap_or_default()}
                </h3>
                <div class="flex gap-2 mt-2 flex-wrap">
                    <Badge variant=BadgeVariant::Blue>
                        {move || character.get().map(|c| c.system.clone()).unwrap_or_default()}
                    </Badge>
                    <Badge variant=BadgeVariant::Purple>
                        {move || character.get().map(|c| c.character_type.clone()).unwrap_or_default()}
                    </Badge>
                    {move || character.get().and_then(|c| c.level).map(|level| view! {
                        <Badge variant=BadgeVariant::Green>{format!("Level {}", level)}</Badge>
                    })}
                </div>
            </div>

            // Attributes
            <div>
                <h4 class="font-semibold text-gray-300 mb-2">"Attributes"</h4>
                <div class="grid grid-cols-3 gap-2">
                    {move || {
                        character.get().map(|c| c.attributes.clone()).unwrap_or_default()
                            .into_iter()
                            .map(|attr| {
                                let attr_name = attr.name.clone();
                                let attr_value = attr.value;
                                let modifier_view = attr.modifier.map(|mod_val| {
                                    let mod_class = if mod_val >= 0 {
                                        "text-xs text-green-400"
                                    } else {
                                        "text-xs text-red-400"
                                    };
                                    let mod_text = if mod_val >= 0 {
                                        format!("+{}", mod_val)
                                    } else {
                                        format!("{}", mod_val)
                                    };
                                    view! {
                                        <div class=mod_class>{mod_text}</div>
                                    }
                                });
                                view! {
                                    <div class="p-2 bg-gray-700 rounded text-center">
                                        <div class="text-xs text-gray-400">{attr_name}</div>
                                        <div class="text-lg font-bold">{attr_value}</div>
                                        {modifier_view}
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
                            <h4 class="font-semibold text-gray-300 mb-2">"Skills"</h4>
                            <div class="flex flex-wrap gap-2">
                                {skills.into_iter().map(|skill| view! {
                                    <span class="px-2 py-1 bg-gray-700 rounded text-sm">{skill}</span>
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    })
                }
            }}

            // Backstory
            {move || {
                character.get().and_then(|c| c.backstory.clone()).map(|backstory| view! {
                    <div>
                        <h4 class="font-semibold text-gray-300 mb-2">"Backstory"</h4>
                        <p class="text-gray-400 text-sm whitespace-pre-wrap">{backstory}</p>
                    </div>
                })
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
