use crate::components::design_system::Card;
use crate::services::theme_service::ThemeState;
use leptos::prelude::*;

#[component]
pub fn GeneralSettings() -> impl IntoView {
    let theme_state = expect_context::<ThemeState>();

    // Theme presets with preview colors
    let presets = vec![
        ("fantasy", "#2a1a1a", "#d4af37"),
        ("cosmic", "#0f0f1a", "#7df9ff"),
        ("terminal", "#0a0a0a", "#00ff41"),
        ("noir", "#000000", "#ffffff"),
        ("neon", "#1a0b1e", "#ff00ff"),
    ];

    view! {
        <div class="space-y-8 animate-fade-in pb-20">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-theme-primary">"Appearance"</h3>
                <p class="text-theme-muted">"Customize the look and feel of your assistant."</p>
            </div>

            // Theme Presets
            <Card class="p-6 space-y-6">
                <h4 class="font-semibold text-theme-secondary">"Theme Preset"</h4>

                <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
                    {presets.into_iter().map(|(id, bg, accent)| {
                        let is_active = move || theme_state.current_preset.get() == Some(id.to_string());
                        let p_clone = id.to_string();
                        let p_text = id.to_string();

                        view! {
                            <button
                                class=move || format!(
                                    "relative p-4 rounded-xl border-2 text-left transition-all duration-300 hover:scale-[1.02] group {}",
                                    if is_active() {
                                        "border-theme-accent bg-theme-elevated ring-2 ring-[var(--accent-primary)]/20 shadow-lg"
                                    } else {
                                        "border-theme-subtle hover:border-theme-strong bg-theme-surface hover:bg-theme-elevated"
                                    }
                                )
                                on:click=move |_| theme_state.set_preset(&p_clone)
                            >
                                <div class="font-medium capitalize mb-3 text-theme-primary group-hover:text-theme-accent transition-colors">
                                    {p_text}
                                </div>

                                // Mini preview swatches
                                <div class="flex gap-2 mb-1">
                                    <div
                                        class="w-8 h-8 rounded-full border border-white/10 shadow-inner"
                                        style=("background-color", bg)
                                        title="Background"
                                    ></div>
                                    <div
                                        class="w-8 h-8 rounded-full border border-white/10 shadow-inner -ml-4"
                                        style=("background-color", accent)
                                        title="Accent"
                                        ></div>
                                </div>

                                // Active Indicator
                                {move || if is_active() {
                                    view! {
                                        <div class="absolute top-3 right-3 text-theme-accent animate-fade-in">
                                            <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="lucide lucide-check-circle-2">
                                                <path d="M12 22c5.523 0 10-4.477 10-10S17.523 2 12 2 2 6.477 2 12s4.477 10 10 10z"/>
                                                <path d="m9 12 2 2 4-4"/>
                                            </svg>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! { <span/> }.into_any()
                                }}
                            </button>
                        }
                    }).collect::<Vec<_>>()}
                </div>
            </Card>

            // Visual Tweaks
            <Card class="p-6">
                 // Motion Toggle (Placeholder)
                 <div class="flex items-center justify-between opacity-50 cursor-not-allowed mb-6" title="Coming soon">
                    <div>
                        <h4 class="font-semibold text-theme-secondary">"Reduce Motion"</h4>
                        <p class="text-sm text-theme-muted">"Disable advanced animations (film grain, scanlines)."</p>
                    </div>
                    <div class="h-6 w-11 bg-theme-surface rounded-full border border-theme-subtle relative">
                         <div class="absolute left-1 top-1 h-4 w-4 bg-theme-muted rounded-full"></div>
                    </div>
                 </div>

                 // Navigation Mode Toggle
                 {
                    let layout_state = crate::services::layout_service::use_layout_state();
                    let is_text_mode = layout_state.text_navigation;

                    view! {
                        <div class="flex items-center justify-between">
                            <div>
                                <h4 class="font-semibold text-theme-secondary">"Text Navigation"</h4>
                                <p class="text-sm text-theme-muted">"Show text labels instead of icons in the navigation bar."</p>
                            </div>
                            <button
                                class=move || format!(
                                    "h-6 w-11 rounded-full border transition-colors duration-200 relative focus:outline-none focus:ring-2 focus:ring-theme-accent {}",
                                    if is_text_mode.get() {
                                        "bg-theme-accent border-theme-accent"
                                    } else {
                                        "bg-theme-surface border-theme-subtle"
                                    }
                                )
                                on:click=move |_| is_text_mode.update(|v| *v = !*v)
                                role="switch"
                                aria-checked=move || is_text_mode.get().to_string()
                            >
                                <div
                                    class=move || format!(
                                        "absolute top-1 left-1 h-4 w-4 rounded-full bg-white shadow-sm transition-transform duration-200 {}",
                                        if is_text_mode.get() { "translate-x-5" } else { "translate-x-0" }
                                    )
                                />
                            </button>
                        </div>
                    }
                 }
            </Card>

        </div>
    }
}
