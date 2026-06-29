use crate::engine::{
    ChipBlueprint, Component, ComponentType, Connection, GateType,
    OutputSource, Simulator, SourcePort, TargetPort, CompiledClock,
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
            ComponentType::SubChip(idx) => {
                if let Some(bp) = self.library.get(idx) {
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
            ComponentType::SubChip(idx) => {
                if let Some(bp) = self.library.get(idx) {
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
                    component_ports.insert(comp.id, (
                        vec![vec![(sim_idx, 0)], vec![(sim_idx, 1)]],
                        vec![OutputSource::DrivenByGate(sim_idx)],
                    ));
                }
                ComponentType::Input => {
                    let sim_idx = sim.add_gate(GateType::Input);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports.insert(comp.id, (
                        vec![],
                        vec![OutputSource::DrivenByGate(sim_idx)],
                    ));
                }
                ComponentType::Output => {
                    let sim_idx = sim.add_gate(GateType::Output);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports.insert(comp.id, (
                        vec![vec![(sim_idx, 0)]],
                        vec![],
                    ));
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

                    component_ports.insert(comp.id, (
                        vec![],
                        vec![OutputSource::DrivenByGate(sim_idx)],
                    ));
                }
                ComponentType::SubChip(sub_idx) => {
                    let path = vec![comp.id];
                    if let Ok(sub_interface) = sim.instantiate_chip_with_mapping(
                        sub_idx,
                        &self.library,
                        &path,
                        &mut instance_to_sim_map,
                        &mut instance_outputs,
                        &mut active_clocks,
                    ) {
                        instance_outputs.insert((path.clone(), comp.id), sub_interface.outputs.clone());
                        component_ports.insert(comp.id, (
                            sub_interface.inputs,
                            sub_interface.outputs,
                        ));
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

        let get_immediate_source = |node: &CanvasNode| -> Option<CanvasNode> {
            match node {
                CanvasNode::CompInput { comp_id, port_idx } => {
                    self.connections.iter()
                        .find(|conn| conn.tgt_comp_id == *comp_id && conn.tgt_port == *port_idx)
                        .map(|conn| CanvasNode::CompOutput {
                            comp_id: conn.src_comp_id,
                            port_idx: conn.src_port,
                        })
                }
                CanvasNode::CompOutput { comp_id, port_idx } => {
                    if let Some((_, outputs)) = component_ports.get(comp_id) {
                        if *port_idx < outputs.len() {
                            match outputs[*port_idx] {
                                OutputSource::PassedThrough(in_idx) => {
                                    Some(CanvasNode::CompInput { comp_id: *comp_id, port_idx: in_idx })
                                }
                                _ => None,
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            }
        };

        let trace_root = |start_node: CanvasNode| -> OutputSource {
            let mut current = start_node;
            let mut visited = HashSet::new();
            
            loop {
                if !visited.insert(current.clone()) {
                    return OutputSource::Floating;
                }
                
                match current {
                    CanvasNode::CompOutput { comp_id, port_idx } => {
                        if let Some((_, outputs)) = component_ports.get(&comp_id) {
                            if port_idx < outputs.len() {
                                match outputs[port_idx] {
                                    OutputSource::DrivenByGate(g_idx) => return OutputSource::DrivenByGate(g_idx),
                                    OutputSource::Floating => return OutputSource::Floating,
                                    OutputSource::PassedThrough(in_idx) => {
                                        current = CanvasNode::CompInput { comp_id, port_idx: in_idx };
                                    }
                                }
                            } else {
                                return OutputSource::Floating;
                            }
                        } else {
                            return OutputSource::Floating;
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

        // 2. Wire up all component inputs on the canvas in the simulator
        for comp in &self.components {
            let (inputs_count, _) = self.get_component_ports_count(comp.comp_type);

            for port_idx in 0..inputs_count {
                let start_node = CanvasNode::CompInput { comp_id: comp.id, port_idx };
                let driver = trace_root(start_node);

                if let OutputSource::DrivenByGate(src_g_idx) = driver {
                    if let Some((inputs, _)) = component_ports.get(&comp.id) {
                        if port_idx < inputs.len() {
                            let targets = &inputs[port_idx];
                            for &(tgt_g_idx, tgt_port) in targets {
                                sim.connect(src_g_idx, tgt_g_idx, tgt_port);
                            }
                        }
                    }
                }
            }
        }

        // 3. Resolve the visual output port states map
        let mut port_to_sim_gate_map = HashMap::new();
        for comp in &self.components {
            let (_, outputs_count) = self.get_component_ports_count(comp.comp_type);

            for port_idx in 0..outputs_count {
                let start_node = CanvasNode::CompOutput { comp_id: comp.id, port_idx };
                let driver = trace_root(start_node);
                if let OutputSource::DrivenByGate(g_idx) = driver {
                    port_to_sim_gate_map.insert((comp.id, port_idx), g_idx);
                }
            }
        }

        // Settle initial states
        let _ = sim.propagate_events(5000);

        self.simulator = sim;
        self.visual_to_sim_map = visual_to_sim_map;
        self.port_to_sim_gate_map = port_to_sim_gate_map;
        self.instance_to_sim_map = instance_to_sim_map;
        self.instance_outputs = instance_outputs;
        self.active_clocks = active_clocks;
    }

    /// Translates the current canvas components and connections into a reusable ChipBlueprint
    pub(crate) fn package_current_canvas(&self) -> Option<ChipBlueprint> {
        // Collect Inputs and Outputs from canvas, sorted by Y position to preserve order
        let mut visual_inputs: Vec<VisualComponent> = self.components.iter()
            .filter(|c| c.comp_type == ComponentType::Input)
            .cloned()
            .collect();
        visual_inputs.sort_by(|a, b| a.pos.y.partial_cmp(&b.pos.y).unwrap());

        let mut visual_outputs: Vec<VisualComponent> = self.components.iter()
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
        let visual_internals: Vec<VisualComponent> = self.components.iter()
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
            let source_port = if let Some(in_idx) = visual_inputs.iter().position(|c| c.id == conn.src_comp_id) {
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
            let target_port = if let Some(out_idx) = visual_outputs.iter().position(|c| c.id == conn.tgt_comp_id) {
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

        // Handle direct feedthrough wires (Input connected directly to Output on canvas)
        for out_idx in 0..visual_outputs.len() {
            let out_comp_id = visual_outputs[out_idx].id;
            // Find if there is a connection directly from a top-level Input to this Output
            if let Some(conn) = self.connections.iter().find(|c| c.tgt_comp_id == out_comp_id) {
                if let Some(in_idx) = visual_inputs.iter().position(|c| c.id == conn.src_comp_id) {
                    connections.push(Connection {
                        source: SourcePort::ChipInput(in_idx),
                        target: TargetPort::ChipOutput(out_idx),
                    });
                }
            }
        }

        if components.is_empty() && connections.is_empty() {
            // Cannot package empty circuit
            return None;
        }

        Some(ChipBlueprint {
            name: self.chip_name_input.clone(),
            inputs: visual_inputs.len(),
            outputs: visual_outputs.len(),
            input_names,
            output_names,
            components,
            connections,
        })
    }
}
