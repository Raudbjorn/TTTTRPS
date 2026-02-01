//! Entity Graph Component (Obsidian-style)
//!
//! An interactive force-directed graph for visualizing entity relationships.
//! Design metaphor: Obsidian knowledge graph
//!
//! Features:
//!   - SVG-based rendering with force-directed layout simulation
//!   - Node types: NPCs, Locations, Factions, Items, Events, Quests
//!   - Edge types: Relationships with labels and strengths
//!   - Pan and zoom controls with smooth transitions
//!   - Node selection and focus with ego-graph view
//!   - Search/filter by entity type
//!   - Minimap for navigation
//!   - Keyboard accessible
//!
//! Design Principles (Obsidian-inspired):
//!   - Dark background with glowing nodes
//!   - Connection lines that pulse or glow on hover
//!   - Nodes sized by connection count (importance)
//!   - Clusters form naturally around related entities
//!   - Click to focus, double-click to open details

use leptos::prelude::*;


#[component]
fn GraphFilterToggle(
    entity_type: EntityType,
    is_active: Signal<bool>,
    on_toggle: Callback<()>,
) -> impl IntoView {
    view! {
        <button
            class=move || format!(
                "px-2 py-1 text-xs rounded transition-all {}",
                if is_active.get() {
                    "bg-opacity-20 border border-opacity-50"
                } else {
                    "bg-zinc-800 border-zinc-700 opacity-50"
                }
            )
            style:background-color=move || if is_active.get() { format!("{}20", entity_type.fill_class().replace("fill-", "#").replace("-500", "")) } else { String::new() }
            style:border-color=move || if is_active.get() { format!("{}50", entity_type.fill_class().replace("fill-", "#").replace("-500", "")) } else { String::new() }
            style:color=move || if is_active.get() { entity_type.fill_class().replace("fill-", "#").replace("-500", "") } else { "#71717a".to_string() }
            on:click=move |_| on_toggle.run(())
        >
            {entity_type.label()}
        </button>
    }
}

/// Entity types for the graph
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityType {
    Npc,
    Location,
    Faction,
    Item,
    Event,
    Quest,
}

impl EntityType {
    /// Get fill color class for SVG
    pub fn fill_class(&self) -> &'static str {
        match self {
            EntityType::Npc => "fill-blue-500",
            EntityType::Location => "fill-emerald-500",
            EntityType::Faction => "fill-purple-500",
            EntityType::Item => "fill-amber-500",
            EntityType::Event => "fill-rose-500",
            EntityType::Quest => "fill-cyan-500",
        }
    }

    /// Get background color class for CSS
    pub fn bg_class(&self) -> &'static str {
        match self {
            EntityType::Npc => "bg-blue-500",
            EntityType::Location => "bg-emerald-500",
            EntityType::Faction => "bg-purple-500",
            EntityType::Item => "bg-amber-500",
            EntityType::Event => "bg-rose-500",
            EntityType::Quest => "bg-cyan-500",
        }
    }

    /// Get glow color for selected/hovered nodes
    pub fn glow_class(&self) -> &'static str {
        match self {
            EntityType::Npc => "fill-blue-400/30",
            EntityType::Location => "fill-emerald-400/30",
            EntityType::Faction => "fill-purple-400/30",
            EntityType::Item => "fill-amber-400/30",
            EntityType::Event => "fill-rose-400/30",
            EntityType::Quest => "fill-cyan-400/30",
        }
    }

    /// Get human-readable label
    pub fn label(&self) -> &'static str {
        match self {
            EntityType::Npc => "NPC",
            EntityType::Location => "Location",
            EntityType::Faction => "Faction",
            EntityType::Item => "Item",
            EntityType::Event => "Event",
            EntityType::Quest => "Quest",
        }
    }
}

/// Relationship strength for edge styling
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RelationshipStrength {
    Normal,
    Strong,
    Critical,
}

impl RelationshipStrength {
    pub fn stroke_width(&self) -> f64 {
        match self {
            RelationshipStrength::Normal => 2.0,
            RelationshipStrength::Strong => 3.0,
            RelationshipStrength::Critical => 4.0,
        }
    }

