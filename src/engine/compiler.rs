use super::simulator::Simulator;
use super::sleep::SleepDomain;
use super::types::*;
use std::collections::{HashMap, HashSet};

pub fn find_gating_input(blueprint: &ChipBlueprint, library: &[ChipBlueprint]) -> Option<usize> {
    // Stage 1: Explicit Pin Name (Fast-Path)
    if let Some(pos) = blueprint.input_names.iter().position(|name| {
        let lower = name.trim().to_ascii_lowercase();
        lower == "cs"
            || lower == "chip_select"
            || lower == "chipselect"
            || lower == "ce"
            || lower == "enable"
            || lower == "en"
            || lower == "we"
            || lower == "write_enable"
    }) {
        return Some(pos);
    }

    if blueprint.inputs == 0 {
        return None;
    }

    // Stage 2: Topological Graph Analysis

    // Case A: Hierarchical Subchip Gating Analysis
    // Count how many subchip gating ports are driven by each ChipInput
    let mut subchip_gating_fanout = vec![0usize; blueprint.inputs];
    let mut has_subchips_with_gating = false;

    for conn in &blueprint.connections {
        if let (
            SourcePort::ChipInput(chip_in),
            TargetPort::ComponentInput {
                component_idx,
                port_idx,
            },
        ) = (conn.source, conn.target)
            && chip_in < blueprint.inputs
            && component_idx < blueprint.components.len()
            && let ComponentType::SubChip(sub_idx) =
                blueprint.components[component_idx].component_type
            && let Some(sub_bp) = library.get(sub_idx)
            && let Some(sub_gating_port) = find_gating_input(sub_bp, library)
        {
            has_subchips_with_gating = true;
            if port_idx == sub_gating_port {
                subchip_gating_fanout[chip_in] += 1;
            }
        }
    }

    if has_subchips_with_gating
        && let Some((best_idx, &max_fanout)) = subchip_gating_fanout
            .iter()
            .enumerate()
            .max_by_key(|entry| entry.1)
        && max_fanout > 0
    {
        return Some(best_idx);
    }

    // Case B: Primitive Feedback Cycle (SCC) Analysis for Raw Gate Latches
    let num_components = blueprint.components.len();
    if num_components == 0 {
        return None;
    }

    // Build adjacency list between components
    let mut adj = vec![Vec::new(); num_components];
    for conn in &blueprint.connections {
        if let (
            SourcePort::ComponentOutput {
                component_idx: src, ..
            },
            TargetPort::ComponentInput {
                component_idx: tgt, ..
            },
        ) = (conn.source, conn.target)
            && src < num_components
            && tgt < num_components
        {
            adj[src].push(tgt);
        }
    }

    struct TarjanState {
        indices: Vec<usize>,
        lowlink: Vec<usize>,
        on_stack: Vec<bool>,
        stack: Vec<usize>,
        index: usize,
        sccs: Vec<Vec<usize>>,
    }

    impl TarjanState {
        fn strongconnect(&mut self, v: usize, adj: &[Vec<usize>]) {
            self.indices[v] = self.index;
            self.lowlink[v] = self.index;
            self.index += 1;
            self.stack.push(v);
            self.on_stack[v] = true;

            for &w in &adj[v] {
                if self.indices[w] == usize::MAX {
                    self.strongconnect(w, adj);
                    self.lowlink[v] = self.lowlink[v].min(self.lowlink[w]);
                } else if self.on_stack[w] {
                    self.lowlink[v] = self.lowlink[v].min(self.indices[w]);
                }
            }

            if self.lowlink[v] == self.indices[v] {
                let mut scc = Vec::new();
                while let Some(w) = self.stack.pop() {
                    self.on_stack[w] = false;
                    scc.push(w);
                    if w == v {
                        break;
                    }
                }
                self.sccs.push(scc);
            }
        }
    }

    let mut state = TarjanState {
        indices: vec![usize::MAX; num_components],
        lowlink: vec![usize::MAX; num_components],
        on_stack: vec![false; num_components],
        stack: Vec::new(),
        index: 0,
        sccs: Vec::new(),
    };

    for v in 0..num_components {
        if state.indices[v] == usize::MAX {
            state.strongconnect(v, &adj);
        }
    }

    // Filter SCCs that represent feedback loops (size >= 2 or self-loops)
    let mut latch_core_nodes = HashSet::new();
    for scc in state.sccs {
        if scc.len() >= 2 {
            latch_core_nodes.extend(scc);
        } else if scc.len() == 1 {
            let node = scc[0];
            if adj[node].contains(&node) {
                latch_core_nodes.insert(node);
            }
        }
    }

    if latch_core_nodes.is_empty() {
        return None; // Pure combinational circuit (DAG) -> Zero false positives!
    }

    // Identify gating components: components OUTSIDE the feedback core that drive nodes inside the core
    let mut gating_components = HashSet::new();
    for conn in &blueprint.connections {
        if let (
            SourcePort::ComponentOutput {
                component_idx: src, ..
            },
            TargetPort::ComponentInput {
                component_idx: tgt, ..
            },
        ) = (conn.source, conn.target)
            && !latch_core_nodes.contains(&src)
            && latch_core_nodes.contains(&tgt)
        {
            gating_components.insert(src);
        }
    }

    // Score each ChipInput by its fan-out to gating_components (or directly into latch_core_nodes)
    let mut latch_input_fanout = vec![0usize; blueprint.inputs];
    for conn in &blueprint.connections {
        if let (
            SourcePort::ChipInput(chip_in),
            TargetPort::ComponentInput {
                component_idx: tgt, ..
            },
        ) = (conn.source, conn.target)
            && chip_in < blueprint.inputs
            && (gating_components.contains(&tgt) || latch_core_nodes.contains(&tgt))
        {
            latch_input_fanout[chip_in] += 1;
        }
    }

    // The enable pin in a latch must control multiple gating arms (fanout >= 2),
    // unlike data inputs which only connect to a single arm (fanout = 1).
    if let Some((best_idx, &max_fanout)) = latch_input_fanout
        .iter()
        .enumerate()
        .max_by_key(|entry| entry.1)
        && max_fanout >= 2
    {
        return Some(best_idx);
    }

    None
}

