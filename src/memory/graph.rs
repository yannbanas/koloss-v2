use crate::core::{Term, Sym, SymbolTable};
use rustc_hash::FxHashMap;

pub type NodeId = u32;
pub type EdgeId = u32;

#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub label: Sym,
    pub attributes: FxHashMap<Sym, Term>,
    pub created_at: u64,
    pub access_count: u32,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub id: EdgeId,
    pub relation: Sym,
    pub source: NodeId,
    pub target: NodeId,
    pub weight: f64,
    pub attributes: FxHashMap<Sym, Term>,
}

#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    nodes: FxHashMap<NodeId, Node>,
    edges: FxHashMap<EdgeId, Edge>,
    outgoing: FxHashMap<NodeId, Vec<EdgeId>>,
    incoming: FxHashMap<NodeId, Vec<EdgeId>>,
    label_index: FxHashMap<Sym, Vec<NodeId>>,
    relation_index: FxHashMap<Sym, Vec<EdgeId>>,
    next_node_id: NodeId,
    next_edge_id: EdgeId,
    tick: u64,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: FxHashMap::default(),
            edges: FxHashMap::default(),
            outgoing: FxHashMap::default(),
            incoming: FxHashMap::default(),
            label_index: FxHashMap::default(),
            relation_index: FxHashMap::default(),
            next_node_id: 1,
            next_edge_id: 1,
            tick: 0,
        }
    }

    pub fn add_node(&mut self, label: Sym) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let node = Node {
            id,
            label,
            attributes: FxHashMap::default(),
            created_at: self.tick,
            access_count: 0,
        };
        self.nodes.insert(id, node);
        self.label_index.entry(label).or_default().push(id);
        id
    }

    pub fn add_node_with_attrs(&mut self, label: Sym, attrs: Vec<(Sym, Term)>) -> NodeId {
        let id = self.add_node(label);
        if let Some(node) = self.nodes.get_mut(&id) {
            for (k, v) in attrs {
                node.attributes.insert(k, v);
            }
        }
        id
    }

    pub fn add_edge(&mut self, source: NodeId, relation: Sym, target: NodeId) -> EdgeId {
        let id = self.next_edge_id;
        self.next_edge_id += 1;
        let edge = Edge {
            id,
            relation,
            source,
            target,
            weight: 1.0,
            attributes: FxHashMap::default(),
        };
        self.edges.insert(id, edge);
        self.outgoing.entry(source).or_default().push(id);
        self.incoming.entry(target).or_default().push(id);
        self.relation_index.entry(relation).or_default().push(id);
        id
    }

    pub fn add_edge_weighted(&mut self, source: NodeId, relation: Sym, target: NodeId, weight: f64) -> EdgeId {
        let id = self.add_edge(source, relation, target);
        if let Some(edge) = self.edges.get_mut(&id) {
            edge.weight = weight;
        }
        id
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.nodes.get_mut(&id)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    pub fn nodes_by_label(&self, label: Sym) -> Vec<NodeId> {
        self.label_index.get(&label).cloned().unwrap_or_default()
    }

    pub fn edges_by_relation(&self, relation: Sym) -> Vec<EdgeId> {
        self.relation_index.get(&relation).cloned().unwrap_or_default()
    }

    pub fn outgoing_edges(&self, node: NodeId) -> Vec<&Edge> {
        self.outgoing.get(&node)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn incoming_edges(&self, node: NodeId) -> Vec<&Edge> {
        self.incoming.get(&node)
            .map(|ids| ids.iter().filter_map(|id| self.edges.get(id)).collect())
            .unwrap_or_default()
    }

    pub fn neighbors(&self, node: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        for edge in self.outgoing_edges(node) {
            if !result.contains(&edge.target) {
                result.push(edge.target);
            }
        }
        for edge in self.incoming_edges(node) {
            if !result.contains(&edge.source) {
                result.push(edge.source);
            }
        }
        result
    }

    pub fn find_path(&self, from: NodeId, to: NodeId, max_depth: usize) -> Option<Vec<EdgeId>> {
        let mut queue = std::collections::VecDeque::new();
        let mut visited = rustc_hash::FxHashSet::default();
        queue.push_back((from, Vec::new()));
        visited.insert(from);

        while let Some((current, path)) = queue.pop_front() {
            if current == to {
                return Some(path);
            }
            if path.len() >= max_depth {
                continue;
            }
            for edge in self.outgoing_edges(current) {
                if !visited.contains(&edge.target) {
                    visited.insert(edge.target);
                    let mut new_path = path.clone();
                    new_path.push(edge.id);
                    queue.push_back((edge.target, new_path));
                }
            }
        }
        None
    }

    pub fn query_triple(&self, source_label: Option<Sym>, relation: Option<Sym>, target_label: Option<Sym>) -> Vec<(NodeId, EdgeId, NodeId)> {
        let mut results = Vec::new();
        for edge in self.edges.values() {
            if let Some(rel) = relation {
                if edge.relation != rel {
                    continue;
                }
            }
            if let Some(sl) = source_label {
                if self.nodes.get(&edge.source).map(|n| n.label) != Some(sl) {
                    continue;
                }
            }
            if let Some(tl) = target_label {
                if self.nodes.get(&edge.target).map(|n| n.label) != Some(tl) {
                    continue;
                }
            }
            results.push((edge.source, edge.id, edge.target));
        }
        results
    }

    pub fn remove_node(&mut self, id: NodeId) -> bool {
        if self.nodes.remove(&id).is_none() {
            return false;
        }
        let edge_ids: Vec<EdgeId> = self.outgoing.remove(&id).unwrap_or_default()
            .into_iter()
            .chain(self.incoming.remove(&id).unwrap_or_default())
            .collect();
        for eid in edge_ids {
            self.remove_edge(eid);
        }
        for ids in self.label_index.values_mut() {
            ids.retain(|n| *n != id);
        }
        true
    }

    pub fn remove_edge(&mut self, id: EdgeId) -> bool {
        if let Some(edge) = self.edges.remove(&id) {
            if let Some(out) = self.outgoing.get_mut(&edge.source) {
                out.retain(|e| *e != id);
            }
            if let Some(inc) = self.incoming.get_mut(&edge.target) {
                inc.retain(|e| *e != id);
            }
            if let Some(rels) = self.relation_index.get_mut(&edge.relation) {
                rels.retain(|e| *e != id);
            }
            true
        } else {
            false
        }
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn tick(&mut self) {
        self.tick += 1;
    }

    pub fn to_terms(&self, _syms: &SymbolTable) -> Vec<Term> {
        let mut terms = Vec::new();
        for edge in self.edges.values() {
            let s_label = self.nodes.get(&edge.source).map(|n| n.label).unwrap_or(0);
            let t_label = self.nodes.get(&edge.target).map(|n| n.label).unwrap_or(0);
            terms.push(Term::compound(edge.relation, vec![
                Term::atom(s_label),
                Term::atom(t_label),
            ]));
        }
        terms
    }
}
