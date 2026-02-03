use crate::bindings::{
    check_ocr_availability, get_extraction_presets, get_extraction_settings, reindex_library,
    save_extraction_settings, ExtractionPreset, ExtractionSettings, OcrAvailability,
    TokenReductionLevel,
};
use crate::components::design_system::{Button, ButtonVariant, Card};
use crate::services::notification_service::{show_error, show_success};
use leptos::ev;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[component]
pub fn DataSettingsView() -> impl IntoView {
    let reindex_status = RwSignal::new(String::new());
    let is_reindexing = RwSignal::new(false);

    // Extraction settings state
    let extraction_settings = RwSignal::new(ExtractionSettings::default());
    let presets = RwSignal::new(Vec::<ExtractionPreset>::new());
    let ocr_availability = RwSignal::new(None::<OcrAvailability>);
    let is_saving = RwSignal::new(false);
    let settings_status = RwSignal::new(String::new());
    let has_changes = RwSignal::new(false);

    // Load settings on mount
    Effect::new(move || {
        spawn_local(async move {
            // Load extraction settings
            if let Ok(settings) = get_extraction_settings().await {
                extraction_settings.set(settings);
            }
            // Load presets
            if let Ok(p) = get_extraction_presets().await {
                presets.set(p);
            }
            // Check OCR availability
            if let Ok(ocr) = check_ocr_availability().await {
                ocr_availability.set(Some(ocr));
            }
        });
    });

    let handle_reindex = move |_: web_sys::MouseEvent| {
        is_reindexing.set(true);
        reindex_status.set("Re-indexing...".to_string());
        spawn_local(async move {
            match reindex_library(None).await {
                Ok(msg) => {
                    reindex_status.set(msg.clone());
                    show_success("Re-indexing complete", Some(&msg));
                }
                Err(e) => {
                    reindex_status.set(format!("Error: {}", e));
                    show_error("Re-indexing Failed", Some(&e), None);
                }
            }
            is_reindexing.set(false);
        });
    };

    let handle_save_settings = move |_: ev::MouseEvent| {
        is_saving.set(true);
        settings_status.set("Saving...".to_string());
        let settings = extraction_settings.get();
        spawn_local(async move {
            match save_extraction_settings(settings).await {
                Ok(_) => {
                    settings_status.set("Settings saved".to_string());
                    has_changes.set(false);
                    show_success("Extraction Settings", Some("Settings saved successfully"));
                }
                Err(e) => {
                    settings_status.set(format!("Error: {}", e));
                    show_error("Save Failed", Some(&e), None);
                }
            }
            is_saving.set(false);
        });
    };

    let apply_preset = move |preset_name: String| {
        let current_presets = presets.get();
        if let Some(preset) = current_presets.iter().find(|p| p.name == preset_name) {
            extraction_settings.set(preset.settings.clone());
            has_changes.set(true);
            settings_status.set(format!("Applied preset: {}", preset_name));
        }
    };

    view! {
         <div class="space-y-8 animate-fade-in">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-theme-primary">"Data & Storage"</h3>
                <p class="text-theme-muted">"Manage your local library, search index, and document extraction."</p>
            </div>

            // Search Index Card
            <Card class="p-6">
                <div class="flex justify-between items-center">
                    <div>
                        <h4 class="font-bold">"Search Index"</h4>
                        <p class="text-sm text-theme-muted">"Rebuild the Meilisearch index if search results are incorrect."</p>
                         <p class="text-xs text-theme-accent mt-1">{move || reindex_status.get()}</p>
                    </div>
                    <Button
                        variant=ButtonVariant::Outline
                        on_click=handle_reindex
                        disabled=Signal::derive(move || is_reindexing.get())
                        loading=Signal::derive(move || is_reindexing.get())
                    >
                        "Re-index Library"
                    </Button>
                </div>
            </Card>

            // Extraction Settings Card
            <Card class="p-6">
                <div class="space-y-6">
                    <div class="flex justify-between items-start">
                        <div>
                            <h4 class="text-lg font-bold text-theme-primary">"Document Extraction"</h4>
                            <p class="text-sm text-theme-muted">"Configure how documents are processed during ingestion."</p>
                        </div>
                        // Preset dropdown
                        <div class="flex items-center gap-2">
                            <span class="text-sm text-theme-muted">"Preset:"</span>
                            <select
                                class="px-3 py-1.5 rounded-lg bg-theme-deep border border-theme-subtle text-theme-primary text-sm outline-none focus:border-theme-accent"
                                style="color-scheme: dark;"
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    if !val.is_empty() {
                                        apply_preset(val);
                                    }
                                }
                            >
                                <option value="">"Select preset..."</option>
                                {move || presets.get().into_iter().map(|p| {
                                    let value = p.name.clone();
                                    let label = format!("{} - {}", p.name, p.description);
                                    view! {
                                        <option value=value>{label}</option>
                                    }
                                }).collect::<Vec<_>>()}
                            </select>
                        </div>
                    </div>

                    // OCR Settings Section
                    <div class="space-y-4">
                        <div class="flex items-center gap-2">
                            <h5 class="font-semibold text-theme-primary">"OCR Settings"</h5>
                            {move || {
                                if let Some(ocr) = ocr_availability.get() {
                                    if ocr.external_ocr_ready {
                                        view! {
                                            <span class="text-xs px-2 py-0.5 rounded-full bg-green-500/20 text-green-400">"Available"</span>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <span class="text-xs px-2 py-0.5 rounded-full bg-yellow-500/20 text-yellow-400">"Not installed"</span>
                                        }.into_any()
                                    }
                                } else {
                                    view! { <span/> }.into_any()
                                }
                            }}
                        </div>

                        <div class="grid grid-cols-2 gap-4">
                            // OCR Enabled toggle
                            <label class="flex items-center gap-3 cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="w-4 h-4 rounded border-theme-subtle text-theme-accent focus:ring-theme-accent"
                                    prop:checked=move || extraction_settings.get().ocr_enabled
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        extraction_settings.update(|s| s.ocr_enabled = checked);
                                        has_changes.set(true);
                                    }
                                />
                                <span class="text-sm text-theme-secondary">"Enable OCR for scanned documents"</span>
                            </label>

                            // Force OCR toggle
                            <label class="flex items-center gap-3 cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="w-4 h-4 rounded border-theme-subtle text-theme-accent focus:ring-theme-accent"
                                    prop:checked=move || extraction_settings.get().force_ocr
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        extraction_settings.update(|s| s.force_ocr = checked);
                                        has_changes.set(true);
                                    }
                                />
                                <span class="text-sm text-theme-secondary">"Force OCR (always use OCR)"</span>
                            </label>
                        </div>

                        <div class="grid grid-cols-2 gap-4">
                            // OCR Language
                            <div>
                                <label class="block text-sm text-theme-muted mb-1">"OCR Language"</label>
                                <select
                                    class="w-full px-3 py-2 rounded-lg bg-theme-deep border border-theme-subtle text-theme-primary text-sm outline-none focus:border-theme-accent"
                                    style="color-scheme: dark;"
                                    prop:value=move || extraction_settings.get().ocr_language.clone()
                                    on:change=move |ev| {
                                        let val = event_target_value(&ev);
                                        extraction_settings.update(|s| s.ocr_language = val);
                                        has_changes.set(true);
                                    }
                                >
                                    <option value="eng">"English"</option>
                                    <option value="deu">"German"</option>
                                    <option value="fra">"French"</option>
                                    <option value="spa">"Spanish"</option>
                                    <option value="ita">"Italian"</option>
                                    <option value="por">"Portuguese"</option>
                                    <option value="nld">"Dutch"</option>
                                    <option value="pol">"Polish"</option>
                                    <option value="rus">"Russian"</option>
                                    <option value="jpn">"Japanese"</option>
                                    <option value="chi_sim">"Chinese (Simplified)"</option>
                                    <option value="kor">"Korean"</option>
                                </select>
                            </div>

                            // Min text threshold
                            <div>
                                <label class="block text-sm text-theme-muted mb-1">"Min text threshold (chars)"</label>
                                <input
                                    type="number"
                                    class="w-full px-3 py-2 rounded-lg bg-theme-deep border border-theme-subtle text-theme-primary text-sm outline-none focus:border-theme-accent"
                                    prop:value=move || extraction_settings.get().ocr_min_text_threshold.to_string()
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<usize>() {
                                            extraction_settings.update(|s| s.ocr_min_text_threshold = val);
                                            has_changes.set(true);
                                        }
                                    }
                                />
                            </div>
                        </div>
                    </div>

                    // Quality Settings Section
                    <div class="space-y-4 pt-4 border-t border-theme-subtle">
                        <h5 class="font-semibold text-theme-primary">"Quality Processing"</h5>

                        <div class="grid grid-cols-2 gap-4">
                            // Quality processing toggle
                            <label class="flex items-center gap-3 cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="w-4 h-4 rounded border-theme-subtle text-theme-accent focus:ring-theme-accent"
                                    prop:checked=move || extraction_settings.get().quality_processing
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        extraction_settings.update(|s| s.quality_processing = checked);
                                        has_changes.set(true);
                                    }
                                />
                                <span class="text-sm text-theme-secondary">"Enable text quality processing"</span>
                            </label>

                            // Language detection toggle
                            <label class="flex items-center gap-3 cursor-pointer">
                                <input
                                    type="checkbox"
                                    class="w-4 h-4 rounded border-theme-subtle text-theme-accent focus:ring-theme-accent"
                                    prop:checked=move || extraction_settings.get().language_detection
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        extraction_settings.update(|s| s.language_detection = checked);
                                        has_changes.set(true);
                                    }
                                />
                                <span class="text-sm text-theme-secondary">"Auto-detect document language"</span>
                            </label>
                        </div>

                        // Token reduction dropdown
                        <div>
                            <label class="block text-sm text-theme-muted mb-1">"Token Reduction Level"</label>
                            <select
                                class="w-full px-3 py-2 rounded-lg bg-theme-deep border border-theme-subtle text-theme-primary text-sm outline-none focus:border-theme-accent"
                                style="color-scheme: dark;"
                                on:change=move |ev| {
                                    let val = event_target_value(&ev);
                                    let level = match val.as_str() {
                                        "light" => TokenReductionLevel::Light,
                                        "moderate" => TokenReductionLevel::Moderate,
                                        "aggressive" => TokenReductionLevel::Aggressive,
                                        "maximum" => TokenReductionLevel::Maximum,
                                        _ => TokenReductionLevel::Off,
                                    };
                                    extraction_settings.update(|s| s.token_reduction = level);
                                    has_changes.set(true);
                                }
                            >
                                <option value="off" selected=move || extraction_settings.get().token_reduction == TokenReductionLevel::Off>"Off - No reduction"</option>
                                <option value="light" selected=move || extraction_settings.get().token_reduction == TokenReductionLevel::Light>"Light - Preserve most content"</option>
                                <option value="moderate" selected=move || extraction_settings.get().token_reduction == TokenReductionLevel::Moderate>"Moderate - Balance quality/size"</option>
                                <option value="aggressive" selected=move || extraction_settings.get().token_reduction == TokenReductionLevel::Aggressive>"Aggressive - Prioritize size"</option>
                                <option value="maximum" selected=move || extraction_settings.get().token_reduction == TokenReductionLevel::Maximum>"Maximum - Minimal output"</option>
                            </select>
                            <p class="text-xs text-theme-muted mt-1">"Reduce token count for LLM context optimization."</p>
                        </div>
                    </div>

                    // Advanced Settings Section
                    <div class="space-y-4 pt-4 border-t border-theme-subtle">
                        <h5 class="font-semibold text-theme-primary">"Advanced Settings"</h5>

                        <div class="grid grid-cols-2 gap-4">
                            // Image DPI
                            <div>
                                <label class="block text-sm text-theme-muted mb-1">"OCR Image DPI"</label>
                                <input
                                    type="number"
                                    min="72"
                                    max="600"
                                    class="w-full px-3 py-2 rounded-lg bg-theme-deep border border-theme-subtle text-theme-primary text-sm outline-none focus:border-theme-accent"
                                    prop:value=move || extraction_settings.get().image_dpi.to_string()
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<u32>() {
                                            extraction_settings.update(|s| s.image_dpi = val);
                                            has_changes.set(true);
                                        }
                                    }
                                />
                            </div>

                            // Max concurrent extractions
                            <div>
                                <label class="block text-sm text-theme-muted mb-1">"Max Concurrent Extractions"</label>
                                <input
                                    type="number"
                                    min="1"
                                    max="32"
                                    class="w-full px-3 py-2 rounded-lg bg-theme-deep border border-theme-subtle text-theme-primary text-sm outline-none focus:border-theme-accent"
                                    prop:value=move || extraction_settings.get().max_concurrent_extractions.to_string()
                                    on:input=move |ev| {
                                        if let Ok(val) = event_target_value(&ev).parse::<usize>() {
                                            extraction_settings.update(|s| s.max_concurrent_extractions = val);
                                            has_changes.set(true);
                                        }
                                    }
                                />
                            </div>
                        </div>

                        // Cache toggle
                        <label class="flex items-center gap-3 cursor-pointer">
                            <input
                                type="checkbox"
                                class="w-4 h-4 rounded border-theme-subtle text-theme-accent focus:ring-theme-accent"
                                prop:checked=move || extraction_settings.get().use_cache
                                on:change=move |ev| {
                                    let checked = event_target_checked(&ev);
                                    extraction_settings.update(|s| s.use_cache = checked);
                                    has_changes.set(true);
                                }
                            />
                            <span class="text-sm text-theme-secondary">"Cache extraction results"</span>
                        </label>
                    </div>

                    // Save button
                    <div class="flex items-center justify-between pt-4 border-t border-theme-subtle">
                        <span class="text-sm text-theme-muted">{move || settings_status.get()}</span>
                        <Button
                            variant=ButtonVariant::Primary
                            on_click=handle_save_settings
                            disabled=Signal::derive(move || is_saving.get() || !has_changes.get())
                            loading=Signal::derive(move || is_saving.get())
                        >
                            "Save Settings"
                        </Button>
                    </div>
                </div>
            </Card>
        </div>
    }
}
