pub mod general;
pub mod llm;
pub mod voice;
pub mod data;
pub mod model_selection;
pub mod extraction;
pub mod embedding;
pub mod claude_gate_auth;
pub mod copilot_auth;

pub use claude_gate_auth::{ClaudeGateAuth, ClaudeGateStatusBadge};
pub use copilot_auth::{CopilotAuth, CopilotStatusBadge};

use leptos::prelude::*;
pub use llm::LLMProvider;
pub use model_selection::ModelSelectionDashboard;
pub use extraction::ExtractionSettingsView;
pub use crate::bindings::TextExtractionProvider;
pub use embedding::{EmbeddingSettingsView, EmbeddingProvider, SemanticAnalysisProvider};

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
        <div class="flex h-screen bg-[var(--bg-deep)] text-[var(--text-primary)] font-sans overflow-hidden">
            // Sidebar Navigation
            <aside class="w-64 flex-shrink-0 border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] flex flex-col">
                <div class="p-6">
                    <h2 class="text-2xl font-bold font-display bg-gradient-to-r from-[var(--accent-primary)] to-[var(--accent-secondary)] bg-clip-text text-transparent">
                        "Settings"
                    </h2>
                    <p class="text-sm text-[var(--text-muted)] mt-1">"Configure your assistant"</p>
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

                <div class="p-4 border-t border-[var(--border-subtle)] text-xs text-[var(--text-muted)] text-center">
                    "TTRPG Assistant v0.1.0"
                </div>
            </aside>

            // Main Content Area
            <main class="flex-1 overflow-y-auto relative bg-[var(--bg-deep)]">
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
                    "bg-[var(--accent-primary)] text-white shadow-lg shadow-[var(--accent-primary)]/20"
                } else {
                    "hover:bg-[var(--bg-elevated)] text-[var(--text-secondary)] hover:text-[var(--text-primary)]"
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
                    <div class=move || format!("font-semibold {}", if is_active() { "text-white" } else { "text-[var(--text-primary)]" })>
                        {label}
                    </div>
                    <div class=move || format!("text-xs {}", if is_active() { "text-white/80" } else { "text-[var(--text-muted)]" })>
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
