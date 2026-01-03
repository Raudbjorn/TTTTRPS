//! Message input area component.

use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn InputArea<F>(on_send: F, disabled: Signal<bool>) -> impl IntoView
where
    F: Fn(String) + Clone + 'static,
{
    let input_value = RwSignal::new(String::new());
    let textarea_ref = NodeRef::<leptos::html::Textarea>::new();

    // Handle Enter key (without Shift)
    let on_send_clone = on_send.clone();
    let handle_keydown = move |ev: KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() {
            ev.prevent_default();
            let value = input_value.get();
            if !value.trim().is_empty() && !disabled.get() {
                on_send_clone(value);
                input_value.set(String::new());

                // Reset textarea height
                if let Some(el) = textarea_ref.get() {
                    el.style().set_property("height", "auto").ok();
                }
            }
        }
    };

    // Auto-resize textarea
    let handle_input = move |ev: leptos::ev::Event| {
        let target = event_target::<web_sys::HtmlTextAreaElement>(&ev);
        input_value.set(target.value());

        // Auto-resize
        target.style().set_property("height", "auto").ok();
        let scroll_height = target.scroll_height();
        target
            .style()
            .set_property("height", &format!("{}px", scroll_height.min(200)))
            .ok();
    };

    // Send button click
    let on_send_clone = on_send.clone();
    let handle_send = move |_| {
        let value = input_value.get();
        if !value.trim().is_empty() && !disabled.get() {
            on_send_clone(value);
            input_value.set(String::new());

            if let Some(el) = textarea_ref.get() {
                el.style().set_property("height", "auto").ok();
                el.focus().ok();
            }
        }
    };

    view! {
        <div class="input-area">
            <div class="input-container">
                <textarea
                    node_ref=textarea_ref
                    class="input-field"
                    placeholder="Type a message..."
                    rows="1"
                    prop:value=move || input_value.get()
                    on:input=handle_input
                    on:keydown=handle_keydown
                    disabled=move || disabled.get()
                ></textarea>
                <button
                    class="send-btn"
                    on:click=handle_send
                    disabled=move || disabled.get() || input_value.get().trim().is_empty()
                >
                    "Send"
                    <span style="font-size: 0.9em;">"↵"</span>
                </button>
            </div>
        </div>
    }
}
