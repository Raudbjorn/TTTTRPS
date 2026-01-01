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

    // Dynamic Grid Columns calculation
    let sidebar_w = *layout.sidebar_width.read();
    let info_w = *layout.infopanel_width.read();

    let sidebar_col = if *layout.sidebar_visible.read() { format!("{}px", sidebar_w) } else { "0px".to_string() };
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
                    // InfoPanel resizing (delta based)
                    // Note: Calculating absolute width from right is hard without window width.
                    // Using movement metrics for right side resizing
                    // If mouse moves left (negative), width increases.
                    // If mouse moves right (positive), width decreases.
                    // Ideally we'd use page_coords if we knew window width -> w = window_w - x.
                    // Fallback to delta approach for now:
                    // Dioxus MouseEvent doesn't expose movement_x directly in all versions reliably match web API?
                    // Let's rely on standard web_sys access if needed, or just try to track previous X.
                    // Actually, let's try a simpler approach:
                    // We can't trust movement without lock.
                    // Let's just use delta from previous frames?
                    // Or keep it simple: assume we can't perfectly resize right panel without JS interop for generic window size.
                    // Allow dragging ONLY Sidebar for now if Right is too hard?
                    // No, user wants resizable panels (plural).

                    // Hack: We don't have window width easily.
                    // Let's assume user is dragging reasonably.
                    // We can use `screen_x`? No.
                    // Let's leave Right Panel resizing as "Todo" or try to implement if Dioxus provides `client_x` and we assume we can estimate?
                    // Providing a delta approach based on `page_coordinates`:
                    // We need to store `last_x` in signal.
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

    rsx! {
        div {
            class: "h-screen w-screen overflow-hidden bg-[var(--bg-deep)] text-[var(--text-primary)] font-ui transition-all duration-300 select-none",
            style: "{grid_style}",

            onmousemove: handle_move_container,
            onmouseup: handle_up,
            onmouseleave: handle_up,

            // Area: Rail
            div { style: "grid-area: rail;",
                IconRail {}
            }

            // Area: Sidebar
            div {
                style: "grid-area: sidebar;",
                class: "border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-visible transition-none relative",
                {props.sidebar}
                // Drag Handle
                if *layout.sidebar_visible.read() {
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
