//! Relationship Graph Component (TASK-009)
//!
//! Interactive SVG-based graph visualization for entity relationships.
//! Supports pan, zoom, node selection, and ego-graph filtering.

use leptos::ev;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use crate::bindings::{
    get_entity_graph, get_ego_graph, EntityGraph, GraphNode, GraphEdge, GraphStats,
};

/// Graph layout configuration
#[derive(Debug, Clone)]
struct LayoutConfig {
    width: f64,
    height: f64,
    node_radius: f64,
    force_strength: f64,
    link_distance: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            node_radius: 20.0,
            force_strength: -300.0,
            link_distance: 100.0,
        }
    }
}

/// Positioned node for rendering
#[derive(Debug, Clone)]
struct PositionedNode {
    node: GraphNode,
    x: f64,
    y: f64,
}

/// Graph legend component
#[component]
fn GraphLegend(entity_counts: HashMap<String, usize>) -> impl IntoView {
    let entity_colors = vec![
        ("PC", "#3b82f6"),
        ("NPC", "#8b5cf6"),
        ("Location", "#10b981"),
        ("Faction", "#f59e0b"),
        ("Item", "#ec4899"),
        ("Event", "#6366f1"),
        ("Quest", "#14b8a6"),
        ("Deity", "#f97316"),
        ("Creature", "#ef4444"),
    ];

    view! {
        <div class="absolute bottom-4 left-4 bg-zinc-900/90 border border-zinc-800 rounded-lg p-4 shadow-xl pointer-events-none">
            <h3 class="text-xs font-bold uppercase text-zinc-500 mb-3">"Entity Types"</h3>
            <div class="space-y-2 text-xs">
                {entity_colors.into_iter().filter_map(|(etype, color)| {
                    let count = entity_counts.get(etype).copied().unwrap_or(0);
                    if count > 0 {
                        Some(view! {
                            <div class="flex items-center gap-2">
                                <span
                                    class="w-3 h-3 rounded-full"
                                    style=format!("background-color: {}", color)
                                ></span>
                                <span class="text-zinc-300">{etype}</span>
                                <span class="text-zinc-500">{format!("({})", count)}</span>
                            </div>
                        })
                    } else {
                        None
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

/// Graph stats panel component
#[component]
fn GraphStatsPanel(stats: GraphStats) -> impl IntoView {
    view! {
        <div class="absolute top-4 left-4 bg-zinc-900/90 border border-zinc-800 rounded-lg p-4 shadow-xl">
            <h3 class="text-xs font-bold uppercase text-zinc-500 mb-3">"Graph Stats"</h3>
            <div class="space-y-1 text-sm">
                <div class="flex justify-between gap-4">
                    <span class="text-zinc-500">"Entities"</span>
                    <span class="text-white font-medium">{stats.node_count}</span>
                </div>
                <div class="flex justify-between gap-4">
                    <span class="text-zinc-500">"Relationships"</span>
                    <span class="text-white font-medium">{stats.edge_count}</span>
                </div>
            </div>
            {if !stats.most_connected_entities.is_empty() {
                Some(view! {
                    <div class="mt-3 pt-3 border-t border-zinc-800">
                        <h4 class="text-xs font-bold uppercase text-zinc-500 mb-2">"Most Connected"</h4>
                        <div class="space-y-1 text-xs">
                            {stats.most_connected_entities.iter().take(3).map(|(name, count)| {
                                view! {
                                    <div class="flex justify-between gap-2">
                                        <span class="text-zinc-400 truncate">{name.clone()}</span>
                                        <span class="text-purple-400">{count.to_string()}</span>
                                    </div>
                                }
                            }).collect_view()}
                        </div>
                    </div>
                })
            } else {
                None
            }}
        </div>
    }
}

/// Toolbar component
#[component]
fn GraphToolbar(
    zoom_level: RwSignal<f64>,
    show_labels: RwSignal<bool>,
    show_inactive: RwSignal<bool>,
    on_refresh: Callback<()>,
    on_reset_view: Callback<()>,
) -> impl IntoView {
    let handle_zoom_in = move |_: ev::MouseEvent| {
        zoom_level.update(|z| *z = (*z * 1.2).min(3.0));
    };

    let handle_zoom_out = move |_: ev::MouseEvent| {
        zoom_level.update(|z| *z = (*z / 1.2).max(0.3));
    };

    let handle_reset = move |_: ev::MouseEvent| {
        on_reset_view.run(());
    };

    let handle_refresh = move |_: ev::MouseEvent| {
        on_refresh.run(());
    };

    let handle_toggle_labels = move |_: ev::MouseEvent| {
        show_labels.update(|v| *v = !*v);
    };

    let handle_toggle_inactive = move |_: ev::MouseEvent| {
        show_inactive.update(|v| *v = !*v);
    };

    view! {
        <div class="absolute top-4 right-4 bg-zinc-900/90 border border-zinc-800 rounded-lg shadow-xl z-10">
            <div class="flex">
                // Zoom controls
                <div class="flex border-r border-zinc-800">
                    <button
                        class="p-2.5 hover:bg-zinc-800 text-zinc-400 hover:text-white transition-colors rounded-l-lg"
                        title="Zoom In"
                        on:click=handle_zoom_in
                    >
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 4v16m8-8H4" />
                        </svg>
                    </button>
                    <button
                        class="p-2.5 hover:bg-zinc-800 text-zinc-400 hover:text-white transition-colors"
                        title="Zoom Out"
                        on:click=handle_zoom_out
                    >
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 12H4" />
                        </svg>
                    </button>
                    <button
                        class="p-2.5 hover:bg-zinc-800 text-zinc-400 hover:text-white transition-colors"
                        title="Reset View"
                        on:click=handle_reset
                    >
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                        </svg>
                    </button>
                </div>

                // View toggles
                <div class="flex border-r border-zinc-800">
                    <button
                        class=move || format!(
                            "p-2.5 hover:bg-zinc-800 transition-colors {}",
                            if show_labels.get() { "text-purple-400" } else { "text-zinc-400 hover:text-white" }
                        )
                        title="Toggle Labels"
                        on:click=handle_toggle_labels
                    >
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z" />
                        </svg>
                    </button>
                    <button
                        class=move || format!(
                            "p-2.5 hover:bg-zinc-800 transition-colors {}",
                            if show_inactive.get() { "text-purple-400" } else { "text-zinc-400 hover:text-white" }
                        )
                        title="Show Inactive Relationships"
                        on:click=handle_toggle_inactive
                    >
                        <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                        </svg>
                    </button>
                </div>

                // Refresh
                <button
                    class="p-2.5 hover:bg-zinc-800 text-zinc-400 hover:text-white transition-colors rounded-r-lg"
                    title="Refresh Graph"
                    on:click=handle_refresh
                >
                    <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
                    </svg>
                </button>
            </div>
        </div>
    }
}

/// Node tooltip component
#[component]
fn NodeTooltip(
    node: GraphNode,
    x: f64,
    y: f64,
) -> impl IntoView {
    view! {
        <div
            class="absolute bg-zinc-900 border border-zinc-700 rounded-lg p-3 shadow-xl pointer-events-none z-20"
            style=format!("left: {}px; top: {}px; transform: translate(-50%, -120%);", x, y)
        >
            <div class="text-sm font-medium text-white">{node.name.clone()}</div>
            <div class="text-xs text-zinc-400 mt-1">{node.entity_type.clone()}</div>
            <div class="text-xs text-zinc-500 mt-1">
                {format!("{} connections", node.connection_count)}
            </div>
            {if node.is_hub {
                Some(view! {
                    <span class="inline-block mt-2 px-1.5 py-0.5 text-xs bg-purple-900/50 text-purple-300 rounded">
                        "Hub Entity"
                    </span>
                })
            } else {
                None
            }}
        </div>
    }
}

/// Calculate simple force-directed layout
fn calculate_layout(nodes: &[GraphNode], edges: &[GraphEdge], config: &LayoutConfig) -> Vec<PositionedNode> {
    let mut positioned: Vec<PositionedNode> = nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            // Initial circular layout
            let angle = (i as f64) * 2.0 * std::f64::consts::PI / (nodes.len().max(1) as f64);
            let radius = config.width.min(config.height) * 0.35;
            PositionedNode {
                node: node.clone(),
                x: config.width / 2.0 + radius * angle.cos(),
                y: config.height / 2.0 + radius * angle.sin(),
            }
        })
        .collect();

    // Simple force simulation (a few iterations)
    for _ in 0..50 {
        // Repulsion between all nodes
        for i in 0..positioned.len() {
            for j in (i + 1)..positioned.len() {
                let dx = positioned[j].x - positioned[i].x;
                let dy = positioned[j].y - positioned[i].y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let force = config.force_strength / dist;

                let fx = dx / dist * force * 0.1;
                let fy = dy / dist * force * 0.1;

                positioned[i].x -= fx;
                positioned[i].y -= fy;
                positioned[j].x += fx;
                positioned[j].y += fy;
            }
        }

        // Attraction along edges
        for edge in edges {
            let source_idx = positioned.iter().position(|n| n.node.id == edge.source);
            let target_idx = positioned.iter().position(|n| n.node.id == edge.target);

            if let (Some(si), Some(ti)) = (source_idx, target_idx) {
                let dx = positioned[ti].x - positioned[si].x;
                let dy = positioned[ti].y - positioned[si].y;
                let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                let force = (dist - config.link_distance) * 0.05;

                let fx = dx / dist * force;
                let fy = dy / dist * force;

                positioned[si].x += fx;
                positioned[si].y += fy;
                positioned[ti].x -= fx;
                positioned[ti].y -= fy;
            }
        }

        // Center constraint
        let center_x: f64 = positioned.iter().map(|n| n.x).sum::<f64>() / positioned.len().max(1) as f64;
        let center_y: f64 = positioned.iter().map(|n| n.y).sum::<f64>() / positioned.len().max(1) as f64;

        for node in &mut positioned {
            node.x += (config.width / 2.0 - center_x) * 0.1;
            node.y += (config.height / 2.0 - center_y) * 0.1;

            // Keep within bounds
            node.x = node.x.max(config.node_radius * 2.0).min(config.width - config.node_radius * 2.0);
            node.y = node.y.max(config.node_radius * 2.0).min(config.height - config.node_radius * 2.0);
        }
    }

    positioned
}

/// Entity filter component
#[component]
fn EntityFilter(
    entity_id: RwSignal<Option<String>>,
    nodes: Vec<GraphNode>,
    on_filter: Callback<Option<String>>,
) -> impl IntoView {
    let handle_change = move |evt: ev::Event| {
        let value = event_target_value(&evt);
        let new_value = if value.is_empty() { None } else { Some(value) };
        entity_id.set(new_value.clone());
        on_filter.run(new_value);
    };

    view! {
        <div class="absolute bottom-4 right-4 bg-zinc-900/90 border border-zinc-800 rounded-lg p-3 shadow-xl">
            <label class="block text-xs font-bold uppercase text-zinc-500 mb-2">
                "Focus on Entity"
            </label>
            <select
                class="w-48 px-3 py-1.5 bg-zinc-800 border border-zinc-700 rounded text-sm text-white focus:border-purple-500 focus:outline-none"
                prop:value=move || entity_id.get().unwrap_or_default()
                on:change=handle_change
            >
                <option value="">"All Entities"</option>
                {nodes.into_iter().map(|node| {
                    view! {
                        <option value=node.id.clone()>
                            {format!("{} ({})", node.name, node.entity_type)}
                        </option>
                    }
                }).collect_view()}
            </select>
        </div>
    }
}

/// Main relationship graph component
#[component]
pub fn RelationshipGraph(
    /// Campaign ID
    campaign_id: String,
    /// Optional: Focus on a specific entity (ego graph)
    #[prop(optional)]
    focus_entity_id: Option<String>,
    /// Callback when a node is selected
    #[prop(optional)]
    on_node_select: Option<Callback<String>>,
) -> impl IntoView {
    // State
    let graph = RwSignal::new(Option::<EntityGraph>::None);
    let positioned_nodes = RwSignal::new(Vec::<PositionedNode>::new());
    let is_loading = RwSignal::new(true);
    let error = RwSignal::new(Option::<String>::None);

    // View state
    let zoom_level = RwSignal::new(1.0_f64);
    let pan_offset = RwSignal::new((0.0_f64, 0.0_f64));
    let show_labels = RwSignal::new(true);
    let show_inactive = RwSignal::new(false);
    let selected_node = RwSignal::new(Option::<String>::None);
    let hovered_node = RwSignal::new(Option::<(GraphNode, f64, f64)>::None);
    let focus_entity = RwSignal::new(focus_entity_id);

    let config = LayoutConfig::default();

    // Load graph data
    let campaign_id_load = campaign_id.clone();
    Effect::new({
        let config = config.clone();
        move |_| {
            let cid = campaign_id_load.clone();
            let include_inactive = show_inactive.get();
            let focus_id = focus_entity.get();
            let layout_config = config.clone();

            spawn_local(async move {
                is_loading.set(true);
                error.set(None);

                let result = if let Some(entity_id) = focus_id {
                    get_ego_graph(cid, entity_id, Some(2)).await
                } else {
                    get_entity_graph(cid, Some(include_inactive)).await
                };

                match result {
                    Ok(g) => {
                        let layout = calculate_layout(&g.nodes, &g.edges, &layout_config);
                        positioned_nodes.set(layout);
                        graph.set(Some(g));
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                is_loading.set(false);
            });
        }
    });

    // Refresh handler
    let campaign_id_refresh = campaign_id.clone();
    let handle_refresh = Callback::new({
        let config = config.clone();
        move |_: ()| {
            let cid = campaign_id_refresh.clone();
            let include_inactive = show_inactive.get();
            let focus_id = focus_entity.get();
            let layout_config = config.clone();

            spawn_local(async move {
                is_loading.set(true);
                let result = if let Some(entity_id) = focus_id {
                    get_ego_graph(cid, entity_id, Some(2)).await
                } else {
                    get_entity_graph(cid, Some(include_inactive)).await
                };

                match result {
                    Ok(g) => {
                        let layout = calculate_layout(&g.nodes, &g.edges, &layout_config);
                        positioned_nodes.set(layout);
                        graph.set(Some(g));
                    }
                    Err(e) => {
                        error.set(Some(e));
                    }
                }
                is_loading.set(false);
            });
        }
    });

    let handle_reset_view = Callback::new(move |_: ()| {
        zoom_level.set(1.0);
        pan_offset.set((0.0, 0.0));
    });

    let handle_node_click = move |node_id: String| {
        selected_node.set(Some(node_id.clone()));
        if let Some(ref cb) = on_node_select {
            cb.run(node_id);
        }
    };

    let handle_entity_filter = Callback::new(move |entity_id: Option<String>| {
        focus_entity.set(entity_id);
    });

    view! {
        <div class="h-full w-full bg-zinc-950 relative overflow-hidden rounded-lg border border-zinc-800">
            // Loading overlay
            <Show when=move || is_loading.get()>
                <div class="absolute inset-0 bg-zinc-950/80 flex items-center justify-center z-30">
                    <div class="text-zinc-400 flex items-center gap-2">
                        <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
                            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none" />
                            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                        </svg>
                        "Loading graph..."
                    </div>
                </div>
            </Show>

            // Error message
            {move || error.get().map(|e| view! {
                <div class="absolute inset-0 flex items-center justify-center z-30">
                    <div class="bg-red-900/50 text-red-300 px-4 py-2 rounded-lg">
                        {format!("Error: {}", e)}
                    </div>
                </div>
            })}

            // Toolbar
            <GraphToolbar
                zoom_level=zoom_level
                show_labels=show_labels
                show_inactive=show_inactive
                on_refresh=handle_refresh
                on_reset_view=handle_reset_view
            />

            // Stats panel
            {move || graph.get().map(|g| view! {
                <GraphStatsPanel stats=g.stats.clone() />
            })}

            // Legend
            {move || graph.get().map(|g| view! {
                <GraphLegend entity_counts=g.stats.entity_type_counts.clone() />
            })}

            // Entity filter dropdown
            {move || graph.get().map(|g| view! {
                <EntityFilter
                    entity_id=focus_entity
                    nodes=g.nodes.clone()
                    on_filter=handle_entity_filter
                />
            })}

            // SVG Graph Canvas
            {
                let config = config.clone();
                view! {
                    <svg
                        class="w-full h-full cursor-move"
                        viewBox=move || format!("0 0 {} {}", config.width, config.height)
                        style=move || format!(
                            "transform: scale({}) translate({}px, {}px);",
                            zoom_level.get(),
                            pan_offset.get().0,
                            pan_offset.get().1
                        )
                    >
                // Edges
                <g>
                    {move || {
                        let nodes = positioned_nodes.get();
                        let g = graph.get();
                        let show_edge_labels = show_labels.get();

                        g.map(|graph| {
                            graph.edges.iter().filter_map(|edge| {
                                // Skip inactive if not showing them
                                if !edge.is_active && !show_inactive.get() {
                                    return None;
                                }

                                let source = nodes.iter().find(|n| n.node.id == edge.source)?;
                                let target = nodes.iter().find(|n| n.node.id == edge.target)?;

                                let mid_x = (source.x + target.x) / 2.0;
                                let mid_y = (source.y + target.y) / 2.0;

                                let opacity = if edge.is_active { "0.7" } else { "0.3" };
                                let stroke_width = ((edge.strength as f64) / 25.0).max(1.0);

                                Some(view! {
                                    <g>
                                        // Edge line
                                        <line
                                            x1=source.x.to_string()
                                            y1=source.y.to_string()
                                            x2=target.x.to_string()
                                            y2=target.y.to_string()
                                            stroke=edge.color.clone()
                                            stroke-width=stroke_width.to_string()
                                            style=format!("opacity: {}", opacity)
                                        />
                                        // Arrowhead for directed edges
                                        {if !edge.bidirectional {
                                            let angle = (target.y - source.y).atan2(target.x - source.x);
                                            let arrow_len = 10.0;
                                            let arrow_x = target.x - config.node_radius * angle.cos();
                                            let arrow_y = target.y - config.node_radius * angle.sin();
                                            Some(view! {
                                                <polygon
                                                    points=format!(
                                                        "{},{} {},{} {},{}",
                                                        arrow_x,
                                                        arrow_y,
                                                        arrow_x - arrow_len * (angle + 0.3).cos(),
                                                        arrow_y - arrow_len * (angle + 0.3).sin(),
                                                        arrow_x - arrow_len * (angle - 0.3).cos(),
                                                        arrow_y - arrow_len * (angle - 0.3).sin()
                                                    )
                                                    fill=edge.color.clone()
                                                    style=format!("opacity: {}", opacity)
                                                />
                                            })
                                        } else {
                                            None
                                        }}
                                        // Edge label
                                        {if show_edge_labels {
                                            Some(view! {
                                                <text
                                                    x=mid_x.to_string()
                                                    y=(mid_y - 5.0).to_string()
                                                    text-anchor="middle"
                                                    class="text-[9px] fill-zinc-500 select-none pointer-events-none"
                                                    style=format!("opacity: {}", opacity)
                                                >
                                                    {edge.label.clone()}
                                                </text>
                                            })
                                        } else {
                                            None
                                        }}
                                    </g>
                                })
                            }).collect_view()
                        }).unwrap_or_default()
                    }}
                </g>
                // Nodes
                <g>
                    {move || {
                        let nodes = positioned_nodes.get();
                        let selected = selected_node.get();

                        nodes.into_iter().map(|pn| {
                            let node_id = pn.node.id.clone();
                            let node_id_click = pn.node.id.clone();
                            let node_for_hover = pn.node.clone();
                            let x = pn.x;
                            let y = pn.y;

                            let is_selected = selected.as_ref() == Some(&pn.node.id);
                            let is_hub = pn.node.is_hub;

                            let node_radius = if is_hub {
                                config.node_radius * 1.3
                            } else {
                                config.node_radius
                            };

                            let stroke_class = if is_selected {
                                "stroke-purple-400"
                            } else {
                                "stroke-zinc-900"
                            };

                            let handle_click = move |_: ev::MouseEvent| {
                                handle_node_click(node_id_click.clone());
                            };

                            let handle_mouse_enter = move |_: ev::MouseEvent| {
                                hovered_node.set(Some((node_for_hover.clone(), x, y)));
                            };

                            let handle_mouse_leave = move |_: ev::MouseEvent| {
                                hovered_node.set(None);
                            };

                            view! {
                                <g
                                    class="cursor-pointer hover:opacity-90 transition-opacity"
                                    on:click=handle_click
                                    on:mouseenter=handle_mouse_enter
                                    on:mouseleave=handle_mouse_leave
                                >
                                    // Node circle
                                    <circle
                                        cx=x.to_string()
                                        cy=y.to_string()
                                        r=node_radius.to_string()
                                        fill=pn.node.color.clone()
                                        class=stroke_class
                                        style=format!("stroke-width: {}", if is_selected { 3 } else { 2 })
                                    />
                                    // Hub indicator ring
                                    {if is_hub {
                                        Some(view! {
                                            <circle
                                                cx=x.to_string()
                                                cy=y.to_string()
                                                r=(node_radius + 4.0).to_string()
                                                fill="none"
                                                stroke="#a855f7"
                                                stroke-width="2"
                                                stroke-dasharray="4 2"
                                                class="opacity-50"
                                            />
                                        })
                                    } else {
                                        None
                                    }}
                                    // Node label
                                    <text
                                        x=x.to_string()
                                        y=(y + node_radius + 14.0).to_string()
                                        text-anchor="middle"
                                        class="text-xs fill-zinc-300 font-medium select-none pointer-events-none"
                                    >
                                        {pn.node.name.clone()}
                                    </text>
                                </g>
                            }
                        }).collect_view()
                    }}
                </g>
            </svg>
                }
            }

            // Tooltip
            {move || hovered_node.get().map(|(node, x, y)| view! {
                <NodeTooltip node=node x=x y=y />
            })}

            // Empty state
            <Show when=move || !is_loading.get() && graph.get().map(|g| g.nodes.is_empty()).unwrap_or(true)>
                <div class="absolute inset-0 flex items-center justify-center">
                    <div class="text-center">
                        <div class="text-zinc-500 mb-4">"No relationships found"</div>
                        <p class="text-sm text-zinc-600">"Create entity relationships to see them visualized here"</p>
                    </div>
                </div>
            </Show>
        </div>
    }
}
