use leptos::prelude::*;
use wasm_bindgen::JsCast;
use crate::services::layout_service::LayoutState;
use crate::components::layout::icon_rail::IconRail;
use crate::components::layout::media_bar::MediaBar;
use crate::components::resizable_panel::{DragHandle, ResizeSide};

#[component]
pub fn MainShell(
    sidebar: fn() -> AnyView,
    info_panel: fn() -> AnyView,
    children: Children,
) -> impl IntoView {
    let layout = expect_context::<LayoutState>();
    let dragging = RwSignal::new(Option::<ResizeSide>::None);
    let last_x = RwSignal::new(0.0_f64);

    // Computed grid template columns
    let grid_template_cols = Signal::derive(move || {
        let sidebar_w = layout.sidebar_width.get();
        let info_w = layout.infopanel_width.get();

        let sidebar_col = if layout.sidebar_visible.get() {
            format!("{}px", sidebar_w)
        } else {
            "0px".to_string()
        };

        let info_col = if layout.infopanel_visible.get() {
            format!("{}px", info_w)
        } else {
            "0px".to_string()
        };

        format!("64px {} 1fr {}", sidebar_col, info_col)
    });

    // Computed cursor style
    let cursor_style = Signal::derive(move || {
        if dragging.get().is_some() {
            "col-resize"
        } else {
            "default"
        }
    });

    // Mouse move handler for resizing
    let handle_mousemove = move |e: web_sys::MouseEvent| {
        if let Some(side) = dragging.get() {
            let current_x = e.page_x() as f64;
            match side {
                ResizeSide::Left => {
                    let new_w = ((current_x as i32) - 64).max(200).min(800);
                    layout.sidebar_width.set(new_w);
                }
                ResizeSide::Right => {
                    let delta = current_x - last_x.get();
                    let current_w = layout.infopanel_width.get();
                    let new_w = ((current_w as f64 - delta) as i32).max(250).min(800);
                    layout.infopanel_width.set(new_w);
                }
            }
            last_x.set(current_x);
        }
    };

    let handle_mouseup = move |_: web_sys::MouseEvent| {
        dragging.set(None);
    };

    let handle_mouseleave = move |_: web_sys::MouseEvent| {
        dragging.set(None);
    };

    // Sidebar drag start handler
    let on_sidebar_drag_start = Callback::new(move |_: ()| {
        dragging.set(Some(ResizeSide::Left));
    });

    // Info panel drag start handler
    let on_info_drag_start = Callback::new(move |_: ()| {
        if let Some(window) = web_sys::window() {
            let event = window.event();
            // JsValue.is_undefined() checks if it's undefined
            if !event.is_undefined() && !event.is_null() {
                if let Some(mouse_event) = event.dyn_ref::<web_sys::MouseEvent>() {
                    last_x.set(mouse_event.page_x() as f64);
                }
            }
        }
        dragging.set(Some(ResizeSide::Right));
    });

    // Visibility signals for conditional rendering
    let sidebar_visible = layout.sidebar_visible;
    let infopanel_visible = layout.infopanel_visible;

    view! {
        <div
            class="h-screen w-screen overflow-hidden bg-[var(--bg-deep)] text-[var(--text-primary)] font-ui transition-all duration-300 select-none"
            style:display="grid"
            style:grid-template-columns=move || grid_template_cols.get()
            style:grid-template-rows="1fr 56px"
            style:grid-template-areas="'rail sidebar main info' 'rail sidebar footer info'"
            style:cursor=move || cursor_style.get()
            on:mousemove=handle_mousemove
            on:mouseup=handle_mouseup
            on:mouseleave=handle_mouseleave
        >
            // Area: Rail
            <div style:grid-area="rail">
                <IconRail />
            </div>

            // Area: Sidebar
            <div
                style:grid-area="sidebar"
                class="border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-none relative"
            >
                {sidebar()}
                <Show when=move || sidebar_visible.get()>
                    <DragHandle
                        side=ResizeSide::Left
                        on_drag_start=on_sidebar_drag_start
                    />
                </Show>
            </div>

            // Area: Main
            <div
                style:grid-area="main"
                class="overflow-y-auto relative bg-[var(--bg-deep)]"
            >
                {children()}
            </div>

            // Area: Info
            <div
                style:grid-area="info"
                class="border-l border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-none relative"
            >
                {info_panel()}
                <Show when=move || infopanel_visible.get()>
                    <DragHandle
                        side=ResizeSide::Right
                        on_drag_start=on_info_drag_start
                    />
                </Show>
            </div>

            // Area: Footer
            <div
                style:grid-area="footer"
                class="border-t border-[var(--border-subtle)] bg-[var(--bg-elevated)] z-10"
            >
                <MediaBar />
            </div>
        </div>
    }
}
