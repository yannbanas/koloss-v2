use super::graph::{KnowledgeGraph, NodeId};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct AnalogicalMapping {
    pub source_nodes: Vec<NodeId>,
    pub target_nodes: Vec<NodeId>,
    pub node_map: FxHashMap<NodeId, NodeId>,
    pub score: f64,
}

pub fn structure_map(
    graph: &KnowledgeGraph,
    source_root: NodeId,
    target_root: NodeId,
    max_depth: usize,
) -> Option<AnalogicalMapping> {
    let source_sub = extract_subgraph(graph, source_root, max_depth);
    let target_sub = extract_subgraph(graph, target_root, max_depth);

    if source_sub.is_empty() || target_sub.is_empty() {
        return None;
    }

    let mut best_map: FxHashMap<NodeId, NodeId> = FxHashMap::default();
    let mut best_score = 0.0;

    best_map.insert(source_root, target_root);
    let initial_score = node_similarity(graph, source_root, target_root);

    let source_edges = graph.outgoing_edges(source_root);
    let target_edges = graph.outgoing_edges(target_root);

    for se in &source_edges {
        for te in &target_edges {
            if se.relation == te.relation {
                let sub_score = node_similarity(graph, se.target, te.target);
                if sub_score > 0.0 {
                    best_map.insert(se.target, te.target);
                    best_score += sub_score + 0.5;
                }
            }
        }
    }

    best_score += initial_score;

    if best_map.len() < 2 {
        return None;
    }

    let total_possible = source_sub.len().min(target_sub.len()) as f64;
    let normalized_score = if total_possible > 0.0 {
        best_score / total_possible
    } else {
        0.0
    };

    Some(AnalogicalMapping {
        source_nodes: source_sub,
        target_nodes: target_sub,
        node_map: best_map,
        score: normalized_score,
    })
}

fn extract_subgraph(graph: &KnowledgeGraph, root: NodeId, max_depth: usize) -> Vec<NodeId> {
    let mut visited = Vec::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back((root, 0usize));

    while let Some((node, depth)) = queue.pop_front() {
        if visited.contains(&node) || depth > max_depth {
            continue;
        }
        visited.push(node);
        for neighbor in graph.neighbors(node) {
            queue.push_back((neighbor, depth + 1));
        }
    }
    visited
}

fn node_similarity(graph: &KnowledgeGraph, a: NodeId, b: NodeId) -> f64 {
    let na = match graph.node(a) {
        Some(n) => n,
        None => return 0.0,
    };
    let nb = match graph.node(b) {
        Some(n) => n,
        None => return 0.0,
    };

    let mut score = 0.0;

    if na.label == nb.label {
        score += 1.0;
    }

    let a_out: Vec<_> = graph.outgoing_edges(a).iter().map(|e| e.relation).collect();
    let b_out: Vec<_> = graph.outgoing_edges(b).iter().map(|e| e.relation).collect();
    let common = a_out.iter().filter(|r| b_out.contains(r)).count();
    let total = a_out.len().max(b_out.len());
    if total > 0 {
        score += common as f64 / total as f64;
    }

    score
}

pub fn find_analogies(graph: &KnowledgeGraph, query_root: NodeId, candidates: &[NodeId], max_depth: usize, min_score: f64) -> Vec<AnalogicalMapping> {
    let mut results = Vec::new();
    for &candidate in candidates {
        if candidate == query_root {
            continue;
        }
        if let Some(mapping) = structure_map(graph, query_root, candidate, max_depth) {
            if mapping.score >= min_score {
                results.push(mapping);
            }
        }
    }
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    results
}
