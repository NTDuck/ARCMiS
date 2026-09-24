//! Typed dependency-graph IR over `petgraph`. The planner consumes batches
//! from `topological_batches`; `strongly_connected_components` mirrors
//! DepWareTrans's cycle handling.

use anyhow::Context as _;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Node kinds of the migration graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GraphNodeKind {
    /// A compilation module (file-level unit).
    Module,
    /// A function or method.
    Function,
    /// A class or struct.
    Class,
    /// A test.
    Test,
    /// An external dependency.
    Dependency,
    /// A datastore (schema, table, migration).
    Datastore,
    /// An entry point (main, handler, job).
    EntryPoint,
    /// A configuration surface.
    Configuration,
}

/// Edge kinds of the migration graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GraphEdgeKind {
    /// Direct call.
    Calls,
    /// Import or use.
    Imports,
    /// Data read.
    Reads,
    /// Data write.
    Writes,
    /// Dynamic dispatch surface.
    Dispatches,
    /// Trait or interface implementation.
    Implements,
    /// Test relationship.
    Tests,
}

/// One graph node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphNode {
    /// Stable id (e.g. `module:src/lib.rs`).
    pub id: String,
    /// Node kind.
    pub kind: GraphNodeKind,
    /// Human label.
    pub label: String,
    /// Source file, when the node maps to one.
    #[serde(default)]
    pub file: Option<String>,
    /// Source line, when known.
    #[serde(default)]
    pub line: Option<usize>,
    /// Free-form attributes.
    #[serde(default)]
    pub attrs: BTreeMap<String, String>,
}

/// One graph edge.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GraphEdge {
    /// Source node id.
    pub from: String,
    /// Target node id.
    pub to: String,
    /// Edge kind.
    pub kind: GraphEdgeKind,
    /// Free-form attributes.
    #[serde(default)]
    pub attrs: BTreeMap<String, String>,
}

/// The typed graph.
#[derive(Debug, Clone, Default)]
pub struct Graph {
    nodes: BTreeMap<String, GraphNode>,
    edges: Vec<GraphEdge>,
    adjacency: BTreeMap<String, BTreeMap<String, GraphEdgeKind>>,
}

impl Graph {
    /// Empty graph.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or replace a node.
    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// Add an edge. Missing endpoints are ignored: the planner may reference
    /// nodes the analyst has not produced.
    pub fn add_edge(&mut self, edge: GraphEdge) {
        if self.nodes.contains_key(&edge.from) && self.nodes.contains_key(&edge.to) {
            self.adjacency
                .entry(edge.from.clone())
                .or_default()
                .insert(edge.to.clone(), edge.kind);
        }
        self.edges.push(edge);
    }

