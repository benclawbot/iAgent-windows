// ---------------------------------------------------------------------------
// Scene Graph and Temporal Tracking (Feature #5)
// ---------------------------------------------------------------------------
// Builds a richer representation of the user's screen context over time.
// Instead of flat window titles, we track: what's active, what changed,
// and what the user is working on right now.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// The type of a scene node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    Window,
    Application,
    UIElement,
    Workspace,
}

impl std::fmt::Display for NodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeType::Window => write!(f, "window"),
            NodeType::Application => write!(f, "application"),
            NodeType::UIElement => write!(f, "ui_element"),
            NodeType::Workspace => write!(f, "workspace"),
        }
    }
}

/// A node in the scene graph — a window, app, or UI element.
#[derive(Debug, Clone)]
pub struct SceneNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub stability_score: f32,
    pub last_seen: DateTime<Utc>,
    pub attributes: HashMap<String, String>,
}

impl SceneNode {
    pub fn new(id: String, node_type: NodeType, label: String) -> Self {
        Self {
            id,
            node_type,
            label,
            stability_score: 1.0,
            last_seen: Utc::now(),
            attributes: HashMap::new(),
        }
    }

    /// Returns true if this node has been stable (seen consistently) over the given duration.
    pub fn is_stable(&self, window: Duration) -> bool {
        self.stability_score >= 0.7 && (Utc::now() - self.last_seen) <= window
    }
}

/// How two scene nodes relate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneRelation {
    Contains,
    Overlaps,
    FollowsFrom,
    OwnedBy,
    RelatedTo,
}

/// An edge between two scene nodes.
#[derive(Debug, Clone)]
pub struct SceneEdge {
    pub source: String,
    pub target: String,
    pub relation: SceneRelation,
    pub confidence: f32,
}

/// The full scene graph at a point in time.
#[derive(Debug, Clone)]
pub struct SceneGraph {
    pub nodes: Vec<SceneNode>,
    pub edges: Vec<SceneEdge>,
    pub scene_timestamp: DateTime<Utc>,
}

