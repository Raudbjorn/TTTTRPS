//! ASCII relationship graph widget for ratatui.
//!
//! Renders NPC/location/faction graphs using Unicode box-drawing characters.
//! Supports tree (hierarchical) and flat (adjacency list) layouts with
//! color-coded node types, scroll offset, and selection highlight.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Widget,
};

use crate::tui::theme;

// ── Data types ──────────────────────────────────────────────────────────────

/// The kind of entity a graph node represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Npc,
    Location,
    Faction,
    Item,
    Custom,
}

impl NodeType {
    /// Return the theme color for this node type.
    fn color(self) -> ratatui::style::Color {
        match self {
            NodeType::Npc => theme::PRIMARY_LIGHT,
            NodeType::Location => theme::WARNING,
            NodeType::Faction => theme::NPC, // lavender
            NodeType::Item => theme::INFO,
            NodeType::Custom => theme::TEXT,
        }
    }

    /// Short prefix tag rendered before the label.
    fn tag(self) -> &'static str {
        match self {
            NodeType::Npc => "NPC",
            NodeType::Location => "LOC",
            NodeType::Faction => "FAC",
            NodeType::Item => "ITM",
            NodeType::Custom => "---",
        }
    }
}

/// A node in the relationship graph.
#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub node_type: NodeType,
    /// IDs of child nodes (used in Tree layout).
    pub children: Vec<String>,
}

/// A directed edge between two nodes.
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub label: String,
}

/// Layout strategy for the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GraphLayout {
    /// Parent-child hierarchy using tree branches.
    #[default]
    Tree,
    /// Flat adjacency list — each node followed by its edges.
    Flat,
}

// ── Widget ──────────────────────────────────────────────────────────────────

/// An ASCII relationship graph rendered with Unicode tree characters.
///
/// # Example
///
/// ```ignore
/// let graph = AsciiGraph::new(&nodes, &edges)
///     .layout(GraphLayout::Tree)
///     .selected(Some("npc_1"))
///     .scroll(0);
/// frame.render_widget(graph, area);
/// ```
pub struct AsciiGraph<'a> {
    nodes: &'a [GraphNode],
    edges: &'a [GraphEdge],
    selected: Option<&'a str>,
    scroll_offset: usize,
    layout_mode: GraphLayout,
}

impl<'a> AsciiGraph<'a> {
    pub fn new(nodes: &'a [GraphNode], edges: &'a [GraphEdge]) -> Self {
        Self {
            nodes,
            edges,
            selected: None,
            scroll_offset: 0,
            layout_mode: GraphLayout::default(),
        }
    }

    pub fn layout(mut self, mode: GraphLayout) -> Self {
        self.layout_mode = mode;
        self
    }

    pub fn selected(mut self, id: Option<&'a str>) -> Self {
        self.selected = id;
        self
    }

    pub fn scroll(mut self, offset: usize) -> Self {
        self.scroll_offset = offset;
        self
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Find a node by id.
    fn find_node(&self, id: &str) -> Option<&'a GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Build the full set of rendered lines depending on layout mode.
    fn build_lines(&self) -> Vec<Line<'static>> {
        match self.layout_mode {
            GraphLayout::Tree => self.build_tree_lines(),
            GraphLayout::Flat => self.build_flat_lines(),
        }
    }

    // ── Tree layout ─────────────────────────────────────────────────────

    /// Identify root nodes: those that never appear as a child of another node.
    fn root_ids(&self) -> Vec<&'a str> {
        let child_ids: std::collections::HashSet<&str> = self
            .nodes
            .iter()
            .flat_map(|n| n.children.iter().map(String::as_str))
            .collect();

