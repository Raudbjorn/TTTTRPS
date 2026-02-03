use crate::components::design_system::{Button, ButtonVariant};
use crate::services::notification_service::{remove_notification, Notification, ToastType};
use leptos::prelude::*;

#[component]
pub fn ToastContainer() -> impl IntoView {
    let state = crate::services::notification_service::use_notification_state();

    view! {
        <div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
            {move || state.notifications.get().into_iter().map(|notification| {
                view! {
                    <Toast notification=notification />
                }
            }).collect_view()}
        </div>
    }
}

#[component]
pub fn Toast(notification: Notification) -> impl IntoView {
    let (is_exiting, set_is_exiting) = signal(false);
    let id = notification.id;

    // Handle close
    let close = move || {
        set_is_exiting.set(true);
        // Wait for animation then remove
        set_timeout(
            move || {
                remove_notification(id);
            },
            std::time::Duration::from_millis(300),
        );
    };

    // Auto-close if duration is set
    if let Some(duration) = notification.duration_ms {
        let close = close.clone();
        set_timeout(
            move || {
                close();
            },
            std::time::Duration::from_millis(duration),
        );
    }

    let bg_class = match notification.toast_type {
        ToastType::Success => "bg-[var(--bg-surface)] border-l-4 border-[var(--success)]",
        ToastType::Error => "bg-[var(--bg-surface)] border-l-4 border-[var(--error)]",
        ToastType::Warning => "bg-[var(--bg-surface)] border-l-4 border-[var(--warning)]",
        ToastType::Info => "bg-[var(--bg-surface)] border-l-4 border-[var(--accent)]",
    };

    let icon = match notification.toast_type {
        ToastType::Success => view! { <span class="text-[var(--success)]">"✓"</span> },
        ToastType::Error => view! { <span class="text-[var(--error)]">"⚠"</span> },
        ToastType::Warning => view! { <span class="text-[var(--warning)]">"!"</span> },
        ToastType::Info => view! { <span class="text-[var(--accent)]">"i"</span> },
    };

    view! {
        <div
            class=move || format!(
                "pointer-events-auto min-w-[300px] max-w-md p-4 rounded shadow-lg border border-[var(--border-subtle)] flex gap-3 transition-all duration-300 transform {} {}",
                bg_class,
                if is_exiting.get() { "translate-x-full opacity-0" } else { "translate-x-0 opacity-100" }
            )
            role="alert"
        >
            <div class="flex-shrink-0 text-lg">
                {icon}
            </div>
            <div class="flex-1 flex flex-col gap-2">
                <div class="font-medium text-[var(--text-primary)]">
                    {notification.title}
                </div>
                {if let Some(msg) = notification.message {
                    view! { <div class="text-sm text-[var(--text-muted)] text-wrap break-words">{msg}</div> }.into_any()
                } else {
                    view! { }.into_any()
                }}

                {if let Some(action) = notification.action {
                    view! {
                        <div class="mt-1 flex justify-end">
                            <Button
                                variant=ButtonVariant::Secondary
                                on_click=move |_| {
                                    (action.handler)();
                                    close();
                                }
                                class="text-xs px-2 py-1 h-auto"
                            >
                                {action.label}
                            </Button>
                        </div>
                    }.into_any()
                } else {
                    view! { }.into_any()
                }}
            </div>
            <button
                class="flex-shrink-0 text-[var(--text-muted)] hover:text-[var(--text-primary)] self-start -mt-1 -mr-1"
                on:click=move |_| close()
                aria-label="Close"
            >
                "×"
            </button>
        </div>
    }
}