impl SceneGraph {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            scene_timestamp: Utc::now(),
        }
    }

    /// Find a node by its ID.
    pub fn get_node(&self, id: &str) -> Option<&SceneNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all nodes of a specific type.
    pub fn nodes_of_type(&self, node_type: NodeType) -> Vec<&SceneNode> {
        self.nodes.iter().filter(|n| n.node_type == node_type).collect()
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of the scene graph at a point in time, for temporal history.
#[derive(Debug, Clone)]
pub struct SceneSnapshot {
    pub graph: SceneGraph,
    pub recorded_at: DateTime<Utc>,
}

/// The difference between two scene graphs.
#[derive(Debug, Clone, Default)]
pub struct SceneDiff {
    /// Node IDs that appeared in curr but not in prev.
    pub added: Vec<String>,
    /// Node IDs that disappeared from prev to curr.
    pub removed: Vec<String>,
    /// (node_id, old_label, new_label) for nodes that changed label.
    pub changed: Vec<(String, String, String)>,
    /// Node IDs that appeared in both and are stable.
    pub stable: Vec<String>,
}

impl SceneDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// Compute the diff between two scene graphs.
pub fn compute_scene_diff(prev: &SceneGraph, curr: &SceneGraph) -> SceneDiff {
    let mut diff = SceneDiff::default();

    let prev_ids: std::collections::HashSet<_> = prev.nodes.iter().map(|n| n.id.clone()).collect();
    let curr_ids: std::collections::HashSet<_> = curr.nodes.iter().map(|n| n.id.clone()).collect();

    // Added: in curr but not in prev
    for id in &curr_ids {
        if !prev_ids.contains(id) {
            diff.added.push(id.clone());
        }
    }

    // Removed: in prev but not in curr
    for id in &prev_ids {
        if !curr_ids.contains(id) {
            diff.removed.push(id.clone());
        }
    }

    // Changed: present in both but with different labels
    for curr_node in &curr.nodes {
        if let Some(prev_node) = prev.nodes.iter().find(|n| n.id == curr_node.id) {
            if prev_node.label != curr_node.label {
                diff.changed.push((
                    curr_node.id.clone(),
                    prev_node.label.clone(),
                    curr_node.label.clone(),
                ));
            } else if curr_node.stability_score >= 0.7 {
                diff.stable.push(curr_node.id.clone());
            }
        }
    }

    diff
}

/// The user's current working context derived from window activity.
#[derive(Debug, Clone)]
pub struct WorkContext {
    /// Window titles that are currently active.
    pub active_windows: Vec<String>,
    /// How many minutes since the user last interacted.
    pub idle_minutes: u32,
    /// A summary of what the user appears to be working on.
    pub context_summary: String,
}

impl Default for WorkContext {
    fn default() -> Self {
        Self {
            active_windows: Vec::new(),
            idle_minutes: 0,
            context_summary: "No recent screen activity".to_string(),
        }
    }
}

/// Tracks per-window focus statistics over time.
#[derive(Debug, Clone)]
pub struct WindowStats {
    pub focus_count: usize,
    pub total_focus_seconds: u64,
    pub last_focus: Option<DateTime<Utc>>,
    pub avg_duration_secs: f64,
}

impl Default for WindowStats {
    fn default() -> Self {
        Self {
            focus_count: 0,
            total_focus_seconds: 0,
            last_focus: None,
            avg_duration_secs: 0.0,
        }
    }
}

/// Maintains temporal history and computes work context.
pub struct TemporalTracker {
    pub history: Vec<SceneSnapshot>,
    pub max_history: usize,
    pub window_stats: HashMap<String, WindowStats>,
}

impl TemporalTracker {
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
            window_stats: HashMap::new(),
        }
    }

    /// Add a new scene graph snapshot, trimming history to max_history.
    pub fn push_snapshot(&mut self, graph: SceneGraph) {
        let snapshot = SceneSnapshot {
            graph: graph.clone(),
            recorded_at: Utc::now(),
        };
        self.history.push(snapshot);
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }
        // Update window stats from the new graph
        self.update_window_stats(&graph);
    }

    fn update_window_stats(&mut self, graph: &SceneGraph) {
        let now = Utc::now();
        for node in &graph.nodes {
            if node.node_type != NodeType::Window {
                continue;
            }
            let entry = self
                .window_stats
                .entry(node.id.clone())
                .or_default();
            entry.focus_count += 1;
            entry.last_focus = Some(now);
        }
    }

    /// Get changes in the last `minutes` window.
    pub fn recent_changes(&self, minutes: i64) -> Option<SceneDiff> {
        let cutoff = Utc::now() - Duration::minutes(minutes);
        let recent: Vec<_> = self.history.iter().filter(|s| s.recorded_at >= cutoff).collect();

        if recent.len() < 2 {
            return None;
        }

        let first = &recent[0].graph;
        let last = &recent[recent.len() - 1].graph;
        Some(compute_scene_diff(first, last))
    }

    /// Get the most recent scene graph, if any.
    pub fn latest_graph(&self) -> Option<&SceneGraph> {
        self.history.last().map(|s| &s.graph)
    }

    /// Compute current work context from the tracker's own history.
    pub fn current_work_context(&self) -> WorkContext {
        let recent: Vec<_> = self.history.iter().rev().take(20).collect();

        if recent.is_empty() {
            return WorkContext::default();
        }

        // Get unique window titles from recent snapshots (last 5 minutes)
        let cutoff = Utc::now() - Duration::minutes(5);
        let recent_frames: Vec<_> = recent.iter().filter(|s| s.recorded_at >= cutoff).collect();

        let active_windows: Vec<String> = recent_frames
            .iter()
            .filter_map(|s| s.graph.nodes.iter().find(|n| n.node_type == NodeType::Window))
            .map(|n| n.label.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(5)
            .collect();

        // Calculate idle time from most recent snapshot
        let idle_minutes = recent_frames
            .last()
            .map(|s| (Utc::now() - s.recorded_at).num_minutes() as u32)
            .unwrap_or(0);

        // Build context summary from window titles and labels
        let context_summary = if !active_windows.is_empty() {
            let summary = active_windows.join(" | ");
            if summary.len() > 100 {
                format!("{}...", &summary[..100])
            } else {
                summary
            }
        } else {
            "No active windows detected".to_string()
        };

        WorkContext {
            active_windows,
            idle_minutes,
            context_summary,
        }
    }

    /// Get the diff for the last 5 minutes.
    pub fn last_5min_diff(&self) -> Option<SceneDiff> {
        self.recent_changes(5)
    }
}