        self.nodes
            .iter()
            .filter(|n| !child_ids.contains(n.id.as_str()))
            .map(|n| n.id.as_str())
            .collect()
    }

    fn build_tree_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let roots = self.root_ids();

        if roots.is_empty() && !self.nodes.is_empty() {
            // No clear roots — fall back to flat listing so nothing is hidden.
            return self.build_flat_lines();
        }

        for (i, root_id) in roots.iter().enumerate() {
            let is_last_root = i == roots.len() - 1;
            self.render_tree_node(root_id, &mut lines, "", is_last_root, true);
        }

        lines
    }

    /// Recursively render a node and its children with tree glyphs.
    fn render_tree_node(
        &self,
        node_id: &str,
        lines: &mut Vec<Line<'static>>,
        prefix: &str,
        is_last: bool,
        is_root: bool,
    ) {
        let Some(node) = self.find_node(node_id) else {
            return;
        };

        let branch = if is_root {
            // Root level — no branch glyph.
            String::new()
        } else if is_last {
            format!("{prefix}└── ")
        } else {
            format!("{prefix}├── ")
        };

        let is_selected = self.selected == Some(node_id);
        lines.push(self.styled_node_line(&branch, node, is_selected));

        // Continuation prefix for content below this node (edges, children).
        let continuation = if is_root {
            String::new()
        } else if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };

        // Edges outgoing from this node (label annotations on the branch).
        for edge in self.edges.iter().filter(|e| e.source == node_id) {
            if !edge.label.is_empty() {
                let edge_line = Line::from(vec![
                    Span::raw(format!("{continuation}  ")),
                    Span::styled(
                        format!("─({})─▸ ", edge.label),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                    Span::styled(
                        edge.target.clone(),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ]);
                lines.push(edge_line);
            }
        }

        // Recurse into children.
        let child_count = node.children.len();
        for (ci, child_id) in node.children.iter().enumerate() {
            let child_is_last = ci == child_count - 1;
            self.render_tree_node(child_id, lines, &continuation, child_is_last, false);
        }
    }

    // ── Flat layout ─────────────────────────────────────────────────────

    fn build_flat_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        for node in self.nodes {
            let is_selected = self.selected == Some(node.id.as_str());
            lines.push(self.styled_node_line("", node, is_selected));

            // Outgoing edges from this node.
            let outgoing: Vec<&GraphEdge> =
                self.edges.iter().filter(|e| e.source == node.id).collect();

            for (ei, edge) in outgoing.iter().enumerate() {
                let is_last_edge = ei == outgoing.len() - 1;
                let glyph = if is_last_edge { "└─" } else { "├─" };
                let target_label = self
                    .find_node(&edge.target)
                    .map(|n| n.label.as_str())
                    .unwrap_or(edge.target.as_str());

                let mut spans = vec![
                    Span::raw(format!("  {glyph} ")),
                    Span::styled(
                        format!("({}) ", edge.label),
                        Style::default().fg(theme::TEXT_DIM),
                    ),
                    Span::styled(
                        target_label.to_string(),
                        Style::default().fg(theme::TEXT_MUTED),
                    ),
                ];

                // If the target node has a type, show a colored tag.
                if let Some(target_node) = self.find_node(&edge.target) {
                    spans.push(Span::styled(
                        format!(" [{}]", target_node.node_type.tag()),
                        Style::default().fg(target_node.node_type.color()),
                    ));
                }

                lines.push(Line::from(spans));
            }
        }

        lines
    }

    // ── Shared styling ──────────────────────────────────────────────────

    /// Build a styled `Line` for a single node.
    fn styled_node_line(
        &self,
        prefix: &str,
        node: &GraphNode,
        is_selected: bool,
    ) -> Line<'static> {
        let tag_style = Style::default().fg(node.node_type.color());
        let label_style = if is_selected {
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::TEXT)
        };

        let selection_indicator = if is_selected { "▸ " } else { "  " };

        Line::from(vec![
            Span::raw(prefix.to_string()),
            Span::styled(
                selection_indicator.to_string(),
                if is_selected {
                    Style::default().fg(theme::ACCENT)
                } else {
                    Style::default()
                },
            ),
            Span::styled(format!("[{}] ", node.node_type.tag()), tag_style),
            Span::styled(node.label.clone(), label_style),
        ])
    }
}

// ── Widget impl ─────────────────────────────────────────────────────────────