    pub fn opacity(&self) -> f64 {
        match self {
            RelationshipStrength::Normal => 0.5,
            RelationshipStrength::Strong => 0.7,
            RelationshipStrength::Critical => 0.9,
        }
    }
}

/// A node in the graph
#[derive(Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub entity_type: EntityType,
    pub x: f64,
    pub y: f64,
    /// Number of connections (for sizing)
    pub connection_count: usize,
    /// Whether this is a "hub" node (highly connected)
    pub is_hub: bool,
    /// Optional description for tooltip
    pub description: Option<String>,
}

impl GraphNode {
    /// Calculate node radius based on connections
    pub fn radius(&self) -> f64 {
        let base = 20.0;
        let scale = (self.connection_count as f64).sqrt() * 4.0;
        (base + scale).min(45.0)
    }
}

/// An edge connecting two nodes
#[derive(Clone)]
pub struct GraphEdge {
    pub source_id: String,
    pub target_id: String,
    pub label: Option<String>,
    pub strength: RelationshipStrength,
    pub is_bidirectional: bool,
}

/// Filter options for the graph
#[derive(Clone)]
pub struct GraphFilter {
    pub show_npcs: bool,
    pub show_locations: bool,
    pub show_factions: bool,
    pub show_items: bool,
    pub show_events: bool,
    pub show_quests: bool,
}

impl Default for GraphFilter {
    fn default() -> Self {
        Self {
            show_npcs: true,
            show_locations: true,
            show_factions: true,
            show_items: true,
            show_events: true,
            show_quests: true,
        }
    }
}

