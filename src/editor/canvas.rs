use crate::editor::state::{CanvasSnapshot, EditingTarget};
use crate::engine::{
    ChipBlueprint, CompiledClock, Component, ComponentType, Connection, GateType, OutputSource,
    Simulator, SourcePort, TargetPort,
};
use macroquad::prelude::Vec2;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
enum CanvasNode {
    CompInput { comp_id: usize, port_idx: usize },
    CompOutput { comp_id: usize, port_idx: usize },
}

use super::Editor;
use super::types::*;

impl Editor {
    pub fn get_component_ports_count(&self, comp_type: ComponentType) -> (usize, usize) {
        self.get_component_ports_count_with_width(comp_type, None)
    }

    pub fn get_component_ports_count_with_width(
        &self,
        comp_type: ComponentType,
        bus_width: Option<usize>,
    ) -> (usize, usize) {
        match comp_type {
            ComponentType::Nand => (2, 1),
            ComponentType::Input => (0, 1),
            ComponentType::Output => (1, 0),
            ComponentType::Clock => (0, 1),
            ComponentType::SevenSegment => (8, 0),
            ComponentType::TriStateBuffer => (2, 1),
            ComponentType::Junction => (1, 1),
            ComponentType::BusJoiner => (bus_width.unwrap_or(4), 1),
            ComponentType::BusSplitter => (1, bus_width.unwrap_or(4)),
            ComponentType::SubChip(idx) => self
                .engine
                .library
                .get(idx)
                .map_or((0, 0), |bp| (bp.inputs, bp.outputs)),
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
            ComponentType::BusJoiner => "JOIN".to_string(),
            ComponentType::BusSplitter => "SPLIT".to_string(),
            ComponentType::SubChip(idx) => self
                .engine
                .library
                .get(idx)
                .map_or("UNKNOWN".to_string(), |bp| bp.name.clone()),
        }
    }

    pub fn expand_connections(&self) -> Vec<VisualConnection> {
        let mut expanded = Vec::new();
        let comp_by_id: std::collections::HashMap<usize, &VisualComponent> =
            self.components.iter().map(|c| (c.id, c)).collect();

        for conn in &self.connections {
            let src_comp = comp_by_id.get(&conn.src_comp_id);
            let tgt_comp = comp_by_id.get(&conn.tgt_comp_id);

            let is_bus = src_comp.map_or(false, |c| c.comp_type == ComponentType::BusJoiner && conn.src_port == 0)
                && tgt_comp.map_or(false, |c| c.comp_type == ComponentType::BusSplitter && conn.tgt_port == 0);

            if is_bus {
                let w_src = src_comp.map_or(4, |c| c.bus_width());
                let w_tgt = tgt_comp.map_or(4, |c| c.bus_width());
                let w = w_src.min(w_tgt);
                for i in 0..w {
                    expanded.push(VisualConnection {
                        src_comp_id: conn.src_comp_id,
                        src_port: i,
                        tgt_comp_id: conn.tgt_comp_id,
                        tgt_port: i,
                    });
                }
            } else {
                expanded.push(conn.clone());
            }
        }
        expanded
    }

    pub fn compile(&mut self) {
        // Fix up sub-chip dimensions so existing projects get the new dynamic widths
        for i in 0..self.components.len() {
            if matches!(self.components[i].comp_type, ComponentType::SubChip(_)) {
                let (w, h) = self.get_component_dimensions(self.components[i].comp_type);
                self.components[i].width = w;
                self.components[i].height = h;
            }
        }

        let mut sim = Simulator::new();
        let mut visual_to_sim_map = HashMap::new();
        let mut component_ports = HashMap::new(); // visual_id -> (inputs_aliases, outputs_drivers)
        let mut instance_to_sim_map = HashMap::new();
        let mut instance_outputs = HashMap::new();
        let mut active_clocks = Vec::new();

        // 1. Allocate all visual components in the simulator
        self.allocate_visual_components(
            &mut sim,
            &mut visual_to_sim_map,
            &mut component_ports,
            &mut instance_to_sim_map,
            &mut instance_outputs,
            &mut active_clocks,
        );

        let expanded_connections = self.expand_connections();

        // 2. Wire up all component inputs on the canvas in the simulator
        let mut net_cache = HashMap::new();
        self.wire_up_component_inputs(&mut sim, &expanded_connections, &component_ports, &mut net_cache);

        // 3. Resolve the visual output port states map
        let port_to_sim_gate_map =
            self.resolve_port_to_sim_gate_map(&mut sim, &expanded_connections, &component_ports, &mut net_cache);

        // Settle initial states
        let max_steps = (sim.nodes.len() * 10).max(1000);
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

        if self.connections.len() < 5000 {
            self.recompute_wire_offsets();
        }
        
        self.rebuild_spatial_grid();
    }

