use leptos::prelude::*;
use crate::components::design_system::{Card, Button, ButtonVariant};
use wasm_bindgen_futures::spawn_local;
use crate::bindings::{reindex_library, check_meilisearch_health};
use crate::services::notification_service::{show_success, show_error};

#[component]
pub fn DataSettingsView() -> impl IntoView {
    let reindex_status = RwSignal::new(String::new());
    let is_reindexing = RwSignal::new(false);

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

    view! {
         <div class="space-y-8 animate-fade-in">
            <div class="space-y-2">
                <h3 class="text-xl font-bold text-[var(--text-primary)]">"Data & Storage"</h3>
                <p class="text-[var(--text-muted)]">"Manage your local library and search index."</p>
            </div>

            <Card class="p-6">
                <div class="flex justify-between items-center">
                    <div>
                        <h4 class="font-bold">"Search Index"</h4>
                        <p class="text-sm text-[var(--text-muted)]">"Rebuild the Meilisearch index if search results are incorrect."</p>
                         <p class="text-xs text-[var(--accent-primary)] mt-1">{move || reindex_status.get()}</p>
                    </div>
                    <Button
                        variant=ButtonVariant::Outline
                        on_click=handle_reindex
                        disabled=is_reindexing.get()
                        loading=is_reindexing.get()
                    >
                        "Re-index Library"
                    </Button>
                </div>
            </Card>
        </div>
    }
}
