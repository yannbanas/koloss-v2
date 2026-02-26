use std::collections::VecDeque;

pub trait SearchState: Clone + std::fmt::Debug {
    type Action: Clone + std::fmt::Debug;
    fn actions(&self) -> Vec<Self::Action>;
    fn apply(&self, action: &Self::Action) -> Self;
    fn is_goal(&self) -> bool;
    fn heuristic(&self) -> f64;
    fn cost(&self) -> f64;
}

#[derive(Debug, Clone)]
pub struct SearchResult<S: SearchState> {
    pub state: S,
    pub actions: Vec<S::Action>,
    pub nodes_explored: usize,
    pub depth: usize,
}

pub fn dfs<S: SearchState>(initial: S, max_depth: usize) -> Option<SearchResult<S>> {
    let mut stack: Vec<(S, Vec<S::Action>, usize)> = vec![(initial, Vec::new(), 0)];
    let mut explored = 0usize;

    while let Some((state, actions, depth)) = stack.pop() {
        explored += 1;
        if state.is_goal() {
            return Some(SearchResult { state, actions, nodes_explored: explored, depth });
        }
        if depth >= max_depth {
            continue;
        }
        for action in state.actions().into_iter().rev() {
            let new_state = state.apply(&action);
            let mut new_actions = actions.clone();
            new_actions.push(action);
            stack.push((new_state, new_actions, depth + 1));
        }
    }
    None
}

pub fn bfs<S: SearchState>(initial: S, max_depth: usize) -> Option<SearchResult<S>> {
    let mut queue: VecDeque<(S, Vec<S::Action>, usize)> = VecDeque::new();
    queue.push_back((initial, Vec::new(), 0));
    let mut explored = 0usize;

    while let Some((state, actions, depth)) = queue.pop_front() {
        explored += 1;
        if state.is_goal() {
            return Some(SearchResult { state, actions, nodes_explored: explored, depth });
        }
        if depth >= max_depth {
            continue;
        }
        for action in state.actions() {
            let new_state = state.apply(&action);
            let mut new_actions = actions.clone();
            new_actions.push(action);
            queue.push_back((new_state, new_actions, depth + 1));
        }
    }
    None
}

pub fn beam_search<S: SearchState>(initial: S, beam_width: usize, max_depth: usize) -> Option<SearchResult<S>> {
    let mut beam: Vec<(S, Vec<S::Action>)> = vec![(initial, Vec::new())];
    let mut explored = 0usize;

    for depth in 0..max_depth {
        let mut candidates: Vec<(S, Vec<S::Action>, f64)> = Vec::new();

        for (state, actions) in &beam {
            explored += 1;
            if state.is_goal() {
                return Some(SearchResult {
                    state: state.clone(),
                    actions: actions.clone(),
                    nodes_explored: explored,
                    depth,
                });
            }
            for action in state.actions() {
                let new_state = state.apply(&action);
                let h = new_state.heuristic();
                let mut new_actions = actions.clone();
                new_actions.push(action);
                candidates.push((new_state, new_actions, h));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        candidates.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
        beam = candidates.into_iter()
            .take(beam_width)
            .map(|(s, a, _)| (s, a))
            .collect();
    }
    None
}

pub fn iterative_deepening<S: SearchState>(initial: S, max_depth: usize) -> Option<SearchResult<S>> {
    for depth in 1..=max_depth {
        if let Some(result) = dfs(initial.clone(), depth) {
            return Some(result);
        }
    }
    None
}

#[derive(Debug)]
pub struct MctsNode<S: SearchState> {
    state: S,
    action: Option<S::Action>,
    visits: u32,
    total_reward: f64,
    children: Vec<MctsNode<S>>,
    unexpanded: Vec<S::Action>,
}

impl<S: SearchState> MctsNode<S> {
    fn new(state: S, action: Option<S::Action>) -> Self {
        let unexpanded = state.actions();
        Self {
            state,
            action,
            visits: 0,
            total_reward: 0.0,
            children: Vec::new(),
            unexpanded,
        }
    }

    fn ucb1(&self, parent_visits: u32, c: f64) -> f64 {
        if self.visits == 0 {
            return f64::INFINITY;
        }
        let exploitation = self.total_reward / self.visits as f64;
        let exploration = c * ((parent_visits as f64).ln() / self.visits as f64).sqrt();
        exploitation + exploration
    }

    fn best_child_idx(&self, c: f64) -> usize {
        self.children.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                a.ucb1(self.visits, c)
                    .partial_cmp(&b.ucb1(self.visits, c))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    fn expand(&mut self) -> Option<usize> {
        if let Some(action) = self.unexpanded.pop() {
            let new_state = self.state.apply(&action);
            let child = MctsNode::new(new_state, Some(action));
            self.children.push(child);
            Some(self.children.len() - 1)
        } else {
            None
        }
    }
}

pub fn mcts<S: SearchState>(initial: S, iterations: usize, max_depth: usize) -> Option<SearchResult<S>> {
    let mut root = MctsNode::new(initial, None);
    let c = 1.414;

    for _ in 0..iterations {
        let reward = select_and_simulate(&mut root, max_depth, 0, c);
        root.visits += 1;
        root.total_reward += reward;
    }

    if root.children.is_empty() {
        return None;
    }

    let best_idx = root.children.iter()
        .enumerate()
        .max_by_key(|(_, c)| c.visits)
        .map(|(i, _)| i)?;

    let mut actions = Vec::new();
    let mut current = &root.children[best_idx];
    if let Some(ref a) = current.action {
        actions.push(a.clone());
    }
    while !current.children.is_empty() {
        let idx = current.children.iter()
            .enumerate()
            .max_by_key(|(_, c)| c.visits)
            .map(|(i, _)| i)
            .unwrap_or(0);
        current = &current.children[idx];
        if let Some(ref a) = current.action {
            actions.push(a.clone());
        }
    }

    let depth = actions.len();
    Some(SearchResult {
        state: current.state.clone(),
        actions,
        nodes_explored: root.visits as usize,
        depth,
    })
}

fn select_and_simulate<S: SearchState>(node: &mut MctsNode<S>, max_depth: usize, depth: usize, c: f64) -> f64 {
    if node.state.is_goal() {
        return 1.0;
    }
    if depth >= max_depth {
        return 1.0 - node.state.heuristic().min(1.0);
    }

    if !node.unexpanded.is_empty() {
        if let Some(child_idx) = node.expand() {
            let reward = simulate(&node.children[child_idx].state, max_depth, depth + 1);
            node.children[child_idx].visits += 1;
            node.children[child_idx].total_reward += reward;
            return reward;
        }
    }

    if node.children.is_empty() {
        return simulate(&node.state, max_depth, depth);
    }

    let idx = node.best_child_idx(c);
    let reward = select_and_simulate(&mut node.children[idx], max_depth, depth + 1, c);
    node.children[idx].visits += 1;
    node.children[idx].total_reward += reward;
    reward
}

fn simulate<S: SearchState>(state: &S, max_depth: usize, depth: usize) -> f64 {
    let mut current = state.clone();
    for _ in depth..max_depth {
        if current.is_goal() {
            return 1.0;
        }
        let actions = current.actions();
        if actions.is_empty() {
            break;
        }
        let idx = (current.heuristic() * 1000.0) as usize % actions.len();
        current = current.apply(&actions[idx]);
    }
    if current.is_goal() { 1.0 } else { 1.0 - current.heuristic().min(1.0) }
}