struct CompilerContext<'a> {
    blueprint: &'a ChipBlueprint,
    component_ports: &'a [(Vec<Vec<(usize, u8)>>, Vec<OutputSource>)],
    connections_map: HashMap<TargetPort, Vec<SourcePort>>,
}

impl<'a> CompilerContext<'a> {
    fn trace_drivers(
        &self,
        start_node: TraceNode,
        sim: &mut Simulator,
        resolver_cache: &mut HashMap<Vec<usize>, usize>,
        node_cache: &mut HashMap<TraceNode, OutputSource>,
    ) -> OutputSource {
        if let Some(&cached) = node_cache.get(&start_node) {
            return cached;
        }

        let mut visited = HashSet::new();
        let mut queue = vec![start_node];
        let mut gate_drivers = Vec::new();
        let mut chip_input_drivers = Vec::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current) {
                continue;
            }

            match current {
                TraceNode::ChipInput(idx) => {
                    if !chip_input_drivers.contains(&idx) {
                        chip_input_drivers.push(idx);
                    }
                }
                TraceNode::ChipOutput(out_idx) => {
                    let target_port = TargetPort::ChipOutput(out_idx);
                    if let Some(srcs) = self.connections_map.get(&target_port) {
                        for src in srcs {
                            match src {
                                SourcePort::ChipInput(i) => queue.push(TraceNode::ChipInput(*i)),
                                SourcePort::ComponentOutput {
                                    component_idx,
                                    port_idx,
                                } => {
                                    queue.push(TraceNode::CompOutput {
                                        component_idx: *component_idx,
                                        port_idx: *port_idx,
                                    });
                                }
                            }
                        }
                    }
                }
                TraceNode::CompInput {
                    component_idx,
                    port_idx,
                } => {
                    let target_port = TargetPort::ComponentInput {
                        component_idx,
                        port_idx,
                    };
                    if let Some(srcs) = self.connections_map.get(&target_port) {
                        for src in srcs {
                            match src {
                                SourcePort::ChipInput(i) => queue.push(TraceNode::ChipInput(*i)),
                                SourcePort::ComponentOutput {
                                    component_idx,
                                    port_idx,
                                } => {
                                    queue.push(TraceNode::CompOutput {
                                        component_idx: *component_idx,
                                        port_idx: *port_idx,
                                    });
                                }
                            }
                        }
                    }
                }
                TraceNode::CompOutput {
                    component_idx,
                    port_idx,
                } => {
                    let component = &self.blueprint.components[component_idx];
                    match &component.component_type {
                        ComponentType::SevenSegment => {}
                        ComponentType::Nand
                        | ComponentType::Clock
                        | ComponentType::TriStateBuffer => {
                            let (_, ref outputs) = self.component_ports[component_idx];
                            if let Some(OutputSource::DrivenByGate(g_idx)) = outputs.first()
                                && !gate_drivers.contains(g_idx)
                            {
                                gate_drivers.push(*g_idx);
                            }
                        }
                        ComponentType::SubChip(_)
                        | ComponentType::Junction
                        | ComponentType::BusJoiner
                        | ComponentType::BusSplitter => {
                            let (_, ref outputs) = self.component_ports[component_idx];
                            if port_idx < outputs.len() {
                                match outputs[port_idx] {
                                    OutputSource::DrivenByGate(g_idx) => {
                                        if !gate_drivers.contains(&g_idx) {
                                            gate_drivers.push(g_idx);
                                        }
                                    }
                                    OutputSource::Floating => {}
                                    OutputSource::PassedThrough(in_idx) => {
                                        queue.push(TraceNode::CompInput {
                                            component_idx,
                                            port_idx: in_idx,
                                        });
                                    }
                                }
                            }
                        }
                        ComponentType::Input | ComponentType::Output => {}
                    }
                }
            }
        }

        gate_drivers.sort();
        let resolved_gate = if gate_drivers.is_empty() {
            None
        } else if gate_drivers.len() == 1 {
            Some(gate_drivers[0])
        } else if let Some(&cached) = resolver_cache.get(&gate_drivers) {
            Some(cached)
        } else {
            let mut current_idx = gate_drivers[0];
            for &driver in gate_drivers.iter().skip(1) {
                let resolver = sim.add_gate(GateType::BusResolver);
                sim.connect(current_idx, resolver, 0);
                sim.connect(driver, resolver, 1);
                current_idx = resolver;
            }
            resolver_cache.insert(gate_drivers, current_idx);
            Some(current_idx)
        };

        let result = if let Some(g_idx) = resolved_gate {
            OutputSource::DrivenByGate(g_idx)
        } else if let Some(&in_idx) = chip_input_drivers.first() {
            OutputSource::PassedThrough(in_idx)
        } else {
            OutputSource::Floating
        };

        node_cache.insert(start_node, result);
        result
    }
}

