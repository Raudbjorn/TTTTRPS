use dioxus::prelude::*;

#[component]
pub fn GraphView() -> Element {
    // Mock Data for Graph
    let nodes = vec![
        ("npc-1", "Garrosh", 100.0, 100.0, "npc"),
        ("npc-2", "Elara", 200.0, 150.0, "npc"),
        ("loc-1", "Ironhold", 150.0, 300.0, "location"),
        ("fac-1", "Crimson Guard", 300.0, 200.0, "faction"),
    ];

    let edges = vec![
        (0, 2), // Garrosh -> Ironhold
        (1, 2), // Elara -> Ironhold
        (0, 3), // Garrosh -> Crimson Guard
    ];

    // Simple SVG rendering
    rsx! {
        div {
            class: "h-full w-full bg-zinc-950 relative overflow-hidden",

            // Toolbar
            div {
                class: "absolute top-4 right-4 bg-zinc-900 border border-zinc-800 rounded-lg p-2 flex gap-2 shadow-xl z-10",
                button { class: "p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white", "➕" }
                button { class: "p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white", "➖" }
                button { class: "p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white", "⟲" }
            }

            // Legend
            div {
                class: "absolute bottom-4 left-4 bg-zinc-900/80 border border-zinc-800 rounded-lg p-4 shadow-xl pointer-events-none",
                h3 { class: "text-xs font-bold uppercase text-zinc-500 mb-2", "Legend" }
                div { class: "space-y-1 text-xs",
                    div { class: "flex items-center gap-2", span { class: "w-3 h-3 rounded-full bg-blue-500" }, "NPC" }
                    div { class: "flex items-center gap-2", span { class: "w-3 h-3 rounded-full bg-emerald-500" }, "Location" }
                    div { class: "flex items-center gap-2", span { class: "w-3 h-3 rounded-full bg-purple-500" }, "Faction" }
                }
            }

            // Graph Canvas
            svg {
                class: "w-full h-full cursor-move",
                view_box: "0 0 800 600",

                // Edges
                g { class: "stroke-zinc-700 stroke-2",
                    for (start, end) in edges.iter() {
                        line {
                            x1: "{nodes[*start].2}",
                            y1: "{nodes[*start].3}",
                            x2: "{nodes[*end].2}",
                            y2: "{nodes[*end].3}"
                        }
                    }
                }

                // Nodes
                g {
                    for (i, (id, label, x, y, kind)) in nodes.iter().enumerate() {
                        {
                            let color_class = match *kind {
                                "npc" => "fill-blue-500",
                                "location" => "fill-emerald-500",
                                "faction" => "fill-purple-500",
                                _ => "fill-zinc-500",
                            };
                            rsx! {
                                g {
                                    class: "hover:opacity-80 transition-opacity cursor-pointer",
                                    circle {
                                        cx: "{x}",
                                        cy: "{y}",
                                        r: "20",
                                        class: "{color_class} stroke-zinc-900 stroke-4"
                                    }
                                    text {
                                        x: "{x}",
                                        y: "{y + 35.0}",
                                        text_anchor: "middle",
                                        class: "fill-zinc-300 text-xs font-medium select-none",
                                        "{label}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
