//! Graph View component for Leptos
//! Relationship/entity graph visualization using SVG

use leptos::prelude::*;

#[component]
pub fn GraphView() -> impl IntoView {
    // Mock Data for Graph
    let nodes: Vec<(&str, &str, f64, f64, &str)> = vec![
        ("npc-1", "Garrosh", 100.0, 100.0, "npc"),
        ("npc-2", "Elara", 200.0, 150.0, "npc"),
        ("loc-1", "Ironhold", 150.0, 300.0, "location"),
        ("fac-1", "Crimson Guard", 300.0, 200.0, "faction"),
    ];

    let edges: Vec<(usize, usize)> = vec![
        (0, 2), // Garrosh -> Ironhold
        (1, 2), // Elara -> Ironhold
        (0, 3), // Garrosh -> Crimson Guard
    ];

    view! {
        <div class="h-full w-full bg-zinc-950 relative overflow-hidden">
            // Toolbar
            <div class="absolute top-4 right-4 bg-zinc-900 border border-zinc-800 rounded-lg p-2 flex gap-2 shadow-xl z-10">
                <button class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white">"➕"</button>
                <button class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white">"➖"</button>
                <button class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white">"⟲"</button>
            </div>

            // Legend
            <div class="absolute bottom-4 left-4 bg-zinc-900/80 border border-zinc-800 rounded-lg p-4 shadow-xl pointer-events-none">
                <h3 class="text-xs font-bold uppercase text-zinc-500 mb-2">"Legend"</h3>
                <div class="space-y-1 text-xs">
                    <div class="flex items-center gap-2">
                        <span class="w-3 h-3 rounded-full bg-blue-500"></span>
                        "NPC"
                    </div>
                    <div class="flex items-center gap-2">
                        <span class="w-3 h-3 rounded-full bg-emerald-500"></span>
                        "Location"
                    </div>
                    <div class="flex items-center gap-2">
                        <span class="w-3 h-3 rounded-full bg-purple-500"></span>
                        "Faction"
                    </div>
                </div>
            </div>

            // Graph Canvas
            <svg class="w-full h-full cursor-move" viewBox="0 0 800 600">
                // Edges
                <g class="stroke-zinc-700" style="stroke-width: 2">
                    {edges.iter().map(|(start, end)| {
                        let (_, _, x1, y1, _) = nodes[*start];
                        let (_, _, x2, y2, _) = nodes[*end];
                        view! {
                            <line
                                x1=x1.to_string()
                                y1=y1.to_string()
                                x2=x2.to_string()
                                y2=y2.to_string()
                            />
                        }
                    }).collect_view()}
                </g>

                // Nodes
                <g>
                    {nodes.iter().map(|(_id, label, x, y, kind)| {
                        let color_class = match *kind {
                            "npc" => "fill-blue-500",
                            "location" => "fill-emerald-500",
                            "faction" => "fill-purple-500",
                            _ => "fill-zinc-500",
                        };
                        let text_y = y + 35.0;
                        view! {
                            <g class="hover:opacity-80 transition-opacity cursor-pointer">
                                <circle
                                    cx=x.to_string()
                                    cy=y.to_string()
                                    r="20"
                                    class=format!("{} stroke-zinc-900", color_class)
                                    style="stroke-width: 4"
                                />
                                <text
                                    x=x.to_string()
                                    y=text_y.to_string()
                                    text-anchor="middle"
                                    class="fill-zinc-300 text-xs font-medium select-none"
                                >
                                    {*label}
                                </text>
                            </g>
                        }
                    }).collect_view()}
                </g>
            </svg>
        </div>
    }
}
