use crate::engine::{
    ChipBlueprint, CompiledClock, Component, ComponentType, Connection, GateType, OutputSource,
    Simulator, SourcePort, TargetPort,
};
use std::collections::{HashMap, HashSet};

use super::Editor;
use super::types::*;

impl Editor {
    pub fn get_component_ports_count(&self, comp_type: ComponentType) -> (usize, usize) {
        match comp_type {
            ComponentType::Nand => (2, 1),
            ComponentType::Input => (0, 1),
            ComponentType::Output => (1, 0),
            ComponentType::Clock => (0, 1),
            ComponentType::SevenSegment => (7, 0),
            ComponentType::TriStateBuffer => (2, 1),
            ComponentType::Junction => (1, 1),
            ComponentType::SubChip(idx) => {
                if let Some(bp) = self.engine.library.get(idx) {
                    (bp.inputs, bp.outputs)
                } else {
                    (0, 0)
                }
            }
        }
    }

    pub fn get_component_label(&self, comp_type: ComponentType) -> String {
        match comp_type {
            ComponentType::Nand => "NAND".to_string(),
            ComponentType::Input => "IN".to_string(),
            ComponentType::Output => "OUT".to_string(),
            ComponentType::Clock => "CLK".to_string(),
            ComponentType::SevenSegment => "7SEG".to_string(),
            ComponentType::TriStateBuffer => "TRI".to_string(),
            ComponentType::Junction => "".to_string(),
            ComponentType::SubChip(idx) => {
                if let Some(bp) = self.engine.library.get(idx) {
                    bp.name.clone()
                } else {
                    "UNKNOWN".to_string()
                }
            }
        }
    }

