use super::types::*;
use super::simulator::Simulator;

impl Simulator {
    /// Instantiates a custom chip from the blueprint library and records mapping of instances to compiled gate indices.
    pub fn instantiate_chip_with_mapping(
        &mut self,
        blueprint_idx: usize,
        library: &[ChipBlueprint],
        path: &[usize],
        instance_map: &mut std::collections::HashMap<(Vec<usize>, usize), usize>,
        instance_outputs: &mut std::collections::HashMap<(Vec<usize>, usize), Vec<OutputSource>>,
        active_clocks: &mut Vec<CompiledClock>,
    ) -> Result<InstantiatedInterface, String> {
        let blueprint = library.get(blueprint_idx)
            .ok_or_else(|| format!("Blueprint not found at index {}", blueprint_idx))?;

        // 1. Instantiate all internal components
        let mut component_ports: Vec<(Vec<Vec<(usize, u8)>>, Vec<OutputSource>)> = Vec::new();

        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            match &component.component_type {
                ComponentType::Nand => {
                    let nand_idx = self.add_gate(GateType::Nand);
                    // Record visual component instance mapping
                    instance_map.insert((path.to_vec(), comp_idx), nand_idx);
                    
                    component_ports.push((
                        vec![vec![(nand_idx, 0)], vec![(nand_idx, 1)]],
                        vec![OutputSource::DrivenByGate(nand_idx)],
                    ));
                }
                ComponentType::Clock => {
                    let clock_idx = self.add_gate(GateType::Input);
                    // Record visual component instance mapping
                    instance_map.insert((path.to_vec(), comp_idx), clock_idx);
                    
                    let period = component.clock_period.unwrap_or(20);
                    active_clocks.push(CompiledClock {
                        gate_idx: clock_idx,
                        period,
                        counter: 0,
                        visual_id: None, // Will be linked at the top-level compile
                    });

                    component_ports.push((
                        vec![], // Clocks have no input ports
                        vec![OutputSource::DrivenByGate(clock_idx)],
                    ));
                }
                ComponentType::SubChip(sub_idx) => {
                    // Recursively compile sub-chip with sub-path
                    let mut sub_path = path.to_vec();
                    sub_path.push(comp_idx);
                    let sub_interface = self.instantiate_chip_with_mapping(*sub_idx, library, &sub_path, instance_map, instance_outputs, active_clocks)?;
                    instance_outputs.insert((path.to_vec(), comp_idx), sub_interface.outputs.clone());
                    component_ports.push((sub_interface.inputs, sub_interface.outputs));
                }
                ComponentType::Input | ComponentType::Output => {
                    return Err("Blueprint components cannot contain top-level Input or Output internally".to_string());
                }
            }
        }

        // Helper closures to avoid duplicate recursive functions and cleanly capture state.
        let get_immediate_source = |node: &TraceNode| -> Option<TraceNode> {
            match node {
                TraceNode::ChipOutput(out_idx) => {
                    blueprint.connections.iter()
                        .find(|conn| conn.target == TargetPort::ChipOutput(*out_idx))
                        .map(|conn| match conn.source {
                            SourcePort::ChipInput(i) => TraceNode::ChipInput(i),
                            SourcePort::ComponentOutput { component_idx, port_idx } => {
                                TraceNode::CompOutput { component_idx, port_idx }
                            }
                        })
                }
                TraceNode::CompInput { component_idx, port_idx } => {
                    blueprint.connections.iter()
                        .find(|conn| conn.target == TargetPort::ComponentInput { component_idx: *component_idx, port_idx: *port_idx })
                        .map(|conn| match conn.source {
                            SourcePort::ChipInput(i) => TraceNode::ChipInput(i),
                            SourcePort::ComponentOutput { component_idx, port_idx } => {
                                TraceNode::CompOutput { component_idx, port_idx }
                            }
                        })
                }
                TraceNode::CompOutput { component_idx, port_idx } => {
                    let component = &blueprint.components[*component_idx];
                    match &component.component_type {
                        ComponentType::Nand | ComponentType::Input | ComponentType::Output | ComponentType::Clock => None,
                        ComponentType::SubChip(_) => {
                            let (_, ref outputs) = component_ports[*component_idx];
                            match outputs[*port_idx] {
                                OutputSource::PassedThrough(in_idx) => {
                                    Some(TraceNode::CompInput { component_idx: *component_idx, port_idx: in_idx })
                                }
                                _ => None,
                            }
                        }
                    }
                }
                TraceNode::ChipInput(_) => None,
            }
        };

