//! MainShell Layout Component
//!
//! Implements a 5-panel CSS Grid layout with responsive behavior and keyboard shortcuts.
//! Layout structure:
//!   - Rail: 64px fixed icon navigation
//!   - Sidebar: Collapsible context panel (280-500px)
//!   - Main: Primary content area (1fr)
//!   - Info: Collapsible info panel (250-600px)
//!   - Footer: Media bar (56px fixed)
//!
//! Keyboard shortcuts:
//!   - Cmd+. (or Ctrl+.) : Toggle sidebar
//!   - Cmd+/ (or Ctrl+/) : Toggle info panel

use leptos::prelude::*;
use wasm_bindgen::prelude::*;
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
    let window_width = RwSignal::new(1920.0_f64);

    // Keyboard shortcut effect
    Effect::new(move |_| {
        let layout_for_keys = layout;

        let handle_keydown = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
            // Check for Cmd (Mac) or Ctrl (Windows/Linux)
            let modifier = event.meta_key() || event.ctrl_key();

            if modifier {
                match event.key().as_str() {
                    // Cmd+. or Ctrl+. toggles sidebar
                    "." => {
                        event.prevent_default();
                        layout_for_keys.toggle_sidebar();
                    }
                    // Cmd+/ or Ctrl+/ toggles info panel
                    "/" => {
                        event.prevent_default();
                        layout_for_keys.toggle_infopanel();
                    }
                    _ => {}
                }
            }
        }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                let _ = document.add_event_listener_with_callback(
                    "keydown",
                    handle_keydown.as_ref().unchecked_ref(),
                );
            }
        }
        handle_keydown.forget();
    });

    // Responsive resize listener
    Effect::new(move |_| {
        let layout_svc = layout;

        let handle_resize = Closure::wrap(Box::new(move || {
            if let Some(window) = web_sys::window() {
                if let Ok(w) = window.inner_width() {
                    let width = w.as_f64().unwrap_or(1920.0);
                    window_width.set(width);

                    // Responsive Logic (Smart Auto-Collapse)
                    // <900: All collapsed (Drawer mode)
                    // 900-1200: Sidebar visible, Info hidden
                    // >=1200: Both visible

                    if width < 1200.0 {
                        layout_svc.infopanel_visible.set(false);
                    } else {
                        layout_svc.infopanel_visible.set(true);
                    }

                    if width < 900.0 {
                        layout_svc.sidebar_visible.set(false);
                    } else {
                        layout_svc.sidebar_visible.set(true);
                    }
                }
            }
        }) as Box<dyn FnMut()>);

        if let Some(window) = web_sys::window() {
            let _ = window.add_event_listener_with_callback(
                "resize",
                handle_resize.as_ref().unchecked_ref(),
            );
            // Trigger once on mount
            let _ = handle_resize
                .as_ref()
                .unchecked_ref::<js_sys::Function>()
                .call0(&JsValue::NULL);
        }
        handle_resize.forget();
    });

    let is_mobile = Signal::derive(move || window_width.get() < 900.0);

    // Computed grid template columns
    let rail_width_px = Signal::derive(move || if layout.text_navigation.get() { 200 } else { 64 });

    let grid_template_cols = Signal::derive(move || {
        let sidebar_w = layout.sidebar_width.get();
        let info_w = layout.infopanel_width.get();
        let rail_w = rail_width_px.get();

        let sidebar_col = if is_mobile.get() {
            "0px".to_string()
        } else if layout.sidebar_visible.get() {
            format!("{}px", sidebar_w)
        } else {
            "0px".to_string()
        };

        let info_col = if layout.infopanel_visible.get() {
            format!("{}px", info_w)
        } else {
            "0px".to_string()
        };

        format!("{}px {} 1fr {}", rail_w, sidebar_col, info_col)
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
            let rail_w = rail_width_px.get();

            match side {
                ResizeSide::Left => {
                    let new_w = ((current_x as i32) - rail_w).max(200).min(800);
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

    // Derived signals for mobile drawer
    let show_backdrop = Signal::derive(move || is_mobile.get() && sidebar_visible.get());

    // Sidebar class based on mobile/desktop
    let sidebar_class = Signal::derive(move || {
        let rail_w = if layout.text_navigation.get() { "200px" } else { "64px" };
        if is_mobile.get() {
            if sidebar_visible.get() {
                format!("fixed left-[{}] top-0 bottom-[56px] w-[300px] z-50 shadow-2xl border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-transform duration-300", rail_w)
            } else {
                "hidden".to_string()
            }
        } else {
            "border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-none relative".to_string()
        }
    });

    view! {
        <div
            class="h-screen w-screen overflow-hidden bg-hero text-[var(--text-primary)] font-ui transition-all duration-300 select-none"
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
            <div style:grid-area="rail" class="relative z-50">
                <IconRail />
            </div>

            // Mobile Backdrop
            <Show when=move || show_backdrop.get()>
                <div
                    class=move || format!("fixed inset-0 bg-black/50 z-40 backdrop-blur-sm ml-[{}]", if layout.text_navigation.get() { "200px" } else { "64px" })
                    on:click=move |_| layout.sidebar_visible.set(false)
                />
            </Show>

            // Area: Sidebar (Drawer or Grid)
            <div
                style:grid-area=move || if is_mobile.get() { "" } else { "sidebar" }
                class=move || {
                    let base = sidebar_class.get();
                    // Append glass effect if not mobile drawer (drawer has its own styling in the signal)
                    if !is_mobile.get() {
                        format!("{} panel-glass border-r-0 my-2 ml-2", base)
                    } else {
                        base.to_string()
                    }
                }
            >
                {sidebar()}
                // Drag Handle (Only if not mobile)
                <Show when=move || !is_mobile.get() && sidebar_visible.get()>
                    <DragHandle
                        side=ResizeSide::Left
                        on_drag_start=on_sidebar_drag_start
                    />
                </Show>
            </div>

            // Area: Main
            <div
                style:grid-area="main"
                class="overflow-y-auto relative scrollbar-none"
            >
                {children()}
            </div>

            // Area: Info
            <div
                style:grid-area="info"
                class="panel-glass overflow-visible transition-none relative my-2 mr-2"
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
                class="panel-glass z-10 m-2 mt-0"
            >
                <MediaBar />
            </div>
        </div>
    }
}