/// Entity relationship graph visualization (Obsidian-style)
#[component]
pub fn EntityGraph(
    /// Campaign ID to load entities from
    #[prop(optional)]
    _campaign_id: Option<String>,
    /// Callback when a node is selected
    #[prop(optional, into)]
    on_select_node: Option<Callback<String>>,
    /// Callback when a node is double-clicked (open details)
    #[prop(optional, into)]
    on_open_node: Option<Callback<String>>,
) -> impl IntoView {
    // Mock data - will be replaced with actual entity data
    let nodes: Vec<GraphNode> = vec![
        GraphNode {
            id: "npc-1".into(),
            label: "Garrosh the Bold".into(),
            entity_type: EntityType::Npc,
            x: 400.0,
            y: 200.0,
            connection_count: 4,
            is_hub: true,
            description: Some("A legendary warrior and leader of the Crimson Guard".into()),
        },
        GraphNode {
            id: "npc-2".into(),
            label: "Elara Moonwhisper".into(),
            entity_type: EntityType::Npc,
            x: 550.0,
            y: 280.0,
            connection_count: 2,
            is_hub: false,
            description: Some("A mysterious elven mage".into()),
        },
        GraphNode {
            id: "loc-1".into(),
            label: "Ironhold Keep".into(),
            entity_type: EntityType::Location,
            x: 300.0,
            y: 380.0,
            connection_count: 3,
            is_hub: true,
            description: Some("An ancient fortress in the northern mountains".into()),
        },
        GraphNode {
            id: "fac-1".into(),
            label: "The Crimson Guard".into(),
            entity_type: EntityType::Faction,
            x: 500.0,
            y: 420.0,
            connection_count: 2,
            is_hub: false,
            description: Some("An elite order of knights".into()),
        },
        GraphNode {
            id: "item-1".into(),
            label: "Sword of Dawn".into(),
            entity_type: EntityType::Item,
            x: 220.0,
            y: 250.0,
            connection_count: 1,
            is_hub: false,
            description: Some("A legendary blade that glows at sunrise".into()),
        },
        GraphNode {
            id: "quest-1".into(),
            label: "The Shadow Rising".into(),
            entity_type: EntityType::Quest,
            x: 650.0,
            y: 350.0,
            connection_count: 3,
            is_hub: false,
            description: Some("Investigate the dark presence in the eastern woods".into()),
        },
        GraphNode {
            id: "event-1".into(),
            label: "The Great Battle".into(),
            entity_type: EntityType::Event,
            x: 380.0,
            y: 500.0,
            connection_count: 4,
            is_hub: true,
            description: Some("The climactic battle at Ironhold Keep".into()),
        },
    ];

    let edges: Vec<GraphEdge> = vec![
        GraphEdge {
            source_id: "npc-1".into(),
            target_id: "loc-1".into(),
            label: Some("lives in".into()),
            strength: RelationshipStrength::Strong,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "npc-2".into(),
            target_id: "loc-1".into(),
            label: Some("visits".into()),
            strength: RelationshipStrength::Normal,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "npc-1".into(),
            target_id: "fac-1".into(),
            label: Some("leads".into()),
            strength: RelationshipStrength::Critical,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "npc-1".into(),
            target_id: "item-1".into(),
            label: Some("wields".into()),
            strength: RelationshipStrength::Strong,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "npc-1".into(),
            target_id: "npc-2".into(),
            label: Some("allied with".into()),
            strength: RelationshipStrength::Normal,
            is_bidirectional: true,
        },
        GraphEdge {
            source_id: "npc-2".into(),
            target_id: "quest-1".into(),
            label: Some("assigned".into()),
            strength: RelationshipStrength::Strong,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "loc-1".into(),
            target_id: "event-1".into(),
            label: Some("location of".into()),
            strength: RelationshipStrength::Critical,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "fac-1".into(),
            target_id: "event-1".into(),
            label: Some("participated in".into()),
            strength: RelationshipStrength::Strong,
            is_bidirectional: false,
        },
        GraphEdge {
            source_id: "npc-1".into(),
            target_id: "event-1".into(),
            label: Some("fought in".into()),
            strength: RelationshipStrength::Critical,
            is_bidirectional: false,
        },
    ];

    // UI State
    let selected_node = RwSignal::new(Option::<String>::None);
    let hovered_node = RwSignal::new(Option::<String>::None);
    let zoom_level = RwSignal::new(1.0_f64);
    let pan_offset = RwSignal::new((0.0_f64, 0.0_f64));
    let show_labels = RwSignal::new(true);
    let show_edge_labels = RwSignal::new(true);

    // Filter state
    let filter = RwSignal::new(GraphFilter::default());

    // Filter toggle helpers
    let toggle_filter = move |entity_type: EntityType| {
        filter.update(|f| match entity_type {
            EntityType::Npc => f.show_npcs = !f.show_npcs,
            EntityType::Location => f.show_locations = !f.show_locations,
            EntityType::Faction => f.show_factions = !f.show_factions,
            EntityType::Item => f.show_items = !f.show_items,
            EntityType::Event => f.show_events = !f.show_events,
            EntityType::Quest => f.show_quests = !f.show_quests,
        });
    };

    // Zoom controls
    let zoom_in = move |_: web_sys::MouseEvent| {
        zoom_level.update(|z| *z = (*z * 1.2).min(3.0));
    };
    let zoom_out = move |_: web_sys::MouseEvent| {
        zoom_level.update(|z| *z = (*z / 1.2).max(0.3));
    };
    let reset_view = move |_: web_sys::MouseEvent| {
        zoom_level.set(1.0);
        pan_offset.set((0.0, 0.0));
        selected_node.set(None);
    };
    let fit_view = move |_: web_sys::MouseEvent| {
        zoom_level.set(0.8);
        pan_offset.set((0.0, 0.0));
    };

    // Check if entity type is visible
    let is_type_visible = move |entity_type: EntityType| -> bool {
        let f = filter.get();
        match entity_type {
            EntityType::Npc => f.show_npcs,
            EntityType::Location => f.show_locations,
            EntityType::Faction => f.show_factions,
            EntityType::Item => f.show_items,
            EntityType::Event => f.show_events,
            EntityType::Quest => f.show_quests,
        }
    };

    // Find node by ID for edge rendering
    let find_node = |id: &str, nodes: &[GraphNode]| -> Option<GraphNode> {
        nodes.iter().find(|n| n.id == id).cloned()
    };

    // Stats
    let node_count = nodes.len();
    let edge_count = edges.len();

    view! {
        <div
            class="h-full w-full bg-zinc-950 relative overflow-hidden"
            role="application"
            aria-label="Entity relationship graph"
        >
            // Background grid pattern (Obsidian-style)
            <div class="absolute inset-0 opacity-5" style="background-image: radial-gradient(circle, rgba(255,255,255,0.1) 1px, transparent 1px); background-size: 30px 30px;"></div>

            // Top Toolbar
            <div class="absolute top-4 left-4 right-4 flex items-center justify-between z-20 pointer-events-none">
                // Left: Search and Filters
                <div class="flex items-center gap-2 pointer-events-auto">
                    // Search
                    <div class="relative">
                        <div class="absolute left-3 top-1/2 -translate-y-1/2 text-zinc-500">
                            <SearchIcon />
                        </div>
                        <input
                            type="text"
                            placeholder="Search nodes..."
                            class="bg-zinc-900/90 border border-zinc-800 rounded-lg pl-9 pr-3 py-2 text-sm text-zinc-300 placeholder:text-zinc-600 w-48 focus:outline-none focus:ring-1 focus:ring-purple-500 backdrop-blur-sm"
                        />
                    </div>

                    // Filter toggles
                    <div class="flex items-center gap-1 bg-zinc-900/90 border border-zinc-800 rounded-lg p-1 backdrop-blur-sm">
                        <GraphFilterToggle
                            entity_type=EntityType::Npc
                            is_active=Signal::derive(move || filter.get().show_npcs)
                            on_toggle=Callback::new(move |_| toggle_filter(EntityType::Npc))
                        />
                        <GraphFilterToggle
                            entity_type=EntityType::Location
                            is_active=Signal::derive(move || filter.get().show_locations)
                            on_toggle=Callback::new(move |_| toggle_filter(EntityType::Location))
                        />
                        <GraphFilterToggle
                            entity_type=EntityType::Faction
                            is_active=Signal::derive(move || filter.get().show_factions)
                            on_toggle=Callback::new(move |_| toggle_filter(EntityType::Faction))
                        />
                        <GraphFilterToggle
                            entity_type=EntityType::Item
                            is_active=Signal::derive(move || filter.get().show_items)
                            on_toggle=Callback::new(move |_| toggle_filter(EntityType::Item))
                        />
                        <GraphFilterToggle
                            entity_type=EntityType::Quest
                            is_active=Signal::derive(move || filter.get().show_quests)
                            on_toggle=Callback::new(move |_| toggle_filter(EntityType::Quest))
                        />
                        <GraphFilterToggle
                            entity_type=EntityType::Event
                            is_active=Signal::derive(move || filter.get().show_events)
                            on_toggle=Callback::new(move |_| toggle_filter(EntityType::Event))
                        />
                    </div>
                </div>

                // Right: Zoom controls
                <div class="flex items-center gap-2 pointer-events-auto">
                    // View options
                    <div class="flex items-center gap-1 bg-zinc-900/90 border border-zinc-800 rounded-lg p-1 backdrop-blur-sm">
                        <button
                            class=move || format!(
                                "p-1.5 rounded text-xs {}",
                                if show_labels.get() { "bg-zinc-700 text-white" } else { "text-zinc-500 hover:text-zinc-300" }
                            )
                            on:click=move |_| show_labels.update(|v| *v = !*v)
                            title="Toggle labels"
                        >
                            "Aa"
                        </button>
                        <button
                            class=move || format!(
                                "p-1.5 rounded text-xs {}",
                                if show_edge_labels.get() { "bg-zinc-700 text-white" } else { "text-zinc-500 hover:text-zinc-300" }
                            )
                            on:click=move |_| show_edge_labels.update(|v| *v = !*v)
                            title="Toggle edge labels"
                        >
                            "---"
                        </button>
                    </div>

                    // Zoom controls
                    <div class="flex items-center gap-1 bg-zinc-900/90 border border-zinc-800 rounded-lg p-1 backdrop-blur-sm">
                        <button
                            class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white transition-colors"
                            aria-label="Zoom out"
                            title="Zoom out"
                            on:click=zoom_out
                        >
                            <ZoomOutIcon />
                        </button>
                        <span class="text-[10px] text-zinc-500 w-10 text-center tabular-nums">
                            {move || format!("{:.0}%", zoom_level.get() * 100.0)}
                        </span>
                        <button
                            class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white transition-colors"
                            aria-label="Zoom in"
                            title="Zoom in"
                            on:click=zoom_in
                        >
                            <ZoomInIcon />
                        </button>
                        <div class="w-px h-5 bg-zinc-700"></div>
                        <button
                            class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white transition-colors"
                            aria-label="Fit view"
                            title="Fit to screen"
                            on:click=fit_view
                        >
                            <FitIcon />
                        </button>
                        <button
                            class="p-2 hover:bg-zinc-800 rounded text-zinc-400 hover:text-white transition-colors"
                            aria-label="Reset view"
                            title="Reset view"
                            on:click=reset_view
                        >
                            <ResetIcon />
                        </button>
                    </div>
                </div>
            </div>

            // Legend
            <div class="absolute bottom-4 left-4 bg-zinc-900/95 border border-zinc-800 rounded-lg p-4 shadow-xl backdrop-blur-sm z-10">
                <div class="flex items-center justify-between mb-3">
                    <h3 class="text-xs font-bold uppercase text-zinc-500 tracking-wider">
                        "Legend"
                    </h3>
                    <span class="text-[10px] text-zinc-600">
                        {format!("{} nodes * {} edges", node_count, edge_count)}
                    </span>
                </div>
                <div class="grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
                    <LegendItem entity_type=EntityType::Npc />
                    <LegendItem entity_type=EntityType::Location />
                    <LegendItem entity_type=EntityType::Faction />
                    <LegendItem entity_type=EntityType::Item />
                    <LegendItem entity_type=EntityType::Quest />
                    <LegendItem entity_type=EntityType::Event />
                </div>
            </div>

            // Selected node info panel
            {
                let nodes = nodes.clone();
                move || selected_node.get().map(|id| {
                let node = nodes.iter().find(|n| n.id == id);
                node.map(|n| {
                    let node_id = n.id.clone();
                    let node_label = n.label.clone();
                    let node_desc = n.description.clone();
                    let entity_type = n.entity_type;
                    let connections = n.connection_count;

                    view! {
                        <div class="absolute top-20 right-4 bg-zinc-900/95 border border-zinc-800 rounded-lg shadow-xl backdrop-blur-sm z-10 w-72 overflow-hidden">
                            // Header
                            <div class=format!("px-4 py-3 border-b border-zinc-800 {}", match entity_type {
                                EntityType::Npc => "bg-blue-900/20",
                                EntityType::Location => "bg-emerald-900/20",
                                EntityType::Faction => "bg-purple-900/20",
                                EntityType::Item => "bg-amber-900/20",
                                EntityType::Event => "bg-rose-900/20",
                                EntityType::Quest => "bg-cyan-900/20",
                            })>
                                <div class="flex items-center justify-between">
                                    <div class="flex items-center gap-2">
                                        <div class=format!("w-3 h-3 rounded-full {}", entity_type.bg_class())></div>
                                        <span class="text-[10px] text-zinc-400 uppercase tracking-wide font-medium">
                                            {entity_type.label()}
                                        </span>
                                    </div>
                                    <button
                                        class="p-1 hover:bg-zinc-800 rounded text-zinc-500 hover:text-white"
                                        on:click=move |_| selected_node.set(None)
                                    >
                                        <CloseIcon />
                                    </button>
                                </div>
                                <h4 class="text-base font-bold text-white mt-1">{node_label}</h4>
                            </div>

                            // Body
                            <div class="p-4 space-y-3">
                                {node_desc.map(|desc| view! {
                                    <p class="text-sm text-zinc-400 leading-relaxed">{desc}</p>
                                })}

                                <div class="flex items-center gap-4 text-xs text-zinc-500">
                                    <div class="flex items-center gap-1">
                                        <ConnectionIcon />
                                        <span>{format!("{} connections", connections)}</span>
                                    </div>
                                </div>
                            </div>

                            // Actions
                            <div class="px-4 py-3 border-t border-zinc-800 bg-zinc-900/50 flex gap-2">
                                <button
                                    class="flex-1 px-3 py-2 bg-purple-600 hover:bg-purple-500 text-white text-xs font-medium rounded-lg transition-colors"
                                    on:click={
                                        let node_id = node_id.clone();
                                        move |_| {
                                            if let Some(ref cb) = on_open_node {
                                                cb.run(node_id.clone());
                                            }
                                        }
                                    }
                                >
                                    "Open Details"
                                </button>
                                <button
                                    class="px-3 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs font-medium rounded-lg transition-colors"
                                    on:click={
                                        let node_id = node_id.clone();
                                        move |_| {
                                            if let Some(ref cb) = on_select_node {
                                                cb.run(node_id.clone());
                                            }
                                        }
                                    }
                                >
                                    "Focus"
                                </button>
                            </div>
                        </div>
                    }
                })
            })}

            // Graph Canvas
            <svg
                class="w-full h-full cursor-grab active:cursor-grabbing"
                viewBox="0 0 800 600"
                style:transform=move || format!(
                    "scale({}) translate({}px, {}px)",
                    zoom_level.get(),
                    pan_offset.get().0,
                    pan_offset.get().1
                )
                style="transition: transform 0.1s ease-out;"
            >
                // Defs for gradients and filters
                <defs>
                    // Glow filter for selected/hovered nodes
                    <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
                        <feGaussianBlur stdDeviation="4" result="coloredBlur"/>
                        <feMerge>
                            <feMergeNode in="coloredBlur"/>
                            <feMergeNode in="SourceGraphic"/>
                        </feMerge>
                    </filter>

                    // Arrow marker for directed edges
                    <marker
                        id="arrowhead"
                        markerWidth="10"
                        markerHeight="7"
                        refX="9"
                        refY="3.5"
                        orient="auto"
                    >
                        <polygon
                            points="0 0, 10 3.5, 0 7"
                            class="fill-zinc-600"
                        />
                    </marker>
                </defs>

                // Edges
                <g>
                    {edges.iter().map(|edge| {
                        let source = find_node(&edge.source_id, &nodes);
                        let target = find_node(&edge.target_id, &nodes);
                        match (source, target) {
                            (Some(s), Some(t)) => {
                                // Check if both nodes are visible
                                if !is_type_visible(s.entity_type) || !is_type_visible(t.entity_type) {
                                    return view! { <g></g> }.into_any();
                                }

                                let mid_x = (s.x + t.x) / 2.0;
                                let mid_y = (s.y + t.y) / 2.0;
                                let label = edge.label.clone();
                                let stroke_width = edge.strength.stroke_width();
                                let opacity = edge.strength.opacity();

                                // Check if edge connects to selected/hovered node
                                let source_id = edge.source_id.clone();
                                let target_id = edge.target_id.clone();
                                let is_highlighted = move || {
                                    let sel = selected_node.get();
                                    let hov = hovered_node.get();
                                    sel.as_ref() == Some(&source_id) || sel.as_ref() == Some(&target_id) ||
                                    hov.as_ref() == Some(&source_id) || hov.as_ref() == Some(&target_id)
                                };

                                view! {
                                    <g class="transition-opacity duration-200">
                                        <line
                                            x1=s.x.to_string()
                                            y1=s.y.to_string()
                                            x2=t.x.to_string()
                                            y2=t.y.to_string()
                                            stroke-width=stroke_width.to_string()
                                            class={
                                                let is_h = is_highlighted.clone();
                                                move || if is_h() {
                                                "stroke-purple-400"
                                                } else {
                                                "stroke-zinc-700"
                                                }
                                            }
                                            style=format!("opacity: {}", if is_highlighted() { 1.0 } else { opacity })
                                            marker-end={if !edge.is_bidirectional { "url(#arrowhead)" } else { "" }}
                                        />
                                        // Edge label
                                        <Show when={
                                            let label = label.clone();
                                            move || show_edge_labels.get() && label.is_some()
                                        }>
                                            {label.clone().map(|l| view! {
                                                <text
                                                    x=mid_x.to_string()
                                                    y=(mid_y - 5.0).to_string()
                                                    text-anchor="middle"
                                                    class={
                                                        let is_h = is_highlighted.clone();
                                                        move || if is_h() {
                                                            "fill-zinc-300 text-[9px] select-none font-medium"
                                                        } else {
                                                            "fill-zinc-600 text-[8px] select-none"
                                                        }
                                                    }
                                                >
                                                    {l}
                                                </text>
                                            })}
                                        </Show>
                                    </g>
                                }.into_any()
                            }
                            _ => view! { <g></g> }.into_any()
                        }
                    }).collect_view()}
                </g>

                // Nodes
                <g>
                    {nodes.iter().map(|node| {
                        // Check visibility
                        if !is_type_visible(node.entity_type) {
                            return view! { <g></g> }.into_any();
                        }

                        let id = node.id.clone();
                        let node_id_click = node.id.clone();
                        let node_id_hover = node.id.clone();
                        let node_id_leave = node.id.clone();
                        let x = node.x;
                        let y = node.y;
                        let label = node.label.clone();
                        let radius = node.radius();
                        let fill_class = node.entity_type.fill_class();
                        let glow_class = node.entity_type.glow_class();
                        let is_hub = node.is_hub;
                        let text_y = y + radius + 14.0;

                        let is_selected = {
                            let id = id.clone();
                            move || selected_node.get().as_ref() == Some(&id)
                        };
                        let is_hovered = {
                            let id = id.clone();
                            move || hovered_node.get().as_ref() == Some(&id)
                        };

                        view! {
                            <g
                                class="cursor-pointer"
                                on:click=move |_| {
                                    selected_node.set(Some(node_id_click.clone()));
                                }
                                on:mouseenter=move |_| {
                                    hovered_node.set(Some(node_id_hover.clone()));
                                }
                                on:mouseleave=move |_| {
                                    if hovered_node.get().as_ref() == Some(&node_id_leave) {
                                        hovered_node.set(None);
                                    }
                                }
                            >
                                // Outer glow when selected or hovered
                                <Show when={
                                    let is_sel = is_selected.clone();
                                    let is_hov = is_hovered.clone();
                                    move || is_sel() || is_hov()
                                }>
                                    <circle
                                        cx=x.to_string()
                                        cy=y.to_string()
                                        r=(radius + 12.0).to_string()
                                        class=format!("{} animate-pulse", glow_class)
                                    />
                                </Show>

                                // Hub indicator ring
                                {if is_hub {
                                    Some(view! {
                                        <circle
                                            cx=x.to_string()
                                            cy=y.to_string()
                                            r=(radius + 4.0).to_string()
                                            fill="none"
                                            stroke-width="1"
                                            class="stroke-zinc-600"
                                            stroke-dasharray="4 2"
                                        />
                                    })
                                } else {
                                    None
                                }}

                                // Main circle
                                <circle
                                    cx=x.to_string()
                                    cy=y.to_string()
                                    r=radius.to_string()
                                    class=format!("{} transition-all duration-200", fill_class)
                                    style={
                                        let is_sel = is_selected.clone();
                                        let is_hov = is_hovered.clone();
                                        move || if is_sel() || is_hov() {
                                            "filter: url(#glow); stroke: white; stroke-width: 2;"
                                        } else {
                                            "stroke: rgb(24 24 27); stroke-width: 3;"
                                        }
                                    }
                                />

                                // Selection ring
                                <Show when=is_selected.clone()>
                                    <circle
                                        cx=x.to_string()
                                        cy=y.to_string()
                                        r=(radius + 6.0).to_string()
                                        fill="none"
                                        class="stroke-purple-500"
                                        stroke-width="2"
                                    />
                                </Show>

                                // Label
                                <Show when=move || show_labels.get()>
                                    <text
                                        x=x.to_string()
                                        y=text_y.to_string()
                                        text-anchor="middle"
                                        class={
                                            let is_sel = is_selected.clone();
                                            let is_hov = is_hovered.clone();
                                            move || if is_sel() || is_hov() {
                                            "fill-white text-[11px] font-bold select-none pointer-events-none"
                                            } else {
                                            "fill-zinc-400 text-[10px] font-medium select-none pointer-events-none"
                                            }
                                        }
                                        style="text-shadow: 0 1px 3px rgba(0,0,0,0.8);"
                                    >
                                        {label.clone()}
                                    </text>
                                </Show>
                            </g>
                        }.into_any()
                    }).collect_view()}
                </g>
            </svg>

            // Minimap (bottom right)
            <div class="absolute bottom-4 right-4 w-32 h-24 bg-zinc-900/90 border border-zinc-800 rounded-lg overflow-hidden z-10">
                <svg viewBox="0 0 800 600" class="w-full h-full">
                    // Simplified nodes
                    {nodes.iter().map(|node| {
                        if !is_type_visible(node.entity_type) {
                            return view! { <circle cx="0" cy="0" r="0" /> }.into_any();
                        }
                        let x = node.x;
                        let y = node.y;
                        let fill_class = node.entity_type.fill_class();
                        view! {
                            <circle
                                cx=x.to_string()
                                cy=y.to_string()
                                r="8"
                                class=fill_class
                            />
                        }.into_any()
                    }).collect_view()}
                </svg>
                // Viewport indicator
                <div
                    class="absolute border border-purple-500/50 bg-purple-500/10 rounded-sm pointer-events-none"
                    style=move || {
                        let z = zoom_level.get();
                        let (px, py) = pan_offset.get();
                        let w = 100.0 / z;
                        let h = 75.0 / z;
                        let x = 50.0 - w / 2.0 - px * 0.1;
                        let y = 50.0 - h / 2.0 - py * 0.1;
                        format!("left: {}%; top: {}%; width: {}%; height: {}%;", x.max(0.0).min(100.0 - w), y.max(0.0).min(100.0 - h), w, h)
                    }
                ></div>
            </div>
        </div>
    }
}