impl Simulator {
    pub fn instantiate_chip_with_mapping(
        &mut self,
        blueprint_idx: usize,
        library: &[ChipBlueprint],
        active_clocks: &mut Vec<CompiledClock>,
        blueprint_stack: &mut Vec<usize>,
    ) -> Result<(InstantiatedInterface, InstanceTree), String> {
        if blueprint_stack.contains(&blueprint_idx) {
            return Err("Recursion cycle detected in custom chip blueprints".to_string());
        }
        blueprint_stack.push(blueprint_idx);

        let blueprint = library
            .get(blueprint_idx)
            .ok_or_else(|| format!("Blueprint not found at index {}", blueprint_idx))?;

        // Validate all connections to prevent out-of-bounds panics
        for conn in &blueprint.connections {
            // Check source
            match conn.source {
                SourcePort::ComponentOutput {
                    component_idx,
                    port_idx,
                } => {
                    if component_idx >= blueprint.components.len() {
                        return Err(format!("Invalid source component index {}", component_idx));
                    }
                    let comp = &blueprint.components[component_idx];
                    let (_, out_ports) =
                        comp.component_type.get_port_counts(comp.bus_width, library);
                    if port_idx >= out_ports {
                        return Err(format!("Invalid source port index {}", port_idx));
                    }
                }
                SourcePort::ChipInput(idx) => {
                    if idx >= blueprint.inputs {
                        return Err(format!("Invalid chip input index {}", idx));
                    }
                }
            }

            // Check target
            match conn.target {
                TargetPort::ComponentInput {
                    component_idx,
                    port_idx,
                } => {
                    if component_idx >= blueprint.components.len() {
                        return Err(format!("Invalid target component index {}", component_idx));
                    }
                    let comp = &blueprint.components[component_idx];
                    let (in_ports, _) =
                        comp.component_type.get_port_counts(comp.bus_width, library);
                    if port_idx >= in_ports {
                        return Err(format!("Invalid target port index {}", port_idx));
                    }
                }
                TargetPort::ChipOutput(idx) => {
                    if idx >= blueprint.outputs {
                        return Err(format!("Invalid chip output index {}", idx));
                    }
                }
            }
        }

        let mut component_ports = Vec::new();
        let mut tree = InstanceTree::default();
        let mut subchip_sleep_info = Vec::new();

        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            let mut sub_node = InstanceTree::default();
            match &component.component_type {
                ComponentType::Nand => {
                    let nand_idx = self.add_gate(GateType::Nand);
                    sub_node.gate_idx = Some(nand_idx);
                    sub_node.outputs = vec![OutputSource::DrivenByGate(nand_idx)];
                    component_ports.push((
                        vec![vec![(nand_idx, 0)], vec![(nand_idx, 1)]],
                        vec![OutputSource::DrivenByGate(nand_idx)],
                    ));
                }
                ComponentType::TriStateBuffer => {
                    let sim_idx = self.add_gate(GateType::TriStateBuffer);
                    sub_node.gate_idx = Some(sim_idx);
                    sub_node.outputs = vec![OutputSource::DrivenByGate(sim_idx)];
                    component_ports.push((
                        vec![vec![(sim_idx, 0)], vec![(sim_idx, 1)]],
                        vec![OutputSource::DrivenByGate(sim_idx)],
                    ));
                }
                ComponentType::Junction => {
                    sub_node.outputs = vec![OutputSource::PassedThrough(0)];
                    component_ports.push((vec![vec![]], vec![OutputSource::PassedThrough(0)]));
                }
                ComponentType::BusJoiner | ComponentType::BusSplitter => {
                    let w = component.bus_width();
                    let outputs: Vec<OutputSource> =
                        (0..w).map(OutputSource::PassedThrough).collect();
                    sub_node.outputs = outputs.clone();
                    component_ports.push((vec![vec![]; w], outputs));
                }
                ComponentType::Clock => {
                    let clock_idx = self.add_gate(GateType::Input);
                    sub_node.gate_idx = Some(clock_idx);
                    sub_node.outputs = vec![OutputSource::DrivenByGate(clock_idx)];

                    let period = component.clock_period.unwrap_or(20);
                    active_clocks.push(CompiledClock {
                        gate_idx: clock_idx,
                        period,
                        counter: 0,
                        visual_id: None,
                    });

                    component_ports.push((vec![], vec![OutputSource::DrivenByGate(clock_idx)]));
                }
                ComponentType::SevenSegment => {
                    let mut inputs = Vec::new();
                    for _ in 0..8 {
                        let sim_idx = self.add_gate(GateType::Output);
                        inputs.push(vec![(sim_idx, 0)]);
                    }
                    component_ports.push((inputs, vec![]));
                }
                ComponentType::SubChip(sub_idx) => {
                    let start_gate_idx = self.nodes.len();
                    let (sub_interface, sub_tree) = self.instantiate_chip_with_mapping(
                        *sub_idx,
                        library,
                        active_clocks,
                        blueprint_stack,
                    )?;
                    let end_gate_idx = self.nodes.len();

                    if let Some(sub_bp) = library.get(*sub_idx)
                        && let Some(gating_port) = find_gating_input(sub_bp, library)
                    {
                        let latches: Vec<usize> = (start_gate_idx..end_gate_idx)
                            .filter(|&g| {
                                g < self.nodes.gate_types.len()
                                    && self.nodes.gate_types[g] == GateType::Nand
                            })
                            .collect();
                        if !latches.is_empty() {
                            subchip_sleep_info.push((
                                comp_idx,
                                gating_port,
                                (start_gate_idx..end_gate_idx).collect::<Vec<_>>(),
                                latches,
                            ));
                        }
                    }

                    sub_node = sub_tree;
                    component_ports.push((sub_interface.inputs, sub_interface.outputs));
                }
                ComponentType::Input | ComponentType::Output => {
                    return Err(
                        "Blueprint components cannot contain top-level Input or Output internally"
                            .to_string(),
                    );
                }
            }
            tree.sub_instances.insert(comp_idx, sub_node);
        }

