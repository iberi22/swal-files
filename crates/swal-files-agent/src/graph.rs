//! Xavier GraphRAG File Dependency Graph Builder.
//! Builds file relationship graphs (imports, references, hierarchy) for semantic indexing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Classification of the relationship edge between two files in the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Rust or ES module import / dependency.
    Import,
    /// Type or trait inheritance/implementation.
    TypeReference,
    /// Parent/Child filesystem directory hierarchy.
    ParentDirectory,
    /// Semantic or contextual similarity linkage.
    SemanticLink,
    /// Associated test file.
    TestFor,
}

/// Represents a single file or directory node in the knowledge graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub path: PathBuf,
    pub label: String,
    pub is_directory: bool,
    pub file_extension: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl GraphNode {
    pub fn new<P: AsRef<Path>>(path: P, is_directory: bool) -> Self {
        let p = path.as_ref().to_path_buf();
        let label = p
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());
        let ext = p.extension().map(|e| e.to_string_lossy().to_string());

        Self {
            id: p.to_string_lossy().to_string(),
            path: p,
            label,
            is_directory,
            file_extension: ext,
            metadata: HashMap::new(),
        }
    }
}

/// Represents a directed dependency edge from source node to target node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub kind: EdgeKind,
    pub weight: f32,
}

impl GraphEdge {
    pub fn new(source: impl Into<String>, target: impl Into<String>, kind: EdgeKind) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            kind,
            weight: 1.0,
        }
    }
}

/// Project-wide dependency graph ready for Xavier GraphRAG ingestion.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileDependencyGraph {
    pub project_root: PathBuf,
    pub nodes: HashMap<String, GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl FileDependencyGraph {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        Self {
            project_root: root.as_ref().to_path_buf(),
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: GraphNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: GraphEdge) {
        self.edges.push(edge);
    }

    /// Extract simple Rust `mod` or `use` dependencies from file content.
    pub fn extract_rust_dependencies(&mut self, file_path: &Path, content: &str) {
        let file_id = file_path.to_string_lossy().to_string();
        if !self.nodes.contains_key(&file_id) {
            self.add_node(GraphNode::new(file_path, false));
        }

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("mod ") || trimmed.starts_with("pub mod ") {
                let mod_name = trimmed
                    .trim_start_matches("pub ")
                    .trim_start_matches("mod ")
                    .trim_end_matches(';')
                    .trim();
                let target_id = format!("{}/{}", file_path.parent().unwrap_or(Path::new("")).display(), mod_name);
                self.add_edge(GraphEdge::new(&file_id, target_id, EdgeKind::Import));
            } else if trimmed.starts_with("use crate::") {
                let use_path = trimmed.trim_start_matches("use crate::").trim_end_matches(';').trim();
                self.add_edge(GraphEdge::new(&file_id, format!("crate::{}", use_path), EdgeKind::TypeReference));
            }
        }
    }

    /// Serializes the graph into a JSON payload formatted for Xavier Node Core (:8006).
    pub fn to_xavier_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "project_root": self.project_root,
            "total_nodes": self.nodes.len(),
            "total_edges": self.edges.len(),
            "nodes": self.nodes.values().collect::<Vec<_>>(),
            "edges": self.edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_creation() {
        let node = GraphNode::new("/home/user/project/src/main.rs", false);
        assert_eq!(node.label, "main.rs");
        assert_eq!(node.file_extension, Some("rs".to_string()));
        assert!(!node.is_directory);
    }

    #[test]
    fn test_file_dependency_graph_building() {
        let mut graph = FileDependencyGraph::new("/home/user/project");
        let rust_code = "\
pub mod scanner;
pub mod types;
use crate::types::FileEntry;
";
        let main_path = Path::new("/home/user/project/src/lib.rs");
        graph.extract_rust_dependencies(main_path, rust_code);

        assert!(graph.nodes.contains_key("/home/user/project/src/lib.rs"));
        assert_eq!(graph.edges.len(), 3);
        assert_eq!(graph.edges[0].kind, EdgeKind::Import);
        assert_eq!(graph.edges[2].kind, EdgeKind::TypeReference);

        let payload = graph.to_xavier_payload();
        assert_eq!(payload["total_nodes"], 1);
        assert_eq!(payload["total_edges"], 3);
    }
}