    /// Iterate nodes of one kind.
    pub fn nodes_by_kind(&self, kind: GraphNodeKind) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values().filter(move |node| node.kind == kind)
    }

    /// All nodes.
    #[must_use]
    pub fn nodes(&self) -> impl Iterator<Item = &GraphNode> {
        self.nodes.values()
    }

    /// All edges.
    #[must_use]
    pub fn edges(&self) -> &[GraphEdge] {
        &self.edges
    }

    /// Node lookup by id.
    #[must_use]
    pub fn node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.get(id)
    }

    /// Strongly connected components (Tarjan, petgraph). Components with one
    /// member and no self-loop are trivial; callers filter cycles themselves.
    #[must_use]
    pub fn strongly_connected_components(&self) -> Vec<Vec<String>> {
        let digraph = self.petgraph_view();
        petgraph::algo::tarjan_scc(&digraph)
            .into_iter()
            .map(|component| {
                component
                    .into_iter()
                    .map(|index| digraph[index].clone())
                    .collect()
            })
            .collect()
    }

    /// Build a petgraph DiGraph from the adjacency map for Tarjan SCC.
    fn petgraph_view(&self) -> petgraph::graph::DiGraph<String, GraphEdgeKind> {
        let mut digraph = petgraph::graph::DiGraph::new();
        let mut indices = BTreeMap::new();
        for id in self.nodes.keys() {
            indices.insert(id.clone(), digraph.add_node(id.clone()));
        }
        for (from, targets) in &self.adjacency {
            for (to, kind) in targets {
                if let (Some(&from_index), Some(&to_index)) =
                    (indices.get(from), indices.get(to))
                {
                    digraph.add_edge(from_index, to_index, *kind);
                }
            }
        }
        digraph
    }

    /// Dependency-consistent batches: Kahn topological layers over module
    /// nodes only (other node kinds ride with their module). Each batch's
    /// members depend only on earlier batches.
    #[must_use]
    pub fn topological_batches(&self) -> Vec<Vec<String>> {
        let modules: Vec<&GraphNode> = self.nodes_by_kind(GraphNodeKind::Module).collect();
        if modules.is_empty() {
            return self.topological_batches_all();
        }
        // Build the subgraph over modules: collapse every edge to its
        // module-level endpoints.
        let module_ids: Vec<String> = modules.iter().map(|node| node.id.clone()).collect();
        let module_set: std::collections::BTreeSet<&String> = module_ids.iter().collect();
        let mut module_adjacency: BTreeMap<String, BTreeMap<String, GraphEdgeKind>> =
            BTreeMap::new();
        for id in &module_ids {
            module_adjacency.entry(id.clone()).or_default();
        }
        for edge in &self.edges {
            let from = self.module_of(&edge.from);
            let to = self.module_of(&edge.to);
            if let (Some(from), Some(to)) = (from, to) {
                if from == to {
                    continue;
                }
                if module_set.contains(&from) && module_set.contains(&to) {
                    module_adjacency
                        .entry(from)
                        .or_default()
                        .insert(to, edge.kind);
                }
            }
        }
        kahn_layers(&module_ids, &module_adjacency)
    }

    /// Topological batches over every node (used when no module nodes exist).
    fn topological_batches_all(&self) -> Vec<Vec<String>> {
        let all: Vec<String> = self.nodes.keys().cloned().collect();
        kahn_layers(&all, &self.adjacency)
    }

    /// The module node that owns `id`: the node itself when it is a module,
    /// otherwise the node id as-is (callers may name modules directly).
    fn module_of(&self, id: &str) -> Option<String> {
        match self.nodes.get(id) {
            Some(node) if node.kind == GraphNodeKind::Module => Some(node.id.clone()),
            Some(node) => node.file.clone().map(|file| format!("module:{file}")),
            None => None,
        }
    }

    /// Serialize the graph to `{dir}/graphs/{name}.graph.json`.
    pub fn write(&self, dir: &Path, name: &str) -> anyhow::Result<()> {
        let dir = dir.join("graphs");
        std::fs::create_dir_all(&dir)?;
        let payload = GraphFile {
            nodes: self.nodes.values().cloned().collect(),
            edges: self.edges.clone(),
        };
        let text = serde_json::to_string_pretty(&payload)?;
        std::fs::write(dir.join(format!("{name}.graph.json")), text)
            .with_context(|| format!("write {name}.graph.json"))
    }

    /// Reconstruct a graph from `{dir}/graphs/{name}.graph.json`.
    pub fn read(dir: &Path, name: &str) -> anyhow::Result<Self> {
        let path = dir.join("graphs").join(format!("{name}.graph.json"));
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        let file: GraphFile = serde_json::from_str(&text)?;
        let mut graph = Self::new();
        for node in file.nodes {
            graph.add_node(node);
        }
        for edge in file.edges {
            graph.add_edge(edge);
        }
        Ok(graph)
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct GraphFile {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

/// Kahn layers over an adjacency map: repeatedly emit nodes with no
/// unemitted predecessors. A cycle emits its members in one terminal batch
/// so the caller still gets a total order.
fn kahn_layers(
    all: &[String],
    adjacency: &BTreeMap<String, BTreeMap<String, GraphEdgeKind>>,
) -> Vec<Vec<String>> {
    let mut emitted: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut layers = Vec::new();
    while emitted.len() < all.len() {
        let layer: Vec<String> = all
            .iter()
            .filter(|id| !emitted.contains(*id))
            .filter(|id| {
                adjacency
                    .iter()
                    .filter(|(from, _)| !emitted.contains(*from))
                    .all(|(from, targets)| {
                        !targets.contains_key(*id) || from.as_str() == id.as_str()
                    })
            })
            .cloned()
            .collect();
        if layer.is_empty() {
            let rest: Vec<String> = all
                .iter()
                .filter(|id| !emitted.contains(*id))
                .cloned()
                .collect();
            layers.push(rest);
            break;
        }
        for id in &layer {
            emitted.insert(id.clone());
        }
        layers.push(layer);
    }
    layers
}