        let mut connections_map: HashMap<TargetPort, Vec<SourcePort>> = HashMap::new();
        for conn in &blueprint.connections {
            let is_bus = match (conn.source, conn.target) {
                (
                    SourcePort::ComponentOutput {
                        component_idx: src_idx,
                        port_idx: src_port,
                    },
                    TargetPort::ComponentInput {
                        component_idx: tgt_idx,
                        port_idx: tgt_port,
                    },
                ) => {
                    let src_comp = &blueprint.components[src_idx];
                    let tgt_comp = &blueprint.components[tgt_idx];
                    src_comp.component_type == ComponentType::BusJoiner
                        && src_port == 0
                        && tgt_comp.component_type == ComponentType::BusSplitter
                        && tgt_port == 0
                }
                _ => false,
            };

            if is_bus {
                let (src_idx, tgt_idx) = match (conn.source, conn.target) {
                    (
                        SourcePort::ComponentOutput {
                            component_idx: src_idx,
                            ..
                        },
                        TargetPort::ComponentInput {
                            component_idx: tgt_idx,
                            ..
                        },
                    ) => (src_idx, tgt_idx),
                    _ => unreachable!(),
                };
                let w = blueprint.components[src_idx].bus_width();
                for i in 0..w {
                    connections_map
                        .entry(TargetPort::ComponentInput {
                            component_idx: tgt_idx,
                            port_idx: i,
                        })
                        .or_default()
                        .push(SourcePort::ComponentOutput {
                            component_idx: src_idx,
                            port_idx: i,
                        });
                }
            } else {
                connections_map
                    .entry(conn.target)
                    .or_default()
                    .push(conn.source);
            }
        }

