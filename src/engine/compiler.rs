use super::simulator::Simulator;
use super::types::*;
use std::collections::{HashMap, HashSet};

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

        let mut component_ports = Vec::new();
        let mut tree = InstanceTree::default();

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
                    let (sub_interface, sub_tree) = self.instantiate_chip_with_mapping(
                        *sub_idx,
                        library,
                        active_clocks,
                        blueprint_stack,
                    )?;
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