    pub fn compile(&mut self) {
        let mut sim = Simulator::new();
        let mut visual_to_sim_map = HashMap::new();
        let mut component_ports = HashMap::new(); // visual_id -> (inputs_aliases, outputs_drivers)
        let mut instance_to_sim_map = HashMap::new();
        let mut instance_outputs = HashMap::new();
        let mut active_clocks = Vec::new();

        // 1. Allocate all visual components in the simulator
        for comp in &self.components {
            match comp.comp_type {
                ComponentType::Nand => {
                    let sim_idx = sim.add_gate(GateType::Nand);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports.insert(
                        comp.id,
                        (
                            vec![vec![(sim_idx, 0)], vec![(sim_idx, 1)]],
                            vec![OutputSource::DrivenByGate(sim_idx)],
                        ),
                    );
                }
                ComponentType::Input => {
                    let sim_idx = sim.add_gate(GateType::Input);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports
                        .insert(comp.id, (vec![], vec![OutputSource::DrivenByGate(sim_idx)]));
                }
                ComponentType::Output => {
                    let sim_idx = sim.add_gate(GateType::Output);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports.insert(comp.id, (vec![vec![(sim_idx, 0)]], vec![]));
                }
                ComponentType::Clock => {
                    let sim_idx = sim.add_gate(GateType::Input);
                    visual_to_sim_map.insert(comp.id, sim_idx);

                    let period = comp.clock_period.unwrap_or(20);
                    active_clocks.push(CompiledClock {
                        gate_idx: sim_idx,
                        period,
                        counter: 0,
                        visual_id: Some(comp.id),
                    });

                    component_ports
                        .insert(comp.id, (vec![], vec![OutputSource::DrivenByGate(sim_idx)]));
                }
                ComponentType::TriStateBuffer => {
                    let sim_idx = sim.add_gate(GateType::TriStateBuffer);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports.insert(
                        comp.id,
                        (
                            vec![vec![(sim_idx, 0)], vec![(sim_idx, 1)]],
                            vec![OutputSource::DrivenByGate(sim_idx)],
                        ),
                    );
                }
                ComponentType::Junction => {
                    component_ports.insert(comp.id, (vec![vec![]], vec![OutputSource::PassedThrough(0)]));
                }
                ComponentType::SevenSegment => {
                    // It has 7 inputs and no outputs
                    let mut inputs = Vec::new();
                    for _ in 0..7 {
                        // Dummy gates acting as input anchors for the 7seg display
                        let sim_idx = sim.add_gate(GateType::Output);
                        inputs.push(vec![(sim_idx, 0)]);
                    }
                    component_ports.insert(comp.id, (inputs, vec![]));
                }
                ComponentType::SubChip(sub_idx) => {
                    let path = vec![comp.id];
                    if let Ok(sub_interface) = sim.instantiate_chip_with_mapping(
                        sub_idx,
                        &self.engine.library,
                        &path,
                        &mut instance_to_sim_map,
                        &mut instance_outputs,
                        &mut active_clocks,
                    ) {
                        instance_outputs.insert((vec![], comp.id), sub_interface.outputs.clone());
                        component_ports
                            .insert(comp.id, (sub_interface.inputs, sub_interface.outputs));
                    }
                }
            }
        }

        // Define Trace Node inside compilation
        #[derive(Clone, Debug, Hash, PartialEq, Eq)]
        enum CanvasNode {
            CompInput { comp_id: usize, port_idx: usize },
            CompOutput { comp_id: usize, port_idx: usize },
        }

        let mut net_cache: HashMap<Vec<usize>, OutputSource> = HashMap::new();

        let mut trace_root = |start_node: CanvasNode, sim: &mut Simulator| -> OutputSource {
            let mut visited = HashSet::new();
            let mut queue = vec![start_node];
            let mut drivers = Vec::new();

            while let Some(current) = queue.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }

                match current {
                    CanvasNode::CompOutput { comp_id, port_idx } => {
                        if let Some((_, outputs)) = component_ports.get(&comp_id) {
                            if port_idx < outputs.len() {
                                match outputs[port_idx] {
                                    OutputSource::DrivenByGate(g_idx) => {
                                        if !drivers.contains(&g_idx) {
                                            drivers.push(g_idx);
                                        }
                                    }
                                    OutputSource::Floating => {}
                                    OutputSource::PassedThrough(in_idx) => {
                                        queue.push(CanvasNode::CompInput {
                                            comp_id,
                                            port_idx: in_idx,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    CanvasNode::CompInput { comp_id, port_idx } => {
                        // find all wires targeting this input
                        for conn in &self.connections {
                            if conn.tgt_comp_id == comp_id && conn.tgt_port == port_idx {
                                queue.push(CanvasNode::CompOutput {
                                    comp_id: conn.src_comp_id,
                                    port_idx: conn.src_port,
                                });
                            }
                        }
                    }
                }
            }

            drivers.sort();
            if let Some(cached) = net_cache.get(&drivers) {
                return *cached;
            }

            let result = if drivers.is_empty() {
                OutputSource::Floating
            } else if drivers.len() == 1 {
                OutputSource::DrivenByGate(drivers[0])
            } else {
                let mut current_idx = drivers[0];
                for i in 1..drivers.len() {
                    let resolver = sim.add_gate(GateType::BusResolver);
                    sim.connect(current_idx, resolver, 0);
                    sim.connect(drivers[i], resolver, 1);
                    current_idx = resolver;
                }
                OutputSource::DrivenByGate(current_idx)
            };

            net_cache.insert(drivers, result);
            result
        };

        // 2. Wire up all component inputs on the canvas in the simulator
        for comp in &self.components {
            let (inputs_count, _) = self.get_component_ports_count(comp.comp_type);

            for port_idx in 0..inputs_count {
                let start_node = CanvasNode::CompInput {
                    comp_id: comp.id,
                    port_idx,
                };
                let driver = trace_root(start_node, &mut sim);

                if let OutputSource::DrivenByGate(src_g_idx) = driver
                    && let Some((inputs, _)) = component_ports.get(&comp.id)
                    && port_idx < inputs.len()
                {
                    let targets = &inputs[port_idx];
                    for &(tgt_g_idx, tgt_port) in targets {
                        sim.connect(src_g_idx, tgt_g_idx, tgt_port);
                    }
                }
            }
        }

        // 3. Resolve the visual output port states map
        let mut port_to_sim_gate_map = HashMap::new();
        for comp in &self.components {
            let (_, outputs_count) = self.get_component_ports_count(comp.comp_type);

            for port_idx in 0..outputs_count {
                let start_node = CanvasNode::CompOutput {
                    comp_id: comp.id,
                    port_idx,
                };
                let driver = trace_root(start_node, &mut sim);
                if let OutputSource::DrivenByGate(g_idx) = driver {
                    port_to_sim_gate_map.insert((comp.id, port_idx), g_idx);
                }
            }
        }

        // Settle initial states
        let max_steps = (sim.gates.len() * 10).max(1000);
        match sim.propagate_events(max_steps) {
            Ok(_) => self.engine.propagation_error = None,
            Err(e) => self.engine.propagation_error = Some(e),
        }

        self.engine.simulator = sim;
        self.engine.visual_to_sim_map = visual_to_sim_map;
        self.engine.port_to_sim_gate_map = port_to_sim_gate_map;
        self.engine.instance_to_sim_map = instance_to_sim_map;
        self.engine.instance_outputs = instance_outputs;
        self.engine.active_clocks = active_clocks;
    }

    /// Translates the current canvas components and connections into a reusable ChipBlueprint
    pub(crate) fn package_current_canvas(&self) -> Option<ChipBlueprint> {
        // Collect Inputs and Outputs from canvas, sorted by Y position to preserve order
        let mut visual_inputs: Vec<VisualComponent> = self
            .components
            .iter()
            .filter(|c| c.comp_type == ComponentType::Input)
            .cloned()
            .collect();
        visual_inputs.sort_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).unwrap());

        let mut visual_outputs: Vec<VisualComponent> = self
            .components
            .iter()
            .filter(|c| c.comp_type == ComponentType::Output)
            .cloned()
            .collect();
        visual_outputs.sort_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).unwrap());

        // Resolve port name collisions and sanitize blank labels
        let mut input_names = Vec::new();
        let mut input_counts = HashMap::new();
        for comp in &visual_inputs {
            let base_label = if comp.label == "IN" || comp.label.trim().is_empty() {
                format!("IN_{}", input_names.len())
            } else {
                comp.label.clone()
            };

            let count = input_counts.entry(base_label.clone()).or_insert(0);
            *count += 1;
            let final_label = if *count > 1 {
                format!("{}_{}", base_label, *count - 1)
            } else {
                base_label
            };
            input_names.push(final_label);
        }

        let mut output_names = Vec::new();
        let mut output_counts = HashMap::new();
        for comp in &visual_outputs {
            let base_label = if comp.label == "OUT" || comp.label.trim().is_empty() {
                format!("OUT_{}", output_names.len())
            } else {
                comp.label.clone()
            };

            let count = output_counts.entry(base_label.clone()).or_insert(0);
            *count += 1;
            let final_label = if *count > 1 {
                format!("{}_{}", base_label, *count - 1)
            } else {
                base_label
            };
            output_names.push(final_label);
        }

        // Collect internal components
        let visual_internals: Vec<VisualComponent> = self
            .components
            .iter()
            .filter(|c| c.comp_type != ComponentType::Input && c.comp_type != ComponentType::Output)
            .cloned()
            .collect();

        // Create blueprint components
        let mut components = Vec::new();
        let mut comp_id_to_bp_idx = HashMap::new(); // visual component ID -> blueprint component index

        for (idx, comp) in visual_internals.iter().enumerate() {
            components.push(Component {
                component_type: comp.comp_type,
                pos: (comp.pos.x, comp.pos.y),
                clock_period: comp.clock_period,
            });
            comp_id_to_bp_idx.insert(comp.id, idx);
        }

        // Translate connections
        let mut connections = Vec::new();

        for conn in &self.connections {
            // 1. Resolve source
            let source_port =
                if let Some(in_idx) = visual_inputs.iter().position(|c| c.id == conn.src_comp_id) {
                    // Connection starts at a top-level Input pin
                    Some(SourcePort::ChipInput(in_idx))
                } else if let Some(&comp_idx) = comp_id_to_bp_idx.get(&conn.src_comp_id) {
                    // Connection starts at an internal component output
                    Some(SourcePort::ComponentOutput {
                        component_idx: comp_idx,
                        port_idx: conn.src_port,
                    })
                } else {
                    None
                };

            // 2. Resolve target
            let target_port = if let Some(out_idx) =
                visual_outputs.iter().position(|c| c.id == conn.tgt_comp_id)
            {
                // Connection targets a top-level Output pin
                Some(TargetPort::ChipOutput(out_idx))
            } else if let Some(&comp_idx) = comp_id_to_bp_idx.get(&conn.tgt_comp_id) {
                // Connection targets an internal component input
                Some(TargetPort::ComponentInput {
                    component_idx: comp_idx,
                    port_idx: conn.tgt_port,
                })
            } else {
                None
            };

            if let (Some(source), Some(target)) = (source_port, target_port) {
                connections.push(Connection { source, target });
            }
        }

        if components.is_empty() && connections.is_empty() {
            // Cannot package empty circuit
            return None;
        }

        Some(ChipBlueprint {
            name: self.ui.chip_name_input.clone(),
            inputs: visual_inputs.len(),
            outputs: visual_outputs.len(),
            input_names,
            output_names,
            components,
            connections,
        })
    }
}