    fn allocate_visual_components(
        &self,
        sim: &mut Simulator,
        visual_to_sim_map: &mut HashMap<usize, usize>,
        component_ports: &mut HashMap<usize, (Vec<Vec<(usize, u8)>>, Vec<OutputSource>)>,
        instance_to_sim_map: &mut HashMap<(Vec<usize>, usize), usize>,
        instance_outputs: &mut HashMap<(Vec<usize>, usize), Vec<OutputSource>>,
        active_clocks: &mut Vec<CompiledClock>,
    ) {
        for comp in &self.components {
            match comp.comp_type {
                ComponentType::Nand => {
                    let sim_idx = sim.add_gate(GateType::Nand);
                    visual_to_sim_map.insert(comp.id, sim_idx);
                    component_ports.insert(
                        comp.id,
                        (
                            vec![vec![(sim_idx, 0u8)], vec![(sim_idx, 1u8)]],
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
                    component_ports.insert(comp.id, (vec![vec![(sim_idx, 0u8)]], vec![]));
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
                            vec![vec![(sim_idx, 0u8)], vec![(sim_idx, 1u8)]],
                            vec![OutputSource::DrivenByGate(sim_idx)],
                        ),
                    );
                }
                ComponentType::Junction => {
                    component_ports.insert(
                        comp.id,
                        (vec![vec![]], vec![OutputSource::PassedThrough(0)]),
                    );
                }
                ComponentType::BusJoiner => {
                    let w = comp.bus_width();
                    // NOTE: Officially BusJoiner has (w, 1) ports, but internally we represent it
                    // as w inputs and w outputs (PassedThrough) to trace each channel independently.
                    component_ports.insert(
                        comp.id,
                        (
                            vec![vec![]; w],
                            (0..w).map(|i| OutputSource::PassedThrough(i)).collect(),
                        ),
                    );
                }
                ComponentType::BusSplitter => {
                    let w = comp.bus_width();
                    // NOTE: Officially BusSplitter has (1, w) ports, but internally we represent it
                    // as w inputs and w outputs (PassedThrough) to trace each channel independently.
                    component_ports.insert(
                        comp.id,
                        (
                            vec![vec![]; w],
                            (0..w).map(|i| OutputSource::PassedThrough(i)).collect(),
                        ),
                    );
                }
                ComponentType::SevenSegment => {
                    let mut inputs = Vec::new();
                    // Port ordering: A, B, C, D, E, F, G, minus (8 ports total)
                    for _ in 0..8 {
                        let sim_idx = sim.add_gate(GateType::Output);
                        inputs.push(vec![(sim_idx, 0u8)]);
                    }
                    component_ports.insert(comp.id, (inputs, vec![]));
                }
                ComponentType::SubChip(sub_idx) => {
                    let path = vec![comp.id];
                    let mut blueprint_stack = Vec::new();
                    if let Ok(sub_interface) = sim.instantiate_chip_with_mapping(
                        sub_idx,
                        &self.engine.library,
                        &path,
                        instance_to_sim_map,
                        instance_outputs,
                        active_clocks,
                        &mut blueprint_stack,
                    ) {
                        component_ports
                            .insert(comp.id, (sub_interface.inputs, sub_interface.outputs));
                    }
                }
            }
        }
    }

    fn wire_up_component_inputs(
        &self,
        sim: &mut Simulator,
        connections: &[VisualConnection],
        component_ports: &HashMap<usize, (Vec<Vec<(usize, u8)>>, Vec<OutputSource>)>,
        net_cache: &mut HashMap<Vec<usize>, OutputSource>,
    ) {
        for comp in &self.components {
            let (inputs_count, _) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));

            for port_idx in 0..inputs_count {
                let start_node = CanvasNode::CompInput {
                    comp_id: comp.id,
                    port_idx,
                };
                let driver = self.trace_canvas_node(start_node, sim, connections, component_ports, net_cache);

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
    }

