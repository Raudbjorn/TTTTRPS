//! Header component with connection controls.

use leptos::prelude::*;

#[component]
pub fn Header(
    connected: RwSignal<bool>,
    connecting: RwSignal<bool>,
    on_connect: impl Fn(web_sys::MouseEvent) + 'static,
    on_disconnect: impl Fn(web_sys::MouseEvent) + 'static,
    on_new_chat: impl Fn(web_sys::MouseEvent) + 'static,
    on_config: impl Fn(web_sys::MouseEvent) + 'static,
) -> impl IntoView {
    let status_class = move || {
        if connecting.get() {
            "status-dot connecting"
        } else if connected.get() {
            "status-dot connected"
        } else {
            "status-dot"
        }
    };

    let status_text = move || {
        if connecting.get() {
            "Connecting..."
        } else if connected.get() {
            "Connected"
        } else {
            "Disconnected"
        }
    };

    view! {
        <header class="header">
            <div class="header__title">
                <span class="header__logo">"Claude Bridge"</span>
                <div class="header__status">
                    <span class=status_class></span>
                    <span>{status_text}</span>
                </div>
            </div>

            <div class="header__controls">
                <Show
                    when=move || connected.get()
                    fallback=move || {
                        view! {
                            <button
                                class="btn btn--primary"
                                on:click=on_connect.clone()
                                disabled=move || connecting.get()
                            >
                                {move || if connecting.get() { "Connecting..." } else { "Connect" }}
                            </button>
                        }
                    }
                >
                    <button class="btn" on:click=on_new_chat.clone()>
                        "New Chat"
                    </button>
                    <button class="btn" on:click=on_disconnect.clone()>
                        "Disconnect"
                    </button>
                </Show>

                <button class="btn btn--ghost" on:click=on_config.clone() title="Settings">
                    "⚙"
                </button>
            </div>
        </header>
    }
}