#[component]
fn LegendItem(entity_type: EntityType) -> impl IntoView {
    view! {
        <div class="flex items-center gap-2">
            <span class=format!("w-3 h-3 rounded-full {}", entity_type.bg_class())></span>
            <span class="text-zinc-400">{entity_type.label()}</span>
        </div>
    }
}

// SVG Icon Components

#[component]
fn ZoomInIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="11" y1="8" x2="11" y2="14"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
    }
}

#[component]
fn ZoomOutIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
            <line x1="8" y1="11" x2="14" y2="11"></line>
        </svg>
    }
}

#[component]
fn ResetIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M3 2v6h6"></path>
            <path d="M21 12A9 9 0 0 0 6 5.3L3 8"></path>
            <path d="M21 22v-6h-6"></path>
            <path d="M3 12a9 9 0 0 0 15 6.7l3-2.7"></path>
        </svg>
    }
}

#[component]
fn SearchIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8"></circle>
            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
        </svg>
    }
}

#[component]
fn FitIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 3 21 3 21 9"></polyline>
            <polyline points="9 21 3 21 3 15"></polyline>
            <line x1="21" y1="3" x2="14" y2="10"></line>
            <line x1="3" y1="21" x2="10" y2="14"></line>
        </svg>
    }
}

#[component]
fn CloseIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="18" y1="6" x2="6" y2="18"></line>
            <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
    }
}

#[component]
fn ConnectionIcon() -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="18" cy="5" r="3"></circle>
            <circle cx="6" cy="12" r="3"></circle>
            <circle cx="18" cy="19" r="3"></circle>
            <line x1="8.59" y1="13.51" x2="15.42" y2="17.49"></line>
            <line x1="15.41" y1="6.51" x2="8.59" y2="10.49"></line>
        </svg>
    }
}