        let trace_root = |start_node: TraceNode| -> OutputSource {
            let mut current = start_node;
            let mut visited = std::collections::HashSet::new();
            
            loop {
                if !visited.insert(current.clone()) {
                    return OutputSource::Floating;
                }
                
                match current {
                    TraceNode::ChipInput(idx) => {
                        return OutputSource::PassedThrough(idx);
                    }
                    TraceNode::CompOutput { component_idx, port_idx } => {
                        let component = &blueprint.components[component_idx];
                        match &component.component_type {
                            ComponentType::Nand | ComponentType::Clock => {
                                let (_, ref outputs) = component_ports[component_idx];
                                match outputs[0] {
                                    OutputSource::DrivenByGate(g_idx) => return OutputSource::DrivenByGate(g_idx),
                                    _ => return OutputSource::Floating,
                                }
                            }
                            ComponentType::SubChip(_) => {
                                let (_, ref outputs) = component_ports[component_idx];
                                match outputs[port_idx] {
                                    OutputSource::DrivenByGate(g_idx) => return OutputSource::DrivenByGate(g_idx),
                                    OutputSource::Floating => return OutputSource::Floating,
                                    OutputSource::PassedThrough(in_idx) => {
                                        current = TraceNode::CompInput { component_idx, port_idx: in_idx };
                                    }
                                }
                            }
                            ComponentType::Input | ComponentType::Output => return OutputSource::Floating,
                        }
                    }
                    _ => {
                        if let Some(next_node) = get_immediate_source(&current) {
                            current = next_node;
                        } else {
                            return OutputSource::Floating;
                        }
                    }
                }
            }
        };

        // 2. Wire up all component inputs by tracing their root drivers
        for (comp_idx, component) in blueprint.components.iter().enumerate() {
            let input_ports_count = match &component.component_type {
                ComponentType::Nand => 2,
                ComponentType::Input => 0,
                ComponentType::Output => 1,
                ComponentType::Clock => 0,
                ComponentType::SubChip(sub_idx) => library[*sub_idx].inputs,
            };

            for port_idx in 0..input_ports_count {
                let start_node = TraceNode::CompInput { component_idx: comp_idx, port_idx };
                let driver = trace_root(start_node);

                if let OutputSource::DrivenByGate(src_g_idx) = driver {
                    // Get all primitive targets of this input port
                    let targets = &component_ports[comp_idx].0[port_idx];
                    for &(tgt_g_idx, tgt_port) in targets {
                        self.connect(src_g_idx, tgt_g_idx, tgt_port);
                    }
                }
            }
        }

        // 3. Resolve the inputs interface for the parent chip
        let mut inputs = vec![Vec::new(); blueprint.inputs];
        for i in 0..blueprint.inputs {
            for (comp_idx, component) in blueprint.components.iter().enumerate() {
                let input_ports_count = match &component.component_type {
                    ComponentType::Nand => 2,
                    ComponentType::Input => 0,
                    ComponentType::Output => 1,
                    ComponentType::Clock => 0,
                    ComponentType::SubChip(sub_idx) => library[*sub_idx].inputs,
                };
                
                for port_idx in 0..input_ports_count {
                    let comp_in_node = TraceNode::CompInput { component_idx: comp_idx, port_idx };
                    let driver = trace_root(comp_in_node);
                    if driver == OutputSource::PassedThrough(i) {
                        let targets = &component_ports[comp_idx].0[port_idx];
                        inputs[i].extend(targets.iter().copied());
                    }
                }
            }
        }

        // 4. Resolve the outputs interface for the parent chip
        let mut outputs = Vec::new();
        for j in 0..blueprint.outputs {
            let start_node = TraceNode::ChipOutput(j);
            let driver = trace_root(start_node);
            outputs.push(driver);
        }

        Ok(InstantiatedInterface { inputs, outputs })
    }

    /// Instantiates a custom chip from the blueprint library using compile-time wire aliasing.
    /// Bypasses string lookups and physical buffer gates, resolving inputs/outputs to their root sources.
    pub fn instantiate_chip(
        &mut self,
        blueprint_idx: usize,
        library: &[ChipBlueprint],
    ) -> Result<InstantiatedInterface, String> {
        let mut dummy_map = std::collections::HashMap::new();
        let mut dummy_outputs = std::collections::HashMap::new();
        let mut dummy_clocks = Vec::new();
        self.instantiate_chip_with_mapping(blueprint_idx, library, &[], &mut dummy_map, &mut dummy_outputs, &mut dummy_clocks)
    }
}
