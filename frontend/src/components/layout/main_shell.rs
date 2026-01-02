use dioxus::prelude::*;
use crate::services::layout_service::LayoutState;
use crate::components::layout::icon_rail::IconRail;
use crate::components::layout::media_bar::MediaBar;

#[derive(Props, Clone, PartialEq)]
pub struct MainShellProps {
    children: Element,
    // Slots for sidebar and info panel content
    sidebar: Element,
    info_panel: Element,
}

use crate::components::resizable_panel::{DragHandle, ResizeSide};

#[component]
pub fn MainShell(props: MainShellProps) -> Element {
    let mut layout = use_context::<LayoutState>();
    let mut dragging = use_signal(|| Option::<ResizeSide>::None);
    let mut window_width = use_signal(|| 1920.0);

    use wasm_bindgen::prelude::*;
    use wasm_bindgen::JsCast;

    use_effect(move || {
        // We use a clone of layout for the closure
        let mut layout_svc = layout;

        let mut handle_resize = Closure::wrap(Box::new(move || {
             if let Some(window) = web_sys::window() {
                 if let Ok(w) = window.inner_width() {
                     let width = w.as_f64().unwrap_or(1920.0);
                     window_width.set(width);

                     // Responsive Logic (Simple Auto-Collapse)
                     // <900: All collapsed (Drawer mode pending)
                     // 900-1200: Sidebar collapsed, Info hidden
                     // 1200-1400: Sidebar visible, Info hidden
                     // >=1400: Both visible

                     // Only apply if we haven't manually overridden?
                     // For now, we enforce "Smart Defaults" purely based on width to satisfy requirement.

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
             let _ = window.add_event_listener_with_callback("resize", handle_resize.as_ref().unchecked_ref());
             // Trigger once
             let _ = handle_resize.as_ref().unchecked_ref::<js_sys::Function>().call0(&JsValue::NULL);
        }
        handle_resize.forget();
    });

    // Dynamic Grid Columns calculation
    let sidebar_w = *layout.sidebar_width.read();
    let info_w = *layout.infopanel_width.read();
    let width = *window_width.read();
    let is_mobile = width < 900.0;

    let sidebar_col = if is_mobile {
         "0px".to_string()
    } else if *layout.sidebar_visible.read() {
        format!("{}px", sidebar_w)
    } else {
        "0px".to_string()
    };

    let info_col = if *layout.infopanel_visible.read() { format!("{}px", info_w) } else { "0px".to_string() };

    let grid_template_cols = format!("64px {} 1fr {}", sidebar_col, info_col);

    // Handlers
    let handle_mousemove = move |e: MouseEvent| {
        if let Some(side) = *dragging.read() {
            e.stop_propagation();
             match side {
                ResizeSide::Left => {
                    // Sidebar resizing (absolute position from left rail)
                    let x = e.page_coordinates().x as i32;
                    let new_w = (x - 64).max(200).min(600);
                    layout.sidebar_width.set(new_w);
                }
                ResizeSide::Right => {
                    // TODO: Implement right-side resizing here, mirroring the delta-based logic in `handle_move_container`.
                }
            }
        }
    };

    // We need state for tracking delta
    let mut last_x = use_signal(|| 0.0);

    let handle_move_container = move |e: MouseEvent| {
         if let Some(side) = *dragging.read() {
             let current_x = e.page_coordinates().x;
             match side {
                 ResizeSide::Left => {
                     let new_w = (current_x as i32 - 64).max(200).min(800);
                     layout.sidebar_width.set(new_w);
                 }
                 ResizeSide::Right => {
                     let delta = current_x - *last_x.read();
                     let current_w = *layout.infopanel_width.read();
                     let new_w = (current_w as f64 - delta) as i32;
                     let new_w = new_w.max(250).min(800);
                     layout.infopanel_width.set(new_w);
                 }
             }
             last_x.set(current_x);
         }
    };

    let handle_up = move |_| {
        dragging.set(None);
    };

    let cursor_style = if dragging.read().is_some() { "col-resize" } else { "default" };
    let grid_style = format!(
        "display: grid; grid-template-columns: {}; grid-template-rows: 1fr 56px; grid-template-areas: 'rail sidebar main info' 'rail sidebar footer info'; cursor: {};",
        grid_template_cols, cursor_style
    );

    // Sidebar Class Logic
    let sidebar_classes = if is_mobile {
        if *layout.sidebar_visible.read() {
            "fixed left-[64px] top-0 bottom-[56px] w-[300px] z-50 shadow-2xl border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-transform duration-300"
        } else {
            "hidden"
        }
    } else {
        "grid-area: sidebar; border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-none relative"
    };

    // For inline style when normal
    let sidebar_style_attr = if is_mobile { "" } else { "grid-area: sidebar;" };

    // Backdrop for mobile
    let show_backdrop = is_mobile && *layout.sidebar_visible.read();

    rsx! {
        div {
            class: "h-screen w-screen overflow-hidden bg-[var(--bg-deep)] text-[var(--text-primary)] font-ui transition-all duration-300 select-none",
            style: "{grid_style}",

            onmousemove: handle_move_container,
            onmouseup: handle_up,
            onmouseleave: handle_up,

            // Area: Rail
            div { style: "grid-area: rail;", IconRail {} }

            // Mobile Backdrop
            if show_backdrop {
                div {
                    class: "fixed inset-0 bg-black/50 z-40 backdrop-blur-sm ml-[64px]",
                    onclick: move |_| layout.sidebar_visible.set(false)
                }
            }

            // Area: Sidebar (Drawer or Grid)
            div {
                class: "{sidebar_classes}",
                style: "{sidebar_style_attr}",
                {props.sidebar}
                // Drag Handle (Only if not mobile)
                if !is_mobile && *layout.sidebar_visible.read() {
                    DragHandle {
                        side: ResizeSide::Left,
                        on_drag_start: move |e: MouseEvent| {
                            last_x.set(e.page_coordinates().x); // Init
                            dragging.set(Some(ResizeSide::Left));
                        }
                    }
                }
            }

            // Area: Main
            div {
                style: "grid-area: main;",
                class: "overflow-y-auto relative bg-[var(--bg-deep)]",
                {props.children}
            }

            // Area: Info
            div {
                style: "grid-area: info;",
                class: "border-l border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-none relative",
                {props.info_panel}
                // Drag Handle
                if *layout.infopanel_visible.read() {
                    DragHandle {
                        side: ResizeSide::Right,
                        on_drag_start: move |e: MouseEvent| {
                            last_x.set(e.page_coordinates().x); // Init start pos
                            dragging.set(Some(ResizeSide::Right));
                        }
                    }
                }
            }

            // Area: Footer
            div {
                style: "grid-area: footer;",
                class: "border-t border-[var(--border-subtle)] bg-[var(--bg-elevated)] z-10",
                MediaBar {}
            }
        }
    }
}