    fn resolve_port_to_sim_gate_map(
        &self,
        sim: &mut Simulator,
        connections: &[VisualConnection],
        component_ports: &HashMap<usize, (Vec<Vec<(usize, u8)>>, Vec<OutputSource>)>,
        net_cache: &mut HashMap<Vec<usize>, OutputSource>,
    ) -> HashMap<(usize, usize), usize> {
        let mut port_to_sim_gate_map = HashMap::new();
        for comp in &self.components {
            let (_, outputs_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));

            for port_idx in 0..outputs_count {
                let start_node = CanvasNode::CompOutput {
                    comp_id: comp.id,
                    port_idx,
                };
                let driver = self.trace_canvas_node(start_node, sim, connections, component_ports, net_cache);
                if let OutputSource::DrivenByGate(g_idx) = driver {
                    port_to_sim_gate_map.insert((comp.id, port_idx), g_idx);
                }
            }
        }
        port_to_sim_gate_map
    }

    fn trace_canvas_node(
        &self,
        start_node: CanvasNode,
        sim: &mut Simulator,
        connections: &[VisualConnection],
        component_ports: &HashMap<usize, (Vec<Vec<(usize, u8)>>, Vec<OutputSource>)>,
        net_cache: &mut HashMap<Vec<usize>, OutputSource>,
    ) -> OutputSource {
        let mut visited = HashSet::new();
        let mut queue = vec![start_node];
        let mut drivers = Vec::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current) {
                continue;
            }

            match current {
                CanvasNode::CompOutput { comp_id, port_idx } => {
                    if let Some((_, outputs)) = component_ports.get(&comp_id)
                        && port_idx < outputs.len()
                    {
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
                CanvasNode::CompInput { comp_id, port_idx } => {
                    for conn in connections {
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
            for &driver in drivers.iter().skip(1) {
                let resolver = sim.add_gate(GateType::BusResolver);
                sim.connect(current_idx, resolver, 0);
                sim.connect(driver, resolver, 1);
                current_idx = resolver;
            }
            OutputSource::DrivenByGate(current_idx)
        };

        net_cache.insert(drivers, result);
        result
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
        visual_inputs.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));

        let mut visual_outputs: Vec<VisualComponent> = self
            .components
            .iter()
            .filter(|c| c.comp_type == ComponentType::Output)
            .cloned()
            .collect();
        visual_outputs.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));

        // Resolve port name collisions and sanitize blank labels
        let mut input_names = Vec::new();
        let mut input_counts = HashMap::new();
        for comp in &visual_inputs {
            let base_label = if comp.label == "IN" || comp.label.trim().is_empty() {
                format!("IN_{}", input_names.len())
            } else {
                comp.label.clone()
            };

            let final_label = match input_counts.get_mut(&base_label) {
                Some(count) => {
                    *count += 1;
                    format!("{}_{}", base_label, *count - 1)
                }
                None => {
                    let label = base_label.clone();
                    input_counts.insert(base_label, 1);
                    label
                }
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

            let final_label = match output_counts.get_mut(&base_label) {
                Some(count) => {
                    *count += 1;
                    format!("{}_{}", base_label, *count - 1)
                }
                None => {
                    let label = base_label.clone();
                    output_counts.insert(base_label, 1);
                    label
                }
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

    pub fn get_component_dimensions(&self, comp_type: ComponentType) -> (f32, f32) {
        self.get_component_dimensions_with_width(comp_type, None)
    }

    pub fn get_component_dimensions_with_width(
        &self,
        comp_type: ComponentType,
        bus_width: Option<usize>,
    ) -> (f32, f32) {
        let (inputs, outputs) = self.get_component_ports_count_with_width(comp_type, bus_width);
        let max_ports = inputs.max(outputs);
        let mut height = 40.0 + (max_ports as f32 * 16.0);
        let mut width = match comp_type {
            ComponentType::SubChip(idx) => {
                if let Some(bp) = self.engine.library.get(idx) {
                    let max_in = bp.input_names.iter().map(|n| n.len()).max().unwrap_or(0);
                    let max_out = bp.output_names.iter().map(|n| n.len()).max().unwrap_or(0);
                    let title_len = bp.name.len();

                    // Left port text takes ~8px per char + 10px padding
                    let left_w = (max_in as f32 * 8.0) + 10.0;
                    // Right port text takes ~8px per char + 10px padding
                    let right_w = (max_out as f32 * 8.0) + 10.0;
                    // Title takes ~10px per char
                    let title_w = (title_len as f32) * 10.0;

                    // Total width needs to comfortably fit all three side-by-side with extra padding
                    let total_w = left_w + title_w + right_w + 30.0;

                    total_w.max(120.0)
                } else {
                    100.0
                }
            }
            ComponentType::BusJoiner | ComponentType::BusSplitter => 50.0,
            _ => 70.0,
        };
        if comp_type == ComponentType::Junction {
            width = 12.0;
            height = 12.0;
        }
        (width, height)
    }

    pub fn unpack_blueprint_to_canvas(&mut self, bp_idx: usize) {
        if self.canvas.editing_target != EditingTarget::MainCanvas {
            self.save_and_repack_blueprint();
        }
        let bp = if let Some(bp) = self.engine.library.get(bp_idx) {
            bp.clone()
        } else {
            return;
        };

        self.canvas.stashed_main_canvas = Some(CanvasSnapshot {
            components: self.components.clone(),
            connections: self.connections.clone(),
            annotations: self.annotations.clone(),
            next_component_id: self.next_component_id,
            pan: self.canvas.pan,
            zoom: self.canvas.zoom,
        });

        self.components.clear();
        self.connections.clear();
        self.annotations.clear();
        self.next_component_id = 1;
        self.canvas.selected_comp_id = None;
        self.canvas.selected_comp_ids.clear();
        self.canvas.inspection_path.clear();
        self.canvas.editing_target = EditingTarget::LibraryChip(bp_idx);

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for comp in &bp.components {
            let (w, h) = self.get_component_dimensions(comp.component_type);
            min_x = min_x.min(comp.pos.0);
            max_x = max_x.max(comp.pos.0 + w);
            min_y = min_y.min(comp.pos.1);
            max_y = max_y.max(comp.pos.1 + h);
        }

        if min_x > max_x {
            min_x = 200.0;
            max_x = 600.0;
            min_y = 100.0;
            max_y = 100.0;
        }

        let mut bp_comp_idx_to_visual_id = HashMap::new();

        for (i, comp) in bp.components.iter().enumerate() {
            let vis_id = self.next_component_id;
            self.next_component_id += 1;

            let label = self.get_component_label(comp.component_type);
            let (width, height) = self.get_component_dimensions(comp.component_type);

            self.components.push(VisualComponent {
                id: vis_id,
                comp_type: comp.component_type,
                pos: Vec2::new(comp.pos.0, comp.pos.1),
                width,
                height,
                label,
                clock_period: comp.clock_period,
                color: None,
            });
            bp_comp_idx_to_visual_id.insert(i, vis_id);
        }

        let mut bp_in_to_visual_id = HashMap::new();
        let center_y = (min_y + max_y) / 2.0;
        let spacing_y = 60.0;
        let inputs_height = (bp.inputs.max(1) - 1) as f32 * spacing_y;
        let input_y_start = center_y - (inputs_height / 2.0);

        for i in 0..bp.inputs {
            let vis_id = self.next_component_id;
            self.next_component_id += 1;
            let label = bp
                .input_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| "IN".to_string());
            self.components.push(VisualComponent {
                id: vis_id,
                comp_type: ComponentType::Input,
                pos: Vec2::new(min_x - 150.0, input_y_start + i as f32 * spacing_y),
                width: 70.0,
                height: 40.0,
                label,
                clock_period: None,
                color: None,
            });
            bp_in_to_visual_id.insert(i, vis_id);
        }

        let mut bp_out_to_visual_id = HashMap::new();
        let outputs_height = (bp.outputs.max(1) - 1) as f32 * spacing_y;
        let output_y_start = center_y - (outputs_height / 2.0);

        for i in 0..bp.outputs {
            let vis_id = self.next_component_id;
            self.next_component_id += 1;
            let label = bp
                .output_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| "OUT".to_string());
            self.components.push(VisualComponent {
                id: vis_id,
                comp_type: ComponentType::Output,
                pos: Vec2::new(max_x + 150.0, output_y_start + i as f32 * spacing_y),
                width: 70.0,
                height: 40.0,
                label,
                clock_period: None,
                color: None,
            });
            bp_out_to_visual_id.insert(i, vis_id);
        }

        for conn in bp.connections {
            let src_comp_id = match conn.source {
                SourcePort::ChipInput(idx) => *bp_in_to_visual_id.get(&idx).unwrap(),
                SourcePort::ComponentOutput {
                    component_idx,
                    port_idx: _,
                } => *bp_comp_idx_to_visual_id.get(&component_idx).unwrap(),
            };
            let src_port = match conn.source {
                SourcePort::ChipInput(_) => 0,
                SourcePort::ComponentOutput {
                    component_idx: _,
                    port_idx,
                } => port_idx,
            };

            let tgt_comp_id = match conn.target {
                TargetPort::ChipOutput(idx) => *bp_out_to_visual_id.get(&idx).unwrap(),
                TargetPort::ComponentInput {
                    component_idx,
                    port_idx: _,
                } => *bp_comp_idx_to_visual_id.get(&component_idx).unwrap(),
            };
            let tgt_port = match conn.target {
                TargetPort::ChipOutput(_) => 0,
                TargetPort::ComponentInput {
                    component_idx: _,
                    port_idx,
                } => port_idx,
            };

            self.connections.push(VisualConnection {
                src_comp_id,
                src_port,
                tgt_comp_id,
                tgt_port,
            });
        }

        self.canvas.pan = Vec2::new(0.0, 0.0);
        self.compile();
        self.center_camera_on_components();
    }

    pub fn center_camera_on_components(&mut self) {
        if self.components.is_empty() {
            self.canvas.pan = Vec2::ZERO;
            self.canvas.zoom = 1.0;
            return;
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for comp in &self.components {
            min_x = min_x.min(comp.pos.x);
            max_x = max_x.max(comp.pos.x + comp.width);
            min_y = min_y.min(comp.pos.y);
            max_y = max_y.max(comp.pos.y + comp.height);
        }

        self.apply_camera_bounds(min_x, max_x, min_y, max_y);
    }

    pub fn center_camera_on_inspection_view(&mut self) {
        if let Some((bp, internal_components)) = self.get_inspected_blueprint_and_components() {
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;

            for comp in &internal_components {
                let (w, h) = self.get_component_dimensions(comp.component_type);
                min_x = min_x.min(comp.pos.0);
                max_x = max_x.max(comp.pos.0 + w);
                min_y = min_y.min(comp.pos.1);
                max_y = max_y.max(comp.pos.1 + h);
            }
            if min_x > max_x {
                min_x = 200.0;
                max_x = 600.0;
                min_y = 100.0;
                max_y = 100.0;
            }

            let input_x = min_x - 150.0;
            let output_x = max_x + 150.0;
            let spacing_y = 60.0;
            let center_y = (min_y + max_y) / 2.0;

            let inputs_height = (bp.inputs.max(1) - 1) as f32 * spacing_y;
            let outputs_height = (bp.outputs.max(1) - 1) as f32 * spacing_y;

            let input_y_start = center_y - (inputs_height / 2.0);
            let output_y_start = center_y - (outputs_height / 2.0);

            let true_min_y = min_y.min(input_y_start).min(output_y_start);
            let true_max_y = max_y
                .max(input_y_start + inputs_height)
                .max(output_y_start + outputs_height);

            self.apply_camera_bounds(input_x, output_x, true_min_y, true_max_y);
        }
    }

    fn apply_camera_bounds(&mut self, min_x: f32, max_x: f32, min_y: f32, max_y: f32) {
        let width = max_x - min_x;
        let height = max_y - min_y;

        let padded_width = width + 400.0;
        let padded_height = height + 400.0;

        let (view_x, view_y, view_w, view_h) = self.ui.canvas_viewport.unwrap_or((
            0.0,
            0.0,
            macroquad::window::screen_width(),
            macroquad::window::screen_height(),
        ));

        let zoom_x = view_w / padded_width;
        let zoom_y = view_h / padded_height;
        let target_zoom = zoom_x.min(zoom_y).clamp(0.01, 2.0);

        self.canvas.zoom = target_zoom;
        let scx = view_x + view_w / 2.0;
        let scy = view_y + view_h / 2.0;

        let cx = (min_x + max_x) / 2.0;
        let cy = (min_y + max_y) / 2.0;

        self.canvas.pan = Vec2::new(scx - cx * target_zoom, scy - cy * target_zoom);
    }

    pub fn save_and_repack_blueprint(&mut self) {
        if let EditingTarget::LibraryChip(bp_idx) = self.canvas.editing_target {
            if let Some(new_bp) = self.package_current_canvas() {
                let mut updated_bp = new_bp;
                if let Some(entry) = self.engine.library.get_mut(bp_idx) {
                    updated_bp.name = entry.name.clone();
                    *entry = updated_bp;
                }
            }

            if let Some(stashed) = self.canvas.stashed_main_canvas.take() {
                self.components = stashed.components;
                self.connections = stashed.connections;
                self.annotations = stashed.annotations;
                self.next_component_id = stashed.next_component_id;
                self.canvas.pan = stashed.pan;
                self.canvas.zoom = stashed.zoom;
            }

            self.canvas.editing_target = EditingTarget::MainCanvas;
            self.compile();
        }
    }

    pub fn cancel_and_return(&mut self) {
        if let EditingTarget::LibraryChip(_) = self.canvas.editing_target {
            if let Some(stashed) = self.canvas.stashed_main_canvas.take() {
                self.components = stashed.components;
                self.connections = stashed.connections;
                self.annotations = stashed.annotations;
                self.next_component_id = stashed.next_component_id;
                self.canvas.pan = stashed.pan;
                self.canvas.zoom = stashed.zoom;
            }

            self.canvas.editing_target = EditingTarget::MainCanvas;
            self.compile();
        }
    }
}