        let context = CompilerContext {
            blueprint,
            component_ports: &component_ports,
            connections_map,
        };

        let mut resolver_cache: HashMap<Vec<usize>, usize> = HashMap::new();
        let mut node_cache: HashMap<TraceNode, OutputSource> = HashMap::new();

        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            let (input_ports_count, _) = component
                .component_type
                .get_port_counts(component.bus_width, library);

            for port_idx in 0..input_ports_count {
                let start_node = TraceNode::CompInput {
                    component_idx: comp_idx,
                    port_idx,
                };
                let driver =
                    context.trace_drivers(start_node, self, &mut resolver_cache, &mut node_cache);

                if let OutputSource::DrivenByGate(src_g_idx) = driver {
                    let targets = &component_ports[comp_idx].0[port_idx];
                    for &(tgt_g_idx, tgt_port) in targets {
                        self.connect(src_g_idx, tgt_g_idx, tgt_port);
                    }

                    for &(s_comp_idx, s_gating_port, ref gates, ref latches) in &subchip_sleep_info
                    {
                        if s_comp_idx == comp_idx && s_gating_port == port_idx {
                            let domain_id = self.sleep_domains.len();
                            let domain = SleepDomain::new(
                                domain_id,
                                src_g_idx,
                                true,
                                gates.clone(),
                                latches.clone(),
                            );
                            self.register_sleep_domain(domain);
                        }
                    }
                }
            }
        }

        let mut inputs = vec![Vec::new(); blueprint.inputs];
        for i in 0..blueprint.inputs {
            for (comp_idx, component) in blueprint.components.iter().enumerate() {
                let (input_ports_count, _) = component
                    .component_type
                    .get_port_counts(component.bus_width, library);

                for port_idx in 0..input_ports_count {
                    let comp_in_node = TraceNode::CompInput {
                        component_idx: comp_idx,
                        port_idx,
                    };
                    let driver = context.trace_drivers(
                        comp_in_node,
                        self,
                        &mut resolver_cache,
                        &mut node_cache,
                    );
                    if driver == OutputSource::PassedThrough(i) {
                        let targets = &component_ports[comp_idx].0[port_idx];
                        inputs[i].extend(targets.iter().copied());
                    }
                }
            }
        }

        let mut outputs = Vec::new();
        for j in 0..blueprint.outputs {
            let start_node = TraceNode::ChipOutput(j);
            let driver =
                context.trace_drivers(start_node, self, &mut resolver_cache, &mut node_cache);
            outputs.push(driver);
        }

        tree.outputs = outputs.clone();

        blueprint_stack.pop();
        Ok((InstantiatedInterface { inputs, outputs }, tree))
    }

    pub fn instantiate_chip(
        &mut self,
        blueprint_idx: usize,
        library: &[ChipBlueprint],
    ) -> Result<InstantiatedInterface, String> {
        let mut dummy_clocks = Vec::new();
        let mut dummy_stack = Vec::new();
        self.instantiate_chip_with_mapping(
            blueprint_idx,
            library,
            &mut dummy_clocks,
            &mut dummy_stack,
        )
        .map(|(interface, _)| interface)
    }
}