impl Widget for AsciiGraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let lines = self.build_lines();
        let visible_height = area.height as usize;

        // Apply scroll offset — clamp so we don't scroll past content.
        let max_offset = lines.len().saturating_sub(visible_height);
        let offset = self.scroll_offset.min(max_offset);

        for (i, line) in lines.iter().skip(offset).take(visible_height).enumerate() {
            let y = area.y + i as u16;
            let mut x = area.x;
            let max_x = area.x + area.width;

            for span in &line.spans {
                if x >= max_x {
                    break;
                }
                let available = (max_x - x) as usize;
                let text: String = span.content.chars().take(available).collect();
                let width = text.len() as u16;
                buf.set_string(x, y, &text, span.style);
                x += width;
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a Buffer of the given size and render the widget into it.
    fn render_to_string(widget: AsciiGraph<'_>, width: u16, height: u16) -> Vec<String> {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);

        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buf.cell((x, y)).map_or(' ', |c| {
                        c.symbol().chars().next().unwrap_or(' ')
                    }))
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    #[test]
    fn test_empty_graph() {
        let nodes: Vec<GraphNode> = vec![];
        let edges: Vec<GraphEdge> = vec![];
        let widget = AsciiGraph::new(&nodes, &edges);
        let output = render_to_string(widget, 40, 5);

        // All lines should be empty for an empty graph.
        for line in &output {
            assert!(line.trim().is_empty(), "expected empty line, got: {line:?}");
        }
    }

    #[test]
    fn test_single_node() {
        let nodes = vec![GraphNode {
            id: "npc_1".into(),
            label: "Gundren Rockseeker".into(),
            node_type: NodeType::Npc,
            children: vec![],
        }];
        let edges: Vec<GraphEdge> = vec![];
        let widget = AsciiGraph::new(&nodes, &edges);
        let output = render_to_string(widget, 60, 5);

        let joined = output.join("\n");
        assert!(
            joined.contains("Gundren Rockseeker"),
            "expected label in output: {joined}"
        );
        assert!(
            joined.contains("[NPC]"),
            "expected [NPC] tag in output: {joined}"
        );
    }

    #[test]
    fn test_parent_child_tree() {
        let nodes = vec![
            GraphNode {
                id: "loc_1".into(),
                label: "Phandalin".into(),
                node_type: NodeType::Location,
                children: vec!["npc_1".into(), "npc_2".into()],
            },
            GraphNode {
                id: "npc_1".into(),
                label: "Sister Garaele".into(),
                node_type: NodeType::Npc,
                children: vec![],
            },
            GraphNode {
                id: "npc_2".into(),
                label: "Toblen Stonehill".into(),
                node_type: NodeType::Npc,
                children: vec![],
            },
        ];
        let edges: Vec<GraphEdge> = vec![];
        let widget = AsciiGraph::new(&nodes, &edges).layout(GraphLayout::Tree);
        let output = render_to_string(widget, 60, 10);

        let joined = output.join("\n");
        // Root should appear.
        assert!(joined.contains("Phandalin"), "missing root: {joined}");
        // Children should appear with tree glyphs.
        assert!(
            joined.contains("├──") || joined.contains("└──"),
            "missing tree glyphs: {joined}"
        );
        assert!(joined.contains("Sister Garaele"), "missing child 1: {joined}");
        assert!(joined.contains("Toblen Stonehill"), "missing child 2: {joined}");
    }

    #[test]
    fn test_node_type_styling() {
        let nodes = vec![
            GraphNode {
                id: "n1".into(),
                label: "Test NPC".into(),
                node_type: NodeType::Npc,
                children: vec![],
            },
            GraphNode {
                id: "n2".into(),
                label: "Test Location".into(),
                node_type: NodeType::Location,
                children: vec![],
            },
            GraphNode {
                id: "n3".into(),
                label: "Test Faction".into(),
                node_type: NodeType::Faction,
                children: vec![],
            },
        ];
        let edges: Vec<GraphEdge> = vec![];

        // Verify each node type maps to the correct color.
        assert_eq!(NodeType::Npc.color(), theme::PRIMARY_LIGHT);
        assert_eq!(NodeType::Location.color(), theme::WARNING);
        assert_eq!(NodeType::Faction.color(), theme::NPC);
        assert_eq!(NodeType::Item.color(), theme::INFO);
        assert_eq!(NodeType::Custom.color(), theme::TEXT);

        // Verify tags render into the buffer.
        let widget = AsciiGraph::new(&nodes, &edges).layout(GraphLayout::Flat);
        let output = render_to_string(widget, 60, 10);
        let joined = output.join("\n");
        assert!(joined.contains("[NPC]"), "missing NPC tag: {joined}");
        assert!(joined.contains("[LOC]"), "missing LOC tag: {joined}");
        assert!(joined.contains("[FAC]"), "missing FAC tag: {joined}");
    }

    #[test]
    fn test_scroll_offset() {
        // Create enough nodes to exceed a small viewport.
        let nodes: Vec<GraphNode> = (0..20)
            .map(|i| GraphNode {
                id: format!("n{i}"),
                label: format!("Node {i}"),
                node_type: NodeType::Npc,
                children: vec![],
            })
            .collect();
        let edges: Vec<GraphEdge> = vec![];

        // Without scroll — first node visible.
        let widget = AsciiGraph::new(&nodes, &edges)
            .layout(GraphLayout::Flat)
            .scroll(0);
        let output_top = render_to_string(widget, 60, 5);
        let joined_top = output_top.join("\n");
        assert!(
            joined_top.contains("Node 0"),
            "expected Node 0 at scroll=0: {joined_top}"
        );
        assert!(
            !joined_top.contains("Node 10"),
            "Node 10 should not be visible at scroll=0: {joined_top}"
        );

        // With scroll=10 — Node 10 visible, Node 0 hidden.
        let widget = AsciiGraph::new(&nodes, &edges)
            .layout(GraphLayout::Flat)
            .scroll(10);
        let output_scrolled = render_to_string(widget, 60, 5);
        let joined_scrolled = output_scrolled.join("\n");
        assert!(
            joined_scrolled.contains("Node 10"),
            "expected Node 10 at scroll=10: {joined_scrolled}"
        );
        assert!(
            !joined_scrolled.contains("Node 0"),
            "Node 0 should be scrolled away: {joined_scrolled}"
        );
    }

    #[test]
    fn test_selected_node_highlight() {
        let nodes = vec![
            GraphNode {
                id: "a".into(),
                label: "Alpha".into(),
                node_type: NodeType::Npc,
                children: vec![],
            },
            GraphNode {
                id: "b".into(),
                label: "Beta".into(),
                node_type: NodeType::Location,
                children: vec![],
            },
        ];
        let edges: Vec<GraphEdge> = vec![];

        let widget = AsciiGraph::new(&nodes, &edges)
            .layout(GraphLayout::Flat)
            .selected(Some("a"));
        let lines = widget.build_lines();

        // The first line (selected) should contain the selection indicator.
        let first_line_text: String = lines[0].spans.iter().map(|s| s.content.to_string()).collect();
        assert!(
            first_line_text.contains('▸'),
            "selected node should have ▸ indicator: {first_line_text}"
        );

        // The selected label span should be ACCENT-colored.
        let label_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("Alpha"))
            .expect("label span missing");
        assert_eq!(
            label_span.style.fg,
            Some(theme::ACCENT),
            "selected label should be ACCENT"
        );
    }

    #[test]
    fn test_flat_layout_edges() {
        let nodes = vec![
            GraphNode {
                id: "npc_1".into(),
                label: "Gundren".into(),
                node_type: NodeType::Npc,
                children: vec![],
            },
            GraphNode {
                id: "npc_2".into(),
                label: "Sildar".into(),
                node_type: NodeType::Npc,
                children: vec![],
            },
        ];
        let edges = vec![GraphEdge {
            source: "npc_1".into(),
            target: "npc_2".into(),
            label: "ally".into(),
        }];

        let widget = AsciiGraph::new(&nodes, &edges).layout(GraphLayout::Flat);
        let output = render_to_string(widget, 60, 10);
        let joined = output.join("\n");
        assert!(joined.contains("ally"), "edge label should appear: {joined}");
        assert!(
            joined.contains("Sildar"),
            "edge target label should appear: {joined}"
        );
    }

    #[test]
    fn test_zero_area_does_not_panic() {
        let nodes = vec![GraphNode {
            id: "x".into(),
            label: "X".into(),
            node_type: NodeType::Custom,
            children: vec![],
        }];
        let edges: Vec<GraphEdge> = vec![];

        // Width=0 should be a no-op.
        let area = Rect::new(0, 0, 0, 0);
        let mut buf = Buffer::empty(area);
        let widget = AsciiGraph::new(&nodes, &edges);
        widget.render(area, &mut buf);
        // No panic = pass.
    }
}
