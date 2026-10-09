use super::types::*;
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub struct GateNode {
    pub gate: PrimitiveGate,
    pub state: u8,
    pub dependents: Vec<usize>,
    pub in_queue: bool,
    pub depth: usize,
}

pub struct Simulator {
    pub nodes: slab::Slab<GateNode>,
    pub event_queue: Vec<Vec<usize>>,
    pub dynamic_threshold: usize,
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            nodes: slab::Slab::new(),
            event_queue: Vec::new(),
            dynamic_threshold: crate::engine::profiler::detect_parallel_crossover_threshold(),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.event_queue.clear();
    }

    pub fn set_single_threaded(&mut self, single: bool) {
        if single {
            self.dynamic_threshold = usize::MAX;
        } else {
            self.dynamic_threshold = crate::engine::profiler::detect_parallel_crossover_threshold();
        }
    }

    /// Adds a gate of a specific type to the simulator.
    /// Inputs are initially set to None (floating).
    /// Returns the unique index of the added gate.
    pub fn add_gate(&mut self, gate_type: GateType) -> usize {
        // Removing event_queue.clear() to preserve `in_queue` invariants for existing gates.

        // 0b00 = Floating, 0b01 = Low, 0b10 = High, 0b11 = Contention
        let initial_state = match gate_type {
            GateType::Nand => 0b10,
            GateType::Input | GateType::Output => 0b01,
            GateType::TriStateBuffer => 0b00,
            GateType::BusResolver => 0b00,
        };

        let node = GateNode {
            gate: PrimitiveGate {
                gate_type,
                input_a_source: None,
                input_b_source: None,
            },
            state: initial_state,
            dependents: Vec::new(),
            in_queue: true,
            depth: 0,
        };

        let index = self.nodes.insert(node);
        if self.event_queue.is_empty() {
            self.event_queue.push(Vec::new());
        }
        self.event_queue[0].push(index);

        index
    }

    /// Removes a gate and automatically disconnects all dependents and sources
    pub fn remove_gate(&mut self, gate_idx: usize) {
        if !self.nodes.contains(gate_idx) {
            return;
        }

        // Removing event_queue.clear() to preserve `in_queue` invariants for existing gates.
        // 1. Tell all dependents to forget about us
        let deps = self.nodes[gate_idx].dependents.clone();
        for dep_idx in deps {
            let mut needs_enqueue = false;
            if let Some(dep_node) = self.nodes.get_mut(dep_idx) {
                if dep_node.gate.input_a_source == Some(gate_idx) {
                    dep_node.gate.input_a_source = None;
                    needs_enqueue = true;
                }
                if dep_node.gate.input_b_source == Some(gate_idx) {
                    dep_node.gate.input_b_source = None;
                    needs_enqueue = true;
                }
            }
            if needs_enqueue {
                self.enqueue(dep_idx);
            }
        }

        // 2. Tell our sources to stop tracking us as a dependent
        let a_src = self.nodes[gate_idx].gate.input_a_source;
        let b_src = self.nodes[gate_idx].gate.input_b_source;

        if let Some(s_idx) = a_src
            && let Some(src_node) = self.nodes.get_mut(s_idx)
        {
            src_node.dependents.retain(|&x| x != gate_idx);
        }
        if let Some(s_idx) = b_src
            && let Some(src_node) = self.nodes.get_mut(s_idx)
        {
            src_node.dependents.retain(|&x| x != gate_idx);
        }

        self.nodes.remove(gate_idx);
    }

    /// Connects the output of source_idx to target_idx on the specified port.
    /// port is 0 for input_a_source, 1 for input_b_source.
    pub fn connect(&mut self, source_idx: usize, target_idx: usize, port: u8) {
        // Removing event_queue.clear() to preserve `in_queue` invariants for existing gates.

        assert!(
            self.nodes.contains(source_idx),
            "Source gate index out of bounds: {}",
            source_idx
        );
        assert!(
            self.nodes.contains(target_idx),
            "Target gate index out of bounds: {}",
            target_idx
        );

        let target_node = &mut self.nodes[target_idx];
        if port == 0 {
            target_node.gate.input_a_source = Some(source_idx);
        } else {
            target_node.gate.input_b_source = Some(source_idx);
        }

        self.nodes[source_idx].dependents.push(target_idx);

        // Queue the target gate because its connection just changed
        self.enqueue(target_idx);
    }

    /// Sets the value of an Input gate.
    /// If the state changes, enqueues all dependents for evaluation.
    pub fn set_input(&mut self, gate_idx: usize, value: bool) {
        assert!(
            self.nodes.contains(gate_idx),
            "Gate index out of bounds: {}",
            gate_idx
        );
        assert!(
            self.nodes[gate_idx].gate.gate_type == GateType::Input,
            "Cannot set input on a non-Input gate: {:?}",
            self.nodes[gate_idx].gate.gate_type
        );

        let new_state = if value { 0b10 } else { 0b01 };
        if self.nodes[gate_idx].state != new_state {
            self.nodes[gate_idx].state = new_state;
            let deps = self.nodes[gate_idx].dependents.clone();
            for dep_idx in deps {
                self.enqueue(dep_idx);
            }
        }
    }

    fn enqueue(&mut self, gate_idx: usize) {
        if let Some(node) = self.nodes.get_mut(gate_idx)
            && !node.in_queue
        {
            node.in_queue = true;
            let depth = node.depth;
            if self.event_queue.len() <= depth {
                self.event_queue.resize(depth + 1, Vec::new());
            }
            self.event_queue[depth].push(gate_idx);
        }
    }

    pub fn calculate_depths(&mut self) {
        let num_nodes = self.nodes.capacity();
        if num_nodes == 0 {
            return;
        }

        let mut index = 0;
        let mut indices = vec![None; num_nodes];
        let mut lowlinks = vec![0; num_nodes];
        let mut on_stack = vec![false; num_nodes];
        let mut stack = Vec::new();
        let mut sccs = Vec::new();

        let mut call_stack = Vec::new();

        for start_node in 0..num_nodes {
            if !self.nodes.contains(start_node) || indices[start_node].is_some() {
                continue;
            }

            call_stack.push((start_node, 0));

            while let Some((v, edge_idx)) = call_stack.pop() {
                if edge_idx == 0 {
                    indices[v] = Some(index);
                    lowlinks[v] = index;
                    index += 1;
                    stack.push(v);
                    on_stack[v] = true;
                }

                let mut returned = false;
                let dependents_len = self.nodes[v].dependents.len();

                let mut current_edge = edge_idx;
                while current_edge < dependents_len {
                    let w = self.nodes[v].dependents[current_edge];
                    if !self.nodes.contains(w) {
                        current_edge += 1;
                        continue;
                    }
                    if indices[w].is_none() {
                        call_stack.push((v, current_edge + 1));
                        call_stack.push((w, 0));
                        returned = true;
                        break;
                    } else if on_stack[w] {
                        lowlinks[v] = lowlinks[v].min(indices[w].unwrap());
                    }
                    current_edge += 1;
                }

                if returned {
                    continue;
                }

                if lowlinks[v] == indices[v].unwrap() {
                    let mut scc = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    sccs.push(scc);
                }

                if let Some(&(parent, _)) = call_stack.last() {
                    lowlinks[parent] = lowlinks[parent].min(lowlinks[v]);
                }
            }
        }

        sccs.reverse();

        let num_sccs = sccs.len();
        let mut node_to_scc = vec![usize::MAX; num_nodes];
        for (scc_idx, scc) in sccs.iter().enumerate() {
            for &node in scc {
                if node < num_nodes {
                    node_to_scc[node] = scc_idx;
                }
            }
        }

        let mut scc_depth = vec![0; num_sccs];
        for scc_idx in 0..num_sccs {
            let current_depth = scc_depth[scc_idx];
            for &node in &sccs[scc_idx] {
                if let Some(n) = self.nodes.get(node) {
                    for &dep in &n.dependents {
                        if dep < num_nodes {
                            let dep_scc = node_to_scc[dep];
                            if dep_scc != usize::MAX && dep_scc != scc_idx {
                                scc_depth[dep_scc] = scc_depth[dep_scc].max(current_depth + 1);
                            }
                        }
                    }
                }
            }
        }

        for (scc_idx, scc) in sccs.iter().enumerate() {
            let depth = scc_depth[scc_idx];
            for &node in scc {
                if let Some(n) = self.nodes.get_mut(node) {
                    n.depth = depth;
                }
            }
        }
    }

    pub fn defragment_and_sort_by_depth(&mut self) -> Vec<usize> {
        let capacity = self.nodes.capacity();

        let is_already_compact_and_sorted = self.nodes.len() == capacity
            && self
                .nodes
                .iter()
                .zip(self.nodes.iter().skip(1))
                .all(|((_, a), (_, b))| a.depth <= b.depth);

        if is_already_compact_and_sorted {
            return (0..capacity).collect();
        }

        let mut old_to_new = vec![usize::MAX; capacity];
        let old_nodes = std::mem::take(&mut self.nodes);
        let mut valid_nodes: Vec<(usize, GateNode)> = old_nodes.into_iter().collect();

        valid_nodes.sort_by_key(|(_, node)| node.depth);

        let mut new_slab = slab::Slab::with_capacity(valid_nodes.len());
        for (old_idx, node) in valid_nodes {
            let new_idx = new_slab.insert(node);
            old_to_new[old_idx] = new_idx;
        }

        for (_, node) in new_slab.iter_mut() {
            if let Some(src) = node.gate.input_a_source
                && src < old_to_new.len()
                && old_to_new[src] != usize::MAX
            {
                node.gate.input_a_source = Some(old_to_new[src]);
            }
            if let Some(src) = node.gate.input_b_source
                && src < old_to_new.len()
                && old_to_new[src] != usize::MAX
            {
                node.gate.input_b_source = Some(old_to_new[src]);
            }
            for dep in &mut node.dependents {
                if *dep < old_to_new.len() && old_to_new[*dep] != usize::MAX {
                    *dep = old_to_new[*dep];
                }
            }
        }

        for depth_queue in &mut self.event_queue {
            for idx in depth_queue.iter_mut() {
                if *idx < old_to_new.len() && old_to_new[*idx] != usize::MAX {
                    *idx = old_to_new[*idx];
                }
            }
        }

        self.nodes = new_slab;

        old_to_new
    }

    pub fn propagate_events(&mut self, max_steps_multiplier: usize) -> Result<usize, String> {
        let mut total_steps = 0;
        let max_steps = self.nodes.capacity() * max_steps_multiplier.max(100);

        let mut depth = 0;
        while depth < self.event_queue.len() {
            if self.event_queue[depth].is_empty() {
                depth += 1;
                continue;
            }

            let current_queue = std::mem::take(&mut self.event_queue[depth]);
            for &idx in &current_queue {
                if let Some(node) = self.nodes.get_mut(idx) {
                    node.in_queue = false;
                }
            }

            let nodes = &self.nodes;
            let compute_state = |&idx: &usize| -> Option<(usize, u8)> {
                if !nodes.contains(idx) {
                    return None;
                }

                let val_a = nodes[idx]
                    .gate
                    .input_a_source
                    .and_then(|s_idx| nodes.get(s_idx))
                    .map(|s_node| s_node.state)
                    .unwrap_or(0b00);
                let val_b = nodes[idx]
                    .gate
                    .input_b_source
                    .and_then(|s_idx| nodes.get(s_idx))
                    .map(|s_node| s_node.state)
                    .unwrap_or(0b00);

                let node = &nodes[idx];
                let new_state = match node.gate.gate_type {
                    GateType::Input => node.state,
                    GateType::Output => val_a,
                    GateType::Nand => {
                        let a_bool = (val_a & 0b10) != 0;
                        let b_bool = (val_b & 0b10) != 0;
                        if !(a_bool && b_bool) { 0b10 } else { 0b01 }
                    }
                    GateType::TriStateBuffer => {
                        let en_bool = (val_b & 0b10) != 0;
                        if en_bool {
                            let data_bool = (val_a & 0b10) != 0;
                            if data_bool { 0b10 } else { 0b01 }
                        } else {
                            0b00
                        }
                    }
                    GateType::BusResolver => val_a | val_b,
                };

                if new_state != node.state {
                    Some((idx, new_state))
                } else {
                    None
                }
            };

            let updates: Vec<(usize, u8)> = if current_queue.len() >= self.dynamic_threshold {
                // Use .map().collect() to force IndexedParallelIterator, preserving EXACT deterministic order,
                // then flatten sequentially. This prevents Rayon's filter_map chunking from causing non-deterministic event loops.
                current_queue
                    .par_iter()
                    .map(compute_state)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .flatten()
                    .collect()
            } else {
                current_queue.iter().filter_map(compute_state).collect()
            };

            let mut next_enqueues = Vec::with_capacity(updates.len());
            for (idx, new_state) in updates {
                self.nodes[idx].state = new_state;
                next_enqueues.push(idx);
            }

            total_steps += current_queue.len();
            if total_steps >= max_steps {
                return Err(format!(
                    "Oscillation detected: exceeded max_steps limit of {}",
                    max_steps
                ));
            }

            for idx in next_enqueues {
                let deps = std::mem::take(&mut self.nodes[idx].dependents);
                for &dep_idx in &deps {
                    self.enqueue(dep_idx);
                }
                self.nodes[idx].dependents = deps;
            }

            if self.event_queue[depth].is_empty() {
                depth += 1;
            }

            if depth >= self.event_queue.len()
                && let Some(first_non_empty) = self.event_queue.iter().position(|q| !q.is_empty())
            {
                depth = first_non_empty;
            }
        }

        Ok(total_steps)
    }

    pub fn get_raw_state(&self, gate_idx: usize) -> u8 {
        self.nodes
            .get(gate_idx)
            .map(|node| node.state)
            .unwrap_or(0b00)
    }

    pub fn get_state(&self, gate_idx: usize) -> bool {
        self.nodes
            .get(gate_idx)
            .map(|node| node.state == 0b10)
            .unwrap_or(false)
    }
}
