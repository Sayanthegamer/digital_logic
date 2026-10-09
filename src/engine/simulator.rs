use super::sleep::SleepDomain;
use super::storage::{NO_SOURCE, SoAGateStorage};
use super::types::*;
use fixedbitset::FixedBitSet;
use rayon::prelude::*;

pub struct Simulator {
    pub nodes: SoAGateStorage,
    pub event_queue: Vec<Vec<usize>>,
    pub dynamic_threshold: usize,
    pub sleep_domains: Vec<SleepDomain>,
    pub sleeping_gates: FixedBitSet,
}

impl Default for Simulator {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            nodes: SoAGateStorage::new(),
            event_queue: Vec::new(),
            dynamic_threshold: crate::engine::profiler::detect_parallel_crossover_threshold(),
            sleep_domains: Vec::new(),
            sleeping_gates: FixedBitSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
        self.event_queue.clear();
        self.sleep_domains.clear();
        self.sleeping_gates.clear();
    }

    pub fn set_single_threaded(&mut self, single: bool) {
        if single {
            self.dynamic_threshold = usize::MAX;
        } else {
            self.dynamic_threshold = crate::engine::profiler::detect_parallel_crossover_threshold();
        }
    }

    pub fn register_sleep_domain(&mut self, mut domain: SleepDomain) {
        if domain.control_gate < self.nodes.states.len() {
            let ctrl_val = self.nodes.states[domain.control_gate];
            if domain.should_sleep(ctrl_val) {
                domain.hibernate(self);
            }
        }
        self.sleep_domains.push(domain);
    }

    pub fn is_domain_sleeping(&self, domain_id: usize) -> bool {
        self.sleep_domains
            .iter()
            .find(|d| d.id == domain_id)
            .map(|d| d.is_sleeping)
            .unwrap_or(false)
    }

    /// Adds a gate of a specific type to the simulator.
    /// Inputs are initially set to None (floating).
    /// Returns the unique index of the added gate.
    pub fn add_gate(&mut self, gate_type: GateType) -> usize {
        // 0b00 = Floating, 0b01 = Low, 0b10 = High, 0b11 = Contention
        let initial_state = match gate_type {
            GateType::Nand => 0b10,
            GateType::Input | GateType::Output => 0b01,
            GateType::TriStateBuffer | GateType::BusResolver => 0b00,
        };

        let index = self.nodes.insert(gate_type, initial_state);
        self.nodes.in_queue.set(index, true);

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

        // 1. Tell all dependents to forget about us
        let deps = self.nodes.dependents[gate_idx].clone();
        for dep_u32 in deps {
            let dep_idx = dep_u32 as usize;
            let mut needs_enqueue = false;
            if self.nodes.contains(dep_idx) {
                if self.nodes.sources[dep_idx][0] == gate_idx as u32 {
                    self.nodes.sources[dep_idx][0] = NO_SOURCE;
                    needs_enqueue = true;
                }
                if self.nodes.sources[dep_idx][1] == gate_idx as u32 {
                    self.nodes.sources[dep_idx][1] = NO_SOURCE;
                    needs_enqueue = true;
                }
            }
            if needs_enqueue {
                self.enqueue(dep_idx);
            }
        }

        // 2. Tell our sources to stop tracking us as a dependent
        let a_src = self.nodes.sources[gate_idx][0];
        let b_src = self.nodes.sources[gate_idx][1];

        if a_src != NO_SOURCE && self.nodes.contains(a_src as usize) {
            self.nodes.dependents[a_src as usize].retain(|&x| x != gate_idx as u32);
        }
        if b_src != NO_SOURCE && self.nodes.contains(b_src as usize) {
            self.nodes.dependents[b_src as usize].retain(|&x| x != gate_idx as u32);
        }

        self.nodes.remove(gate_idx);
    }

    /// Connects the output of source_idx to target_idx on the specified port.
    /// port is 0 for input_a_source, 1 for input_b_source.
    pub fn connect(&mut self, source_idx: usize, target_idx: usize, port: u8) {
        self.nodes.connect(source_idx, target_idx, port);
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
            self.nodes.gate_types[gate_idx] == GateType::Input,
            "Cannot set input on a non-Input gate: {:?}",
            self.nodes.gate_types[gate_idx]
        );

        let new_state = if value { 0b10 } else { 0b01 };
        if self.nodes.states[gate_idx] != new_state {
            self.nodes.states[gate_idx] = new_state;

            // Trigger sleep domain state transitions if this gate controls a domain
            self.sync_sleep_domains_for_control_gate(gate_idx, new_state);

            let deps = self.nodes.dependents[gate_idx].clone();
            for dep_u32 in deps {
                self.enqueue(dep_u32 as usize);
            }
        }
    }

    pub fn enqueue_internal(&mut self, gate_idx: usize) {
        self.enqueue(gate_idx);
    }

    fn enqueue(&mut self, gate_idx: usize) {
        // Gates inside sleeping domains are skipped from scheduling
        if gate_idx < self.sleeping_gates.len() && self.sleeping_gates.contains(gate_idx) {
            return;
        }

        if self.nodes.contains(gate_idx) && !self.nodes.in_queue.contains(gate_idx) {
            self.nodes.in_queue.set(gate_idx, true);
            let depth = self.nodes.depths[gate_idx] as usize;
            if self.event_queue.len() <= depth {
                self.event_queue.resize(depth + 1, Vec::new());
            }
            self.event_queue[depth].push(gate_idx);
        }
    }

    fn sync_sleep_domains_for_control_gate(&mut self, ctrl_gate_idx: usize, new_state: u8) {
        if self.sleep_domains.is_empty() {
            return;
        }

        for d_idx in 0..self.sleep_domains.len() {
            if self.sleep_domains[d_idx].control_gate == ctrl_gate_idx {
                let should_sleep = self.sleep_domains[d_idx].should_sleep(new_state);
                if should_sleep && !self.sleep_domains[d_idx].is_sleeping {
                    let mut d = self.sleep_domains[d_idx].clone();
                    d.hibernate(self);
                    self.sleep_domains[d_idx] = d;
                } else if !should_sleep && self.sleep_domains[d_idx].is_sleeping {
                    let mut d = self.sleep_domains[d_idx].clone();
                    d.wake(self);
                    self.sleep_domains[d_idx] = d;
                }
            }
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
                let dependents_len = self.nodes.dependents[v].len();

                let mut current_edge = edge_idx;
                while current_edge < dependents_len {
                    let w = self.nodes.dependents[v][current_edge] as usize;
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
                for &dep_u32 in &self.nodes.dependents[node] {
                    let dep = dep_u32 as usize;
                    if dep < num_nodes {
                        let dep_scc = node_to_scc[dep];
                        if dep_scc != usize::MAX && dep_scc != scc_idx {
                            scc_depth[dep_scc] = scc_depth[dep_scc].max(current_depth + 1);
                        }
                    }
                }
            }
        }

        for (scc_idx, scc) in sccs.iter().enumerate() {
            let depth = scc_depth[scc_idx] as u32;
            for &node in scc {
                self.nodes.depths[node] = depth;
            }
        }

        // Re-bucket all currently queued gates into their newly calculated topological depth queues
        let mut queued = Vec::new();
        for q in &mut self.event_queue {
            queued.append(q);
        }
        for idx in queued {
            self.nodes.in_queue.set(idx, false);
            self.enqueue(idx);
        }
    }

    /// Enqueues all active gates and runs event propagation to settle the circuit into stable rest state
    pub fn settle(&mut self) -> Result<usize, String> {
        for idx in 0..self.nodes.len() {
            if self.nodes.contains(idx) {
                self.enqueue(idx);
            }
        }
        self.propagate_events(200)
    }

    pub fn defragment_and_sort_by_depth(&mut self) -> Vec<usize> {
        let capacity = self.nodes.capacity();

        let is_already_compact_and_sorted =
            self.nodes.len() == capacity && self.nodes.depths.windows(2).all(|w| w[0] <= w[1]);

        if is_already_compact_and_sorted {
            return (0..capacity).collect();
        }

        let mut old_to_new = vec![usize::MAX; capacity];
        let mut valid_nodes: Vec<usize> = (0..capacity)
            .filter(|&idx| self.nodes.contains(idx))
            .collect();

        valid_nodes.sort_by_key(|&idx| self.nodes.depths[idx]);

        let mut new_storage = SoAGateStorage::with_capacity(valid_nodes.len());
        for &old_idx in &valid_nodes {
            let new_idx =
                new_storage.insert(self.nodes.gate_types[old_idx], self.nodes.states[old_idx]);
            new_storage.depths[new_idx] = self.nodes.depths[old_idx];
            old_to_new[old_idx] = new_idx;
        }

        for (new_idx, &old_idx) in valid_nodes.iter().enumerate() {
            let src_a = self.nodes.sources[old_idx][0];
            let src_b = self.nodes.sources[old_idx][1];

            if src_a != NO_SOURCE && (src_a as usize) < old_to_new.len() {
                let remapped = old_to_new[src_a as usize];
                if remapped != usize::MAX {
                    new_storage.sources[new_idx][0] = remapped as u32;
                }
            }
            if src_b != NO_SOURCE && (src_b as usize) < old_to_new.len() {
                let remapped = old_to_new[src_b as usize];
                if remapped != usize::MAX {
                    new_storage.sources[new_idx][1] = remapped as u32;
                }
            }

            for &dep_u32 in &self.nodes.dependents[old_idx] {
                let dep = dep_u32 as usize;
                if dep < old_to_new.len() {
                    let remapped = old_to_new[dep];
                    if remapped != usize::MAX {
                        new_storage.dependents[new_idx].push(remapped as u32);
                    }
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

        // Remap sleep domains
        for domain in &mut self.sleep_domains {
            if domain.control_gate < old_to_new.len()
                && old_to_new[domain.control_gate] != usize::MAX
            {
                domain.control_gate = old_to_new[domain.control_gate];
            }
            for g in &mut domain.gates {
                if *g < old_to_new.len() && old_to_new[*g] != usize::MAX {
                    *g = old_to_new[*g];
                }
            }
            for latch in &mut domain.latch_indices {
                if *latch < old_to_new.len() && old_to_new[*latch] != usize::MAX {
                    *latch = old_to_new[*latch];
                }
            }
        }

        self.nodes = new_storage;
        old_to_new
    }

    pub fn propagate_events(&mut self, budget_multiplier: usize) -> Result<usize, String> {
        let mut total_steps = 0;
        let max_steps = self.nodes.capacity().max(1) * budget_multiplier.max(100);

        // Synchronize all sleep domains with current control values
        if !self.sleep_domains.is_empty() {
            for d_idx in 0..self.sleep_domains.len() {
                let ctrl_gate = self.sleep_domains[d_idx].control_gate;
                if ctrl_gate < self.nodes.states.len() {
                    let ctrl_val = self.nodes.states[ctrl_gate];
                    let should_sleep = self.sleep_domains[d_idx].should_sleep(ctrl_val);
                    if should_sleep && !self.sleep_domains[d_idx].is_sleeping {
                        let mut d = self.sleep_domains[d_idx].clone();
                        d.hibernate(self);
                        self.sleep_domains[d_idx] = d;
                    } else if !should_sleep && self.sleep_domains[d_idx].is_sleeping {
                        let mut d = self.sleep_domains[d_idx].clone();
                        d.wake(self);
                        self.sleep_domains[d_idx] = d;
                    }
                }
            }
        }

        let mut depth = 0;
        while depth < self.event_queue.len() {
            if self.event_queue[depth].is_empty() {
                depth += 1;
                continue;
            }

            let current_queue = std::mem::take(&mut self.event_queue[depth]);
            for &idx in &current_queue {
                self.nodes.in_queue.set(idx, false);
            }

            let states = &self.nodes.states;
            let gate_types = &self.nodes.gate_types;
            let sources = &self.nodes.sources;
            let allocated = &self.nodes.allocated;

            let compute_state = |&idx: &usize| -> Option<(usize, u8)> {
                if idx >= states.len() || !allocated.contains(idx) {
                    return None;
                }

                let src_a = sources[idx][0];
                let val_a = if src_a != NO_SOURCE && (src_a as usize) < states.len() {
                    states[src_a as usize]
                } else {
                    0b00
                };

                let src_b = sources[idx][1];
                let val_b = if src_b != NO_SOURCE && (src_b as usize) < states.len() {
                    states[src_b as usize]
                } else {
                    0b00
                };

                let curr_state = states[idx];
                let new_state = match gate_types[idx] {
                    GateType::Input => curr_state,
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

                if new_state != curr_state {
                    Some((idx, new_state))
                } else {
                    None
                }
            };

            let updates: Vec<(usize, u8)> = if current_queue.len() >= self.dynamic_threshold {
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
                self.nodes.states[idx] = new_state;
                next_enqueues.push(idx);

                // Update sleep domains if a control gate state changed
                self.sync_sleep_domains_for_control_gate(idx, new_state);
            }

            total_steps += current_queue.len();
            if total_steps >= max_steps {
                return Err(format!(
                    "Oscillation detected: exceeded max_steps limit of {}",
                    max_steps
                ));
            }

            for idx in next_enqueues {
                let deps = self.nodes.dependents[idx].clone();
                for dep_u32 in deps {
                    self.enqueue(dep_u32 as usize);
                }
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
        if self.nodes.contains(gate_idx) {
            self.nodes.states[gate_idx]
        } else {
            0b00
        }
    }

    pub fn get_state(&self, gate_idx: usize) -> bool {
        if self.nodes.contains(gate_idx) {
            self.nodes.states[gate_idx] == 0b10
        } else {
            false
        }
    }
}
