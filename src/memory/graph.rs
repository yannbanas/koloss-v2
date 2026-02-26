use crate::core::{Term, Sym, SymbolTable};
use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};

pub type NodeId = u32;
pub type EdgeId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub label: Sym,
    pub attributes: Vec<(Sym, TermSer)>,
    pub created_at: u64,
    pub last_access: u64,
    pub access_count: u32,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub relation: Sym,
    pub source: NodeId,
    pub target: NodeId,
    pub weight: f64,
    pub attributes: Vec<(Sym, TermSer)>,
    pub created_at: u64,
    pub last_access: u64,
    pub access_count: u32,
}

// Serializable term subset (for persistence)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TermSer {
    Atom(Sym),
    Int(i64),
    Str(String),
    Bool(bool),
}

impl TermSer {
    pub fn to_term(&self) -> Term {
        match self {
            TermSer::Atom(a) => Term::Atom(*a),
            TermSer::Int(n) => Term::Int(*n),
            TermSer::Str(s) => Term::Str(s.clone().into()),
            TermSer::Bool(b) => Term::Bool(*b),
        }
    }

    pub fn from_term(t: &Term) -> Option<Self> {
        match t {
            Term::Atom(a) => Some(TermSer::Atom(*a)),
            Term::Int(n) => Some(TermSer::Int(*n)),
            Term::Str(s) => Some(TermSer::Str(s.to_string())),
            Term::Bool(b) => Some(TermSer::Bool(*b)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub next_node_id: NodeId,
    pub next_edge_id: EdgeId,
    pub tick: u64,
}

#[derive(Debug, Clone)]
pub struct DecayConfig {
    pub decay_rate: f64,
    pub min_weight: f64,
    pub prune_threshold: f64,
    pub access_boost: f64,
}

impl Default for DecayConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.01,
            min_weight: 0.0,
            prune_threshold: 0.05,
            access_boost: 0.2,
        }
    }
}