impl Default for TemporalTracker {
    fn default() -> Self {
        Self::new(20)
    }
}

/// Build a scene graph from a list of recent recorded frames.
pub fn build_scene_graph_from_frames(frames: &[crate::ambient::RecordedFrame]) -> SceneGraph {
    let mut graph = SceneGraph::new();
    let now = Utc::now();

    // Deduplicate by window title
    let mut seen_titles: std::collections::HashSet<String> = std::collections::HashSet::new();

    for frame in frames.iter().rev().take(20) {
        let title = frame
            .window_title
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        if seen_titles.contains(&title) {
            // Update last_seen of existing node
            if let Some(node) = graph.nodes.iter_mut().find(|n| n.label == title) {
                node.last_seen = now;
                node.stability_score = (node.stability_score + 0.1).min(1.0);
            }
            continue;
        }
        seen_titles.insert(title.clone());

        let node = SceneNode::new(
            format!("window_{}", graph.nodes.len()),
            NodeType::Window,
            title.clone(),
        );
        graph.nodes.push(node);
    }

    graph.scene_timestamp = now;
    graph
}

/// Render scene context as a section for the system prompt.
/// `active_windows` is a list of window title strings seen recently.
/// `idle_minutes` is how long since the last user interaction.
/// `recent_changes` describes what changed in the last 5 minutes (added/removed/changed labels).
pub fn build_scene_context_section(
    _tracker: &TemporalTracker,
    active_windows: &[String],
    idle_minutes: u32,
    context_summary: &str,
    recent_changes: Option<&SceneDiff>,
) -> String {
    let mut lines = Vec::new();
    lines.push("## Current Scene Context".to_string());

    // Active windows
    if active_windows.is_empty() {
        lines.push("- Active windows: none detected".to_string());
    } else {
        lines.push("- Active windows:".to_string());
        for win in active_windows.iter().take(5) {
            lines.push(format!("  - {}", win));
        }
    }

    // Idle
    if idle_minutes > 0 {
        lines.push(format!("- Idle: {} minutes", idle_minutes));
    }

    // Context summary
    lines.push(format!("- Current work: {}", context_summary));

    // Changes in last 5 minutes
    if let Some(diff) = recent_changes {
        if !diff.added.is_empty() {
            lines.push(format!("- New windows: {}", diff.added.join(", ")));
        }
        if !diff.removed.is_empty() {
            lines.push(format!("- Closed windows: {}", diff.removed.join(", ")));
        }
        if !diff.changed.is_empty() {
            lines.push("- Changed windows:".to_string());
            for (id, old, new_val) in &diff.changed {
                lines.push(format!("  - {}: '{}' -> '{}'", id, old, new_val));
            }
        }
        if !diff.stable.is_empty() {
            lines.push(format!("- Stable windows: {}", diff.stable.join(", ")));
        }
    }

    let result = lines.join("\n");
    if result.lines().count() <= 2 {
        String::new()
    } else {
        result
    }
}