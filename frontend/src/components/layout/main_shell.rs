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

#[component]
pub fn MainShell(props: MainShellProps) -> Element {
    let layout = use_context::<LayoutState>();

    // Dynamic Grid Columns calculation
    // Rail (64px) | Sidebar (Auto/0) | Main (1fr) | Info (Auto/0)
    let sidebar_width = if *layout.sidebar_visible.read() { "280px" } else { "0px" };
    let info_width = if *layout.infopanel_visible.read() { "320px" } else { "0px" };

    let grid_template_cols = format!("64px {} 1fr {}", sidebar_width, info_width);

    // Dynamic Areas
    // "rail sidebar main info"
    // "rail sidebar footer info"

    rsx! {
        div {
            class: "h-screen w-screen overflow-hidden bg-[var(--bg-deep)] text-[var(--text-primary)] font-ui transition-all duration-300",
            style: "display: grid;
                   grid-template-columns: {grid_template_cols};
                   grid-template-rows: 1fr 56px;
                   grid-template-areas: 'rail sidebar main info' 'rail sidebar footer info';",

            // Area: Rail
            div { style: "grid-area: rail;",
                IconRail {}
            }

            // Area: Sidebar
            div {
                style: "grid-area: sidebar;",
                class: "border-r border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-hidden transition-all duration-300 relative",
                {props.sidebar}
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
                class: "border-l border-[var(--border-subtle)] bg-[var(--bg-surface)] overflow-hidden transition-all duration-300",
                {props.info_panel}
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