// Symbolic embedding: subgraph → fixed-size vector
pub type Embedding = Vec<f64>;

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
    decay_config: DecayConfig,
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
            decay_config: DecayConfig::default(),
        }
    }

    pub fn with_decay(mut self, config: DecayConfig) -> Self {
        self.decay_config = config;
        self
    }

    // --- Persistence ---

    pub fn save(&self) -> GraphSnapshot {
        GraphSnapshot {
            nodes: self.nodes.values().cloned().collect(),
            edges: self.edges.values().cloned().collect(),
            next_node_id: self.next_node_id,
            next_edge_id: self.next_edge_id,
            tick: self.tick,
        }
    }

    pub fn save_json(&self) -> String {
        serde_json::to_string(&self.save()).unwrap_or_default()
    }

    pub fn load(snapshot: &GraphSnapshot) -> Self {
        let mut g = Self::new();
        g.next_node_id = snapshot.next_node_id;
        g.next_edge_id = snapshot.next_edge_id;
        g.tick = snapshot.tick;

        for node in &snapshot.nodes {
            g.nodes.insert(node.id, node.clone());
            g.label_index.entry(node.label).or_default().push(node.id);
        }
        for edge in &snapshot.edges {
            g.edges.insert(edge.id, edge.clone());
            g.outgoing.entry(edge.source).or_default().push(edge.id);
            g.incoming.entry(edge.target).or_default().push(edge.id);
            g.relation_index.entry(edge.relation).or_default().push(edge.id);
        }
        g
    }

    pub fn load_json(json: &str) -> Option<Self> {
        serde_json::from_str::<GraphSnapshot>(json).ok().map(|s| Self::load(&s))
    }

    // --- Temporal Decay ---

    pub fn apply_decay(&mut self) {
        let rate = self.decay_config.decay_rate;
        let min = self.decay_config.min_weight;

        for node in self.nodes.values_mut() {
            let age = self.tick.saturating_sub(node.last_access) as f64;
            node.weight = (node.weight - rate * age).max(min);
        }
        for edge in self.edges.values_mut() {
            let age = self.tick.saturating_sub(edge.last_access) as f64;
            edge.weight = (edge.weight - rate * age).max(min);
        }
    }

    pub fn prune_weak(&mut self) -> usize {
        let threshold = self.decay_config.prune_threshold;
        let weak_nodes: Vec<NodeId> = self.nodes.values()
            .filter(|n| n.weight < threshold)
            .map(|n| n.id)
            .collect();
        let mut removed = 0;
        for id in weak_nodes {
            if self.remove_node(id) { removed += 1; }
        }

        let weak_edges: Vec<EdgeId> = self.edges.values()
            .filter(|e| e.weight < threshold)
            .map(|e| e.id)
            .collect();
        for id in weak_edges {
            if self.remove_edge(id) { removed += 1; }
        }
        removed
    }

    fn touch_node(&mut self, id: NodeId) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.last_access = self.tick;
            node.access_count += 1;
            node.weight = (node.weight + self.decay_config.access_boost).min(1.0);
        }
    }

    pub fn touch_edge(&mut self, id: EdgeId) {
        if let Some(edge) = self.edges.get_mut(&id) {
            edge.last_access = self.tick;
            edge.access_count += 1;
            edge.weight = (edge.weight + self.decay_config.access_boost).min(1.0);
        }
    }

    // --- Graph Inference ---

    pub fn extract_patterns(&self) -> Vec<GraphPattern> {
        let mut patterns = Vec::new();

        // Pattern 1: Frequent relation pairs (A--r1-->B--r2-->C)
        for edge1 in self.edges.values() {
            if let Some(outgoing) = self.outgoing.get(&edge1.target) {
                for &eid2 in outgoing {
                    if let Some(edge2) = self.edges.get(&eid2) {
                        let s_label = self.nodes.get(&edge1.source).map(|n| n.label).unwrap_or(0);
                        let m_label = self.nodes.get(&edge1.target).map(|n| n.label).unwrap_or(0);
                        let t_label = self.nodes.get(&edge2.target).map(|n| n.label).unwrap_or(0);
                        patterns.push(GraphPattern::Chain {
                            source_label: s_label,
                            rel1: edge1.relation,
                            mid_label: m_label,
                            rel2: edge2.relation,
                            target_label: t_label,
                        });
                    }
                }
            }
        }

        // Pattern 2: Shared targets (A--r-->C and B--r-->C)
        for (&target, incoming) in &self.incoming {
            if incoming.len() >= 2 {
                let t_label = self.nodes.get(&target).map(|n| n.label).unwrap_or(0);
                let mut rels: FxHashMap<Sym, Vec<Sym>> = FxHashMap::default();
                for &eid in incoming {
                    if let Some(edge) = self.edges.get(&eid) {
                        let s_label = self.nodes.get(&edge.source).map(|n| n.label).unwrap_or(0);
                        rels.entry(edge.relation).or_default().push(s_label);
                    }
                }
                for (rel, sources) in rels {
                    if sources.len() >= 2 {
                        patterns.push(GraphPattern::SharedTarget {
                            relation: rel,
                            target_label: t_label,
                            source_labels: sources,
                        });
                    }
                }
            }
        }

        patterns
    }

    pub fn infer_rules(&self, syms: &SymbolTable) -> Vec<InferredRule> {
        let patterns = self.extract_patterns();
        let mut rules = Vec::new();

        for pattern in &patterns {
            match pattern {
                GraphPattern::Chain { source_label, rel1, mid_label: _, rel2, target_label } => {
                    // If A--r1-->B--r2-->C appears, infer: transitive_r1_r2(A, C) :- r1(A, B), r2(B, C)
                    let r1_name = syms.resolve(*rel1).unwrap_or("?");
                    let r2_name = syms.resolve(*rel2).unwrap_or("?");
                    rules.push(InferredRule {
                        head: format!("chain_{}_{}", r1_name, r2_name),
                        head_sym: (*source_label, *target_label),
                        body_rels: vec![*rel1, *rel2],
                        confidence: 0.5 + 0.1 * (self.edges.len() as f64).min(5.0),
                        support: 1,
                    });
                }
                GraphPattern::SharedTarget { relation, target_label, source_labels } => {
                    let r_name = syms.resolve(*relation).unwrap_or("?");
                    rules.push(InferredRule {
                        head: format!("shared_{}", r_name),
                        head_sym: (*target_label, *relation),
                        body_rels: vec![*relation],
                        confidence: 0.3 + 0.1 * (source_labels.len() as f64).min(7.0),
                        support: source_labels.len(),
                    });
                }
            }
        }
        rules
    }

    // --- Symbolic Embedding ---

    pub fn embed_node(&self, id: NodeId, dim: usize) -> Embedding {
        let mut vec = vec![0.0f64; dim];
        if let Some(node) = self.nodes.get(&id) {
            // Feature 0: label hash
            vec[0] = (node.label as f64) / 100.0;
            // Feature 1: degree
            let out_deg = self.outgoing.get(&id).map(|e| e.len()).unwrap_or(0);
            let in_deg = self.incoming.get(&id).map(|e| e.len()).unwrap_or(0);
            if dim > 1 { vec[1] = (out_deg + in_deg) as f64 / 10.0; }
            // Feature 2: out-degree ratio
            if dim > 2 { vec[2] = if out_deg + in_deg > 0 { out_deg as f64 / (out_deg + in_deg) as f64 } else { 0.5 }; }
            // Feature 3: weight
            if dim > 3 { vec[3] = node.weight; }
            // Feature 4: access count normalized
            if dim > 4 { vec[4] = (node.access_count as f64).ln_1p() / 10.0; }
            // Features 5+: relation type distribution
            let mut rel_counts: FxHashMap<Sym, usize> = FxHashMap::default();
            for edge in self.outgoing_edges(id) {
                *rel_counts.entry(edge.relation).or_default() += 1;
            }
            for (i, (_, count)) in rel_counts.iter().enumerate() {
                if 5 + i < dim {
                    vec[5 + i] = *count as f64;
                }
            }
        }
        vec
    }

    pub fn embed_subgraph(&self, center: NodeId, radius: usize, dim: usize) -> Embedding {
        let reachable = self.bfs_collect(center, radius);
        if reachable.is_empty() { return vec![0.0; dim]; }

        let mut sum = vec![0.0f64; dim];
        for &nid in &reachable {
            let emb = self.embed_node(nid, dim);
            for (i, v) in emb.iter().enumerate() {
                sum[i] += v;
            }
        }
        let n = reachable.len() as f64;
        sum.iter_mut().for_each(|v| *v /= n);
        sum
    }

    pub fn similarity(a: &Embedding, b: &Embedding) -> f64 {
        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if mag_a < f64::EPSILON || mag_b < f64::EPSILON { return 0.0; }
        dot / (mag_a * mag_b)
    }

    pub fn find_similar_nodes(&self, target: NodeId, dim: usize, top_k: usize) -> Vec<(NodeId, f64)> {
        let target_emb = self.embed_node(target, dim);
        let mut scores: Vec<(NodeId, f64)> = self.nodes.keys()
            .filter(|&&id| id != target)
            .map(|&id| {
                let emb = self.embed_node(id, dim);
                (id, Self::similarity(&target_emb, &emb))
            })
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    fn bfs_collect(&self, start: NodeId, max_depth: usize) -> Vec<NodeId> {
        let mut visited = rustc_hash::FxHashSet::default();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back((start, 0usize));
        visited.insert(start);

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth { continue; }
            for edge in self.outgoing_edges(current) {
                if visited.insert(edge.target) {
                    queue.push_back((edge.target, depth + 1));
                }
            }
        }
        visited.into_iter().collect()
    }

    // --- Original methods ---

    pub fn add_node(&mut self, label: Sym) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let node = Node {
            id,
            label,
            attributes: Vec::new(),
            created_at: self.tick,
            last_access: self.tick,
            access_count: 0,
            weight: 1.0,
        };
        self.nodes.insert(id, node);
        self.label_index.entry(label).or_default().push(id);
        id
    }

    pub fn add_node_with_attrs(&mut self, label: Sym, attrs: Vec<(Sym, Term)>) -> NodeId {
        let id = self.add_node(label);
        if let Some(node) = self.nodes.get_mut(&id) {
            for (k, v) in attrs {
                if let Some(ts) = TermSer::from_term(&v) {
                    node.attributes.push((k, ts));
                }
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
            attributes: Vec::new(),
            created_at: self.tick,
            last_access: self.tick,
            access_count: 0,
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
        self.touch_node_read(id);
        self.nodes.get(&id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.touch_node(id);
        self.nodes.get_mut(&id)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(&id)
    }

    fn touch_node_read(&self, _id: NodeId) {
        // Read-only access tracking would need interior mutability
        // For now, touch_node is called on mutable access
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
                if edge.relation != rel { continue; }
            }
            if let Some(sl) = source_label {
                if self.nodes.get(&edge.source).map(|n| n.label) != Some(sl) { continue; }
            }
            if let Some(tl) = target_label {
                if self.nodes.get(&edge.target).map(|n| n.label) != Some(tl) { continue; }
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

    pub fn current_tick(&self) -> u64 {
        self.tick
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

#[derive(Debug, Clone)]
pub enum GraphPattern {
    Chain {
        source_label: Sym,
        rel1: Sym,
        mid_label: Sym,
        rel2: Sym,
        target_label: Sym,
    },
    SharedTarget {
        relation: Sym,
        target_label: Sym,
        source_labels: Vec<Sym>,
    },
}

#[derive(Debug, Clone)]
pub struct InferredRule {
    pub head: String,
    pub head_sym: (Sym, Sym),
    pub body_rels: Vec<Sym>,
    pub confidence: f64,
    pub support: usize,
}