#[derive(Debug, Clone)]
pub struct SubchipTemplate {
    pub gate_types: Vec<GateType>,
    pub internal_connections: Vec<(usize, usize, u8)>, // (local_src, local_tgt, port)
    pub interface: InstantiatedInterface,
    pub instance_tree: InstanceTree,
    pub relative_clocks: Vec<(usize, usize)>, // (local_gate_idx, period)
    pub sleep_domains: Vec<(usize, Vec<usize>, Vec<usize>)>, // (local_ctrl, local_gates, local_latches)
}

impl SubchipTemplate {
    pub fn instantiate_into(
        &self,
        sim: &mut Simulator,
        active_clocks: &mut Vec<CompiledClock>,
    ) -> (InstantiatedInterface, InstanceTree) {
        let base_idx = sim.nodes.len();

        // 1. Allocate all gates
        for &gt in &self.gate_types {
            sim.add_gate(gt);
        }

        // 2. Wire up internal connections
        for &(src, tgt, port) in &self.internal_connections {
            sim.connect(base_idx + src, base_idx + tgt, port);
        }

        // 3. Offset clocks
        for &(local_gate, period) in &self.relative_clocks {
            active_clocks.push(CompiledClock {
                gate_idx: base_idx + local_gate,
                period,
                counter: 0,
                visual_id: None,
            });
        }

        // 4. Offset sleep domains
        for (local_ctrl, local_gates, local_latches) in &self.sleep_domains {
            let domain_id = sim.sleep_domains.len();
            let remapped_gates = local_gates.iter().map(|&g| base_idx + g).collect();
            let remapped_latches = local_latches.iter().map(|&l| base_idx + l).collect();
            let domain = SleepDomain::new(
                domain_id,
                base_idx + local_ctrl,
                true,
                remapped_gates,
                remapped_latches,
            );
            sim.register_sleep_domain(domain);
        }

        // 5. Offset interface
        let mut interface = self.interface.clone();
        for targets in &mut interface.inputs {
            for (tgt_gate, _) in targets {
                *tgt_gate += base_idx;
            }
        }
        for out in &mut interface.outputs {
            if let OutputSource::DrivenByGate(g) = out {
                *g += base_idx;
            }
        }

        // 6. Offset instance tree
        let mut tree = self.instance_tree.clone();
        tree.apply_offset(base_idx);

        (interface, tree)
    }
}

pub fn compile_subchip_template(
    blueprint_idx: usize,
    library: &[ChipBlueprint],
) -> Result<SubchipTemplate, String> {
    let mut scratch_sim = Simulator::new();
    let mut active_clocks = Vec::new();
    let mut blueprint_stack = Vec::new();

    let (interface, instance_tree) = scratch_sim.instantiate_chip_with_mapping(
        blueprint_idx,
        library,
        &mut active_clocks,
        &mut blueprint_stack,
    )?;

    let num_gates = scratch_sim.nodes.len();
    let gate_types = scratch_sim.nodes.gate_types[..num_gates].to_vec();

    let mut internal_connections = Vec::new();
    for tgt in 0..num_gates {
        let src_a = scratch_sim.nodes.sources[tgt][0];
        let src_b = scratch_sim.nodes.sources[tgt][1];
        if src_a != crate::engine::storage::NO_SOURCE {
            internal_connections.push((src_a as usize, tgt, 0));
        }
        if src_b != crate::engine::storage::NO_SOURCE {
            internal_connections.push((src_b as usize, tgt, 1));
        }
    }

    let relative_clocks = active_clocks
        .iter()
        .map(|c| (c.gate_idx, c.period))
        .collect();

    let sleep_domains = scratch_sim
        .sleep_domains
        .iter()
        .map(|d| (d.control_gate, d.gates.clone(), d.latch_indices.clone()))
        .collect();

    Ok(SubchipTemplate {
        gate_types,
        internal_connections,
        interface,
        instance_tree,
        relative_clocks,
        sleep_domains,
    })
}
