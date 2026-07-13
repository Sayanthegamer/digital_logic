use super::simulator::Simulator;
use super::types::*;
use std::collections::{HashMap, HashSet};

struct CompilerContext<'a> {
    blueprint: &'a ChipBlueprint,
    component_ports: &'a [(Vec<Vec<(usize, u8)>>, Vec<OutputSource>)],
    connections_map: HashMap<TargetPort, SourcePort>,
}

impl<'a> CompilerContext<'a> {
    fn get_immediate_source(&self, node: &TraceNode) -> Option<TraceNode> {
        let target_port = match node {
            TraceNode::ChipOutput(out_idx) => TargetPort::ChipOutput(*out_idx),
            TraceNode::CompInput { component_idx, port_idx } => TargetPort::ComponentInput {
                component_idx: *component_idx,
                port_idx: *port_idx,
            },
            TraceNode::CompOutput { component_idx, port_idx } => {
                let component = &self.blueprint.components[*component_idx];
                match &component.component_type {
                    ComponentType::SevenSegment
                    | ComponentType::Nand
                    | ComponentType::TriStateBuffer
                    | ComponentType::Input
                    | ComponentType::Output
                    | ComponentType::Clock => return None,
                    ComponentType::SubChip(_)
                    | ComponentType::Junction
                    | ComponentType::BusJoiner
                    | ComponentType::BusSplitter => {
                        let (_, ref outputs) = self.component_ports[*component_idx];
                        match outputs[*port_idx] {
                            OutputSource::PassedThrough(in_idx) => return Some(TraceNode::CompInput {
                                component_idx: *component_idx,
                                port_idx: in_idx,
                            }),
                            _ => return None,
                        }
                    }
                }
            }
            TraceNode::ChipInput(_) => return None,
        };

        self.connections_map.get(&target_port).map(|src| match src {
            SourcePort::ChipInput(i) => TraceNode::ChipInput(*i),
            SourcePort::ComponentOutput { component_idx, port_idx } => TraceNode::CompOutput {
                component_idx: *component_idx,
                port_idx: *port_idx,
            },
        })
    }

    fn trace_root(
        &self,
        start_node: TraceNode,
        trace_cache: &mut HashMap<TraceNode, OutputSource>,
    ) -> OutputSource {
        if let Some(&cached) = trace_cache.get(&start_node) {
            return cached;
        }

        let mut current = start_node;
        let mut visited = HashSet::new();
        let mut path_nodes = Vec::new();

        let result = loop {
            if let Some(&cached) = trace_cache.get(&current) {
                break cached;
            }
            if !visited.insert(current) {
                break OutputSource::Floating;
            }
            path_nodes.push(current);

            match current {
                TraceNode::ChipInput(idx) => {
                    break OutputSource::PassedThrough(idx);
                }
                TraceNode::CompOutput { component_idx, port_idx } => {
                    let component = &self.blueprint.components[component_idx];
                    match &component.component_type {
                        ComponentType::SevenSegment => break OutputSource::Floating,
                        ComponentType::Nand | ComponentType::Clock | ComponentType::TriStateBuffer => {
                            let (_, ref outputs) = self.component_ports[component_idx];
                            match outputs.first() {
                                Some(OutputSource::DrivenByGate(g_idx)) => break OutputSource::DrivenByGate(*g_idx),
                                _ => break OutputSource::Floating,
                            }
                        }
                        ComponentType::SubChip(_)
                        | ComponentType::Junction
                        | ComponentType::BusJoiner
                        | ComponentType::BusSplitter => {
                            let (_, ref outputs) = self.component_ports[component_idx];
                            match outputs[port_idx] {
                                OutputSource::DrivenByGate(g_idx) => break OutputSource::DrivenByGate(g_idx),
                                OutputSource::Floating => break OutputSource::Floating,
                                OutputSource::PassedThrough(in_idx) => {
                                    current = TraceNode::CompInput {
                                        component_idx,
                                        port_idx: in_idx,
                                    };
                                }
                            }
                        }
                        ComponentType::Input | ComponentType::Output => break OutputSource::Floating,
                    }
                }
                _ => {
                    if let Some(next_node) = self.get_immediate_source(&current) {
                        current = next_node;
                    } else {
                        break OutputSource::Floating;
                    }
                }
            }
        };

        for node in path_nodes {
            trace_cache.insert(node, result);
        }

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
                    let outputs: Vec<OutputSource> = (0..w).map(|i| OutputSource::PassedThrough(i)).collect();
                    sub_node.outputs = outputs.clone();
                    component_ports.push((
                        vec![vec![]; w],
                        outputs,
                    ));
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

                    component_ports.push((
                        vec![],
                        vec![OutputSource::DrivenByGate(clock_idx)],
                    ));
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

        let mut connections_map = HashMap::new();
        for conn in &blueprint.connections {
            let is_bus = match (conn.source, conn.target) {
                (
                    SourcePort::ComponentOutput { component_idx: src_idx, port_idx: src_port },
                    TargetPort::ComponentInput { component_idx: tgt_idx, port_idx: tgt_port }
                ) => {
                    let src_comp = &blueprint.components[src_idx];
                    let tgt_comp = &blueprint.components[tgt_idx];
                    src_comp.component_type == ComponentType::BusJoiner && src_port == 0
                    && tgt_comp.component_type == ComponentType::BusSplitter && tgt_port == 0
                }
                _ => false,
            };

            if is_bus {
                let (src_idx, tgt_idx) = match (conn.source, conn.target) {
                    (SourcePort::ComponentOutput { component_idx: src_idx, .. }, TargetPort::ComponentInput { component_idx: tgt_idx, .. }) => (src_idx, tgt_idx),
                    _ => unreachable!(),
                };
                let w = blueprint.components[src_idx].bus_width();
                for i in 0..w {
                    connections_map.insert(
                        TargetPort::ComponentInput { component_idx: tgt_idx, port_idx: i },
                        SourcePort::ComponentOutput { component_idx: src_idx, port_idx: i },
                    );
                }
            } else {
                connections_map.insert(conn.target, conn.source);
            }
        }

        let context = CompilerContext {
            blueprint,
            component_ports: &component_ports,
            connections_map,
        };

        let mut trace_cache: HashMap<TraceNode, OutputSource> = HashMap::new();

        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            let (input_ports_count, _) = component.component_type.get_port_counts(component.bus_width, library);

            for port_idx in 0..input_ports_count {
                let start_node = TraceNode::CompInput {
                    component_idx: comp_idx,
                    port_idx,
                };
                let driver = context.trace_root(start_node, &mut trace_cache);

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
                let (input_ports_count, _) = component.component_type.get_port_counts(component.bus_width, library);

                for port_idx in 0..input_ports_count {
                    let comp_in_node = TraceNode::CompInput {
                        component_idx: comp_idx,
                        port_idx,
                    };
                    let driver = context.trace_root(comp_in_node, &mut trace_cache);
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
            let driver = context.trace_root(start_node, &mut trace_cache);
            outputs.push(driver);
        }

        tree.outputs = outputs.clone();

        blueprint_stack.pop();
        Ok((
            InstantiatedInterface { inputs, outputs },
            tree
        ))
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
        ).map(|(interface, _)| interface)
    }
}
