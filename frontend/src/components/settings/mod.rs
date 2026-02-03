pub mod claude_auth;
pub mod copilot_auth;
pub mod data;
pub mod embedding;
pub mod extraction;
pub mod gemini_auth;
pub mod general;
pub mod llm;
pub mod model_selection;
pub mod voice;

pub use claude_auth::{ClaudeAuth, ClaudeStatusBadge};
pub use copilot_auth::{CopilotAuth, CopilotStatusBadge};
pub use gemini_auth::{GeminiAuth, GeminiStatusBadge};

pub use crate::bindings::TextExtractionProvider;
pub use embedding::{EmbeddingProvider, EmbeddingSettingsView, SemanticAnalysisProvider};
pub use extraction::ExtractionSettingsView;
use leptos::prelude::*;
pub use llm::LLMProvider;
pub use model_selection::ModelSelectionDashboard;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Intelligence,
    Voice,
    Data,
    Extraction,
}

#[component]
pub fn Settings() -> impl IntoView {
    SettingsShell()
}

#[component]
pub fn SettingsShell() -> impl IntoView {
    let active_tab = RwSignal::new(SettingsTab::General);

    view! {
        <div class="flex h-screen bg-theme-deep text-theme-primary font-sans overflow-hidden">
            // Sidebar Navigation
            <aside class="w-64 flex-shrink-0 border-r border-theme-subtle bg-theme-surface flex flex-col">
                <div class="p-6">
                    <h2 class="text-2xl font-bold font-display bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] bg-clip-text text-transparent">
                        "Settings"
                    </h2>
                    <p class="text-sm text-theme-muted mt-1">"Configure your assistant"</p>
                </div>

                <nav class="flex-1 px-4 space-y-2 overflow-y-auto">
                    <TabButton
                        tab=SettingsTab::General
                        active_tab=active_tab
                        icon="cog"
                        label="General"
                        desc="Theme & Appearance"
                    />
                    <TabButton
                        tab=SettingsTab::Intelligence
                        active_tab=active_tab
                        icon="brain"
                        label="Intelligence"
                        desc="LLM Providers & Models"
                    />
                    <TabButton
                        tab=SettingsTab::Voice
                        active_tab=active_tab
                        icon="mic"
                        label="Voice"
                        desc="TTS & Cloning"
                    />
                    <TabButton
                        tab=SettingsTab::Data
                        active_tab=active_tab
                        icon="database"
                        label="Data & Library"
                        desc="Storage & Indexing"
                    />
                    <TabButton
                        tab=SettingsTab::Extraction
                        active_tab=active_tab
                        icon="file"
                        label="Extraction"
                        desc="Document Processing"
                    />
                </nav>

                <div class="p-4 border-t border-theme-subtle text-xs text-theme-muted text-center">
                    "TTRPG Assistant v0.1.0"
                </div>
            </aside>

            // Main Content Area
            <main class="flex-1 overflow-y-auto relative bg-theme-deep">
                <div class="max-w-4xl mx-auto p-8 min-h-full">
                    <div class="transition-opacity duration-300 ease-in-out">
                         {move || match active_tab.get() {
                            SettingsTab::General => view! { <general::GeneralSettings /> }.into_any(),
                            SettingsTab::Intelligence => view! { <llm::LLMSettingsView /> }.into_any(),
                            SettingsTab::Voice => view! { <voice::VoiceSettingsView /> }.into_any(),
                            SettingsTab::Data => view! { <data::DataSettingsView /> }.into_any(),
                            SettingsTab::Extraction => view! { <extraction::ExtractionSettingsView /> }.into_any(),
                        }}
                    </div>
                </div>
            </main>
        </div>
    }
}

#[component]
fn TabButton(
    tab: SettingsTab,
    active_tab: RwSignal<SettingsTab>,
    icon: &'static str,
    label: &'static str,
    desc: &'static str,
) -> impl IntoView {
    let is_active = move || active_tab.get() == tab;

    view! {
        <button
            class=move || format!(
                "w-full text-left px-4 py-3 rounded-xl transition-all duration-200 group relative overflow-hidden {}",
                if is_active() {
                    "bg-theme-accent text-white shadow-lg shadow-[var(--accent-primary)]/20"
                } else {
                    "hover:bg-theme-elevated text-theme-secondary hover:text-theme-primary"
                }
            )
            on:click=move |_| active_tab.set(tab)
        >
            <div class="flex items-center gap-3 relative z-10">
                <span class="text-xl">
                    {match icon {
                        "cog" => view! { <i class="las la-cog"></i> }.into_any(), // Placeholder icons, will replace with SVGs
                        "brain" => view! { <i class="las la-brain"></i> }.into_any(),
                        "mic" => view! { <i class="las la-microphone"></i> }.into_any(),
                        "database" => view! { <i class="las la-database"></i> }.into_any(),
                        "file" => view! { <i class="las la-file-alt"></i> }.into_any(),
                        _ => view! { <i></i> }.into_any()
                    }}
                </span>
                <div>
                    <div class=move || format!("font-semibold {}", if is_active() { "text-white" } else { "text-theme-primary" })>
                        {label}
                    </div>
                    <div class=move || format!("text-xs {}", if is_active() { "text-white/80" } else { "text-theme-muted" })>
                        {desc}
                    </div>
                </div>
            </div>

            // Subtle glow effect for active state
            {move || if is_active() {
                view! {
                    <div class="absolute inset-0 bg-gradient-to-tr from-white/10 to-transparent pointer-events-none" />
                }.into_any()
            } else {
                view! { <span /> }.into_any()
            }}
        </button>
    }
}
