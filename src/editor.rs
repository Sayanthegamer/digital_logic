use crate::engine::{
    ChipBlueprint, Component, ComponentType, Connection, GateType,
    OutputSource, Simulator, SourcePort, TargetPort, TraceNode, CompiledClock,
};
use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

pub mod serde_vec2 {
    use macroquad::prelude::Vec2;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(vec: &Vec2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tup = (vec.x, vec.y);
        serde::Serialize::serialize(&tup, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec2, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tup: (f32, f32) = Deserialize::deserialize(deserializer)?;
        Ok(Vec2::new(tup.0, tup.1))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VisualComponent {
    pub id: usize,
    pub comp_type: ComponentType,
    #[serde(with = "serde_vec2")]
    pub pos: Vec2,
    pub width: f32,
    pub height: f32,
    pub label: String,
    pub clock_period: Option<usize>, // Localized period in ticks (only for Clock)
}

impl VisualComponent {
    pub fn input_port_pos(&self, port_idx: usize, num_inputs: usize) -> Vec2 {
        if num_inputs == 0 {
            return self.pos;
        }
        let spacing = self.height / (num_inputs + 1) as f32;
        let y = self.pos.y + spacing * (port_idx + 1) as f32;
        Vec2::new(self.pos.x, y)
    }

    pub fn output_port_pos(&self, port_idx: usize, num_outputs: usize) -> Vec2 {
        if num_outputs == 0 {
            return self.pos + Vec2::new(self.width, 0.0);
        }
        let spacing = self.height / (num_outputs + 1) as f32;
        let y = self.pos.y + spacing * (port_idx + 1) as f32;
        Vec2::new(self.pos.x + self.width, y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VisualConnection {
    pub src_comp_id: usize,
    pub src_port: usize,
    pub tgt_comp_id: usize,
    pub tgt_port: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveTool {
    PlaceComponent(ComponentType),
    PlaceAnnotation,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TextAnnotation {
    pub text: String,
    #[serde(with = "serde_vec2")]
    pub pos: Vec2,
}

pub struct Editor {
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub next_component_id: usize,
    
    // Annotations
    pub annotations: Vec<TextAnnotation>,
    pub selected_annotation_idx: Option<usize>,
    pub dragging_annotation_idx: Option<usize>,
    
    // Zoom/Pan
    pub pan: Vec2,
    pub zoom: f32,
    
    // Interaction States
    pub selected_tool: Option<ActiveTool>,
    pub active_wire_drag: Option<(usize, usize)>, // (src_comp_id, src_port_idx)
    pub dragging_comp_id: Option<usize>,
    pub drag_offset: Vec2,
    
    // Simulation Backend
    pub library: Vec<ChipBlueprint>,
    pub simulator: Simulator,
    pub visual_to_sim_map: HashMap<usize, usize>, // Visual ID -> Sim gate index (for primitives)
    pub port_to_sim_gate_map: HashMap<(usize, usize), usize>, // (Visual ID, port_idx) -> Sim gate index
    
    // Simulation controls
    pub is_playing: bool,
    pub ticks_per_frame: usize,
    pub sim_tick_counter: usize,
    
    // Packaging Menu
    pub chip_name_input: String,
    
    // egui pointer input check cached from the previous frame
    pub egui_wants_pointer: bool,

    // Currently selected visual component
    pub selected_comp_id: Option<usize>,

    // Look inside mappings and navigation path
    pub instance_to_sim_map: HashMap<(Vec<usize>, usize), usize>,
    pub instance_outputs: HashMap<(Vec<usize>, usize), Vec<OutputSource>>,
    pub inspection_path: Vec<usize>,
    
    // Clocks
    pub active_clocks: Vec<CompiledClock>,
}

impl Editor {
    pub fn new() -> Self {
        let mut editor = Self {
            components: Vec::new(),
            connections: Vec::new(),
            next_component_id: 1,
            annotations: Vec::new(),
            selected_annotation_idx: None,
            dragging_annotation_idx: None,
            pan: Vec2::new(200.0, 100.0),
            zoom: 1.0,
            selected_tool: None,
            active_wire_drag: None,
            dragging_comp_id: None,
            drag_offset: Vec2::ZERO,
            library: Vec::new(),
            simulator: Simulator::new(),
            visual_to_sim_map: HashMap::new(),
            port_to_sim_gate_map: HashMap::new(),
            is_playing: true,
            ticks_per_frame: 1,
            sim_tick_counter: 0,
            chip_name_input: "MY_CHIP".to_string(),
            egui_wants_pointer: false,
            selected_comp_id: None,
            instance_to_sim_map: HashMap::new(),
            instance_outputs: HashMap::new(),
            inspection_path: Vec::new(),
            active_clocks: Vec::new(),
        };

        // Add some basic chips to the library as initial examples
        editor.setup_default_library();
        editor.compile();
        editor
    }

    fn setup_default_library(&mut self) {
        // 0: AND (2 NANDs)
        self.library.push(ChipBlueprint {
            name: "AND".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component { component_type: ComponentType::Nand, pos: (200.0, 150.0), clock_period: None }, // Comp 0
                Component { component_type: ComponentType::Nand, pos: (400.0, 150.0), clock_period: None }, // Comp 1
            ],
            connections: vec![
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
            ],
        });

        // 1: OR (3 NANDs)
        self.library.push(ChipBlueprint {
            name: "OR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component { component_type: ComponentType::Nand, pos: (200.0, 100.0), clock_period: None }, // A inverter
                Component { component_type: ComponentType::Nand, pos: (200.0, 250.0), clock_period: None }, // B inverter
                Component { component_type: ComponentType::Nand, pos: (400.0, 175.0), clock_period: None }, // final NAND
            ],
            connections: vec![
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
            ],
        });

        // 2: XOR (4 NANDs)
        self.library.push(ChipBlueprint {
            name: "XOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component { component_type: ComponentType::Nand, pos: (200.0, 175.0), clock_period: None }, // NAND 0: Shared inputs
                Component { component_type: ComponentType::Nand, pos: (350.0, 100.0), clock_period: None }, // NAND 1: Top branch
                Component { component_type: ComponentType::Nand, pos: (350.0, 250.0), clock_period: None }, // NAND 2: Bottom branch
                Component { component_type: ComponentType::Nand, pos: (500.0, 175.0), clock_period: None }, // NAND 3: Combiner
            ],
            connections: vec![
                // A -> NAND 0 input 0, and NAND 1 input 0
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                
                // B -> NAND 0 input 1, and NAND 2 input 1
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                
                // NAND 0 output -> NAND 1 input 1, and NAND 2 input 0
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                
                // NAND 1 output -> NAND 3 input 0
                Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 3, port_idx: 0 } },
                // NAND 2 output -> NAND 3 input 1
                Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 3, port_idx: 1 } },
                
                // NAND 3 output -> Chip Output 0
                Connection { source: SourcePort::ComponentOutput { component_idx: 3, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
            ],
        });
    }

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

    pub fn get_inspected_blueprint_and_components(&self) -> Option<(&ChipBlueprint, Vec<Component>)> {
        if self.inspection_path.is_empty() {
            return None;
        }
        
        let bp_idx = self.get_blueprint_idx_for_path(&self.inspection_path)?;
        let blueprint = &self.library[bp_idx];
        Some((blueprint, blueprint.components.clone()))
    }

    pub fn get_blueprint_idx_for_path(&self, path: &[usize]) -> Option<usize> {
        if path.is_empty() {
            return None;
        }
        let first_comp_id = path[0];
        let curr_comp = self.components.iter().find(|c| c.id == first_comp_id)?;
        let mut curr_bp_idx = match curr_comp.comp_type {
            ComponentType::SubChip(idx) => idx,
            _ => return None,
        };

        for &comp_idx in path.iter().skip(1) {
            let blueprint = &self.library[curr_bp_idx];
            if comp_idx < blueprint.components.len() {
                let next_comp = &blueprint.components[comp_idx];
                curr_bp_idx = match next_comp.component_type {
                    ComponentType::SubChip(idx) => idx,
                    _ => return None,
                };
            } else {
                return None;
            }
        }
        Some(curr_bp_idx)
    }

    pub fn get_node_state_at_path(&self, node: &TraceNode, path: &[usize]) -> bool {
        if path.is_empty() {
            match node {
                TraceNode::ChipInput(idx) => {
                    let inputs: Vec<&VisualComponent> = self.components.iter()
                        .filter(|c| c.comp_type == ComponentType::Input)
                        .collect();
                    if let Some(comp) = inputs.get(*idx) {
                        if let Some(&g_idx) = self.visual_to_sim_map.get(&comp.id) {
                            return self.simulator.get_state(g_idx);
                        }
                    }
                }
                TraceNode::CompOutput { component_idx, port_idx } => {
                    if let Some(&g_idx) = self.port_to_sim_gate_map.get(&(*component_idx, *port_idx)) {
                        return self.simulator.get_state(g_idx);
                    }
                }
                TraceNode::CompInput { component_idx, port_idx } => {
                    if let Some(conn) = self.connections.iter()
                        .find(|c| c.tgt_comp_id == *component_idx && c.tgt_port == *port_idx) {
                        let src_node = TraceNode::CompOutput {
                            component_idx: conn.src_comp_id,
                            port_idx: conn.src_port,
                        };
                        return self.get_node_state_at_path(&src_node, &[]);
                    }
                }
                _ => {}
            }
            return false;
        }

        let parent_path = &path[..path.len() - 1];
        let comp_id_in_parent = path[path.len() - 1];

        if let Some(bp_idx) = self.get_blueprint_idx_for_path(path) {
            let blueprint = &self.library[bp_idx];
            let driver = self.trace_local_driver(node, blueprint, path);

            match driver {
                OutputSource::DrivenByGate(g_idx) => {
                    self.simulator.get_state(g_idx)
                }
                OutputSource::PassedThrough(in_idx) => {
                    let parent_node = TraceNode::CompInput {
                        component_idx: comp_id_in_parent,
                        port_idx: in_idx,
                    };
                    self.get_node_state_at_path(&parent_node, parent_path)
                }
                OutputSource::Floating => false,
            }
        } else {
            false
        }
    }

    fn trace_local_driver(
        &self,
        node: &TraceNode,
        blueprint: &ChipBlueprint,
        path: &[usize],
    ) -> OutputSource {
        match node {
            TraceNode::CompOutput { component_idx, port_idx } => {
                let component = &blueprint.components[*component_idx];
                match component.component_type {
                    ComponentType::Nand | ComponentType::Clock => {
                        if let Some(&g_idx) = self.instance_to_sim_map.get(&(path.to_vec(), *component_idx)) {
                            OutputSource::DrivenByGate(g_idx)
                        } else {
                            OutputSource::Floating
                        }
                    }
                    ComponentType::SubChip(_) => {
                        if let Some(outputs) = self.instance_outputs.get(&(path.to_vec(), *component_idx)) {
                            if *port_idx < outputs.len() {
                                outputs[*port_idx]
                            } else {
                                OutputSource::Floating
                            }
                        } else {
                            OutputSource::Floating
                        }
                    }
                    ComponentType::Input | ComponentType::Output => OutputSource::Floating,
                }
            }
            TraceNode::CompInput { component_idx, port_idx } => {
                let conn = blueprint.connections.iter()
                    .find(|c| c.target == TargetPort::ComponentInput { component_idx: *component_idx, port_idx: *port_idx });
                
                if let Some(c) = conn {
                    match c.source {
                        SourcePort::ChipInput(i) => OutputSource::PassedThrough(i),
                        SourcePort::ComponentOutput { component_idx: src_c, port_idx: src_p } => {
                            self.trace_local_driver(&TraceNode::CompOutput { component_idx: src_c, port_idx: src_p }, blueprint, path)
                        }
                    }
                } else {
                    OutputSource::Floating
                }
            }
            TraceNode::ChipOutput(out_idx) => {
                let conn = blueprint.connections.iter()
                    .find(|c| c.target == TargetPort::ChipOutput(*out_idx));
                
                if let Some(c) = conn {
                    match c.source {
                        SourcePort::ChipInput(i) => OutputSource::PassedThrough(i),
                        SourcePort::ComponentOutput { component_idx: src_c, port_idx: src_p } => {
                            self.trace_local_driver(&TraceNode::CompOutput { component_idx: src_c, port_idx: src_p }, blueprint, path)
                        }
                    }
                } else {
                    OutputSource::Floating
                }
            }
            TraceNode::ChipInput(idx) => OutputSource::PassedThrough(*idx),
        }
    }

    pub fn to_world_space(&self, screen_pos: Vec2) -> Vec2 {
        (screen_pos - self.pan) / self.zoom
    }

    pub fn to_screen_space(&self, world_pos: Vec2) -> Vec2 {
        world_pos * self.zoom + self.pan
    }

    pub fn update(&mut self) {
        let mouse_pos_screen = mouse_position().into();
        let mouse_pos_world = self.to_world_space(mouse_pos_screen);
        
        let egui_wants_pointer = self.egui_wants_pointer;

        // Keyboard Shortcuts
        if !egui_wants_pointer {
            if is_key_pressed(KeyCode::Space) {
                self.is_playing = !self.is_playing;
            }
            if is_key_pressed(KeyCode::C) {
                self.compile();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.selected_tool = None;
                self.selected_comp_id = None;
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::S) {
                self.save_project();
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::L) {
                self.load_project();
            }
        }

        // 1. Zoom with mouse wheel
        if !egui_wants_pointer {
            let scroll = mouse_wheel().1;
            if scroll != 0.0 {
                let prev_zoom = self.zoom;
                if scroll > 0.0 {
                    self.zoom *= 1.15;
                } else {
                    self.zoom /= 1.15;
                }
                self.zoom = self.zoom.clamp(0.15, 4.0);
                
                // Pan adjustment to zoom on mouse cursor
                self.pan = mouse_pos_screen - (mouse_pos_screen - self.pan) * (self.zoom / prev_zoom);
            }
        }

        // 2. Pan with right drag
        if !egui_wants_pointer && is_mouse_button_down(MouseButton::Right) {
            let delta: Vec2 = mouse_delta_position().into();
            self.pan += delta * Vec2::new(screen_width(), screen_height());
        }

        // 3. Interactions: Left click / drag (only in main canvas)
        if !egui_wants_pointer && self.inspection_path.is_empty() {
            if is_mouse_button_pressed(MouseButton::Left) {
                // Click on a port or component
                let mut clicked_something = false;

                // Check ports first (wiring starts here)
                for comp in &self.components {
                    let (_, outputs_count) = self.get_component_ports_count(comp.comp_type);
                    
                    // Click on output ports
                    for o in 0..outputs_count {
                        let port_pos = comp.output_port_pos(o, outputs_count);
                        if port_pos.distance(mouse_pos_world) < 8.0 {
                            self.active_wire_drag = Some((comp.id, o));
                            clicked_something = true;
                            break;
                        }
                    }
                    if clicked_something { break; }
                }

                // Check clicking inside components (dragging, toggling)
                if !clicked_something {
                    let mut found_comp = None;
                    for comp in &self.components {
                        if mouse_pos_world.x >= comp.pos.x
                            && mouse_pos_world.x <= comp.pos.x + comp.width
                            && mouse_pos_world.y >= comp.pos.y
                            && mouse_pos_world.y <= comp.pos.y + comp.height
                        {
                            found_comp = Some(comp.clone());
                            break;
                        }
                    }

                    if let Some(comp) = found_comp {
                        self.selected_comp_id = Some(comp.id);
                        self.selected_annotation_idx = None;
                        if comp.comp_type == ComponentType::Input {
                            // Toggle Input state directly in simulator using mapping table without full recompile
                            if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp.id) {
                                let curr_val = self.simulator.get_state(gate_idx);
                                self.simulator.set_input(gate_idx, !curr_val);
                                let _ = self.simulator.propagate_events(5000);
                            }
                            clicked_something = true;
                        } else {
                            // Start dragging
                            self.dragging_comp_id = Some(comp.id);
                            self.drag_offset = comp.pos - mouse_pos_world;
                            clicked_something = true;
                        }
                    } else {
                        // Check if we clicked an annotation
                        let mut clicked_ann = None;
                        for (idx, ann) in self.annotations.iter().enumerate() {
                            let text_w = measure_text(&ann.text, None, 15, 1.0).width;
                            let rect = Rect::new(ann.pos.x - 5.0, ann.pos.y - 14.0, text_w + 10.0, 20.0);
                            if rect.contains(mouse_pos_world) {
                                clicked_ann = Some(idx);
                                break;
                            }
                        }
                        
                        if let Some(idx) = clicked_ann {
                            self.selected_annotation_idx = Some(idx);
                            self.dragging_annotation_idx = Some(idx);
                            self.selected_comp_id = None;
                            self.drag_offset = self.annotations[idx].pos - mouse_pos_world;
                            clicked_something = true;
                        } else {
                            // Clicked empty space
                            self.selected_comp_id = None;
                            self.selected_annotation_idx = None;
                        }
                    }
                }

                // If nothing was clicked and a tool is active, place the component or annotation
                if !clicked_something {
                    if let Some(tool) = self.selected_tool {
                        match tool {
                            ActiveTool::PlaceComponent(comp_type) => {
                                let (inputs, outputs) = self.get_component_ports_count(comp_type);
                                let max_ports = inputs.max(outputs);
                                let height = 40.0 + (max_ports as f32 * 16.0);
                                let width = match comp_type {
                                    ComponentType::SubChip(_) => 100.0,
                                    _ => 70.0,
                                };
                                
                                let label = self.get_component_label(comp_type);
                                
                                let clock_period = match comp_type {
                                    ComponentType::Clock => Some(20),
                                    _ => None,
                                };

                                let new_id = self.next_component_id;
                                let target_pos = mouse_pos_world - Vec2::new(width / 2.0, height / 2.0);
                                let snapped_pos = Vec2::new((target_pos.x / 20.0).round() * 20.0, (target_pos.y / 20.0).round() * 20.0);

                                self.components.push(VisualComponent {
                                    id: new_id,
                                    comp_type,
                                    pos: snapped_pos,
                                    width,
                                    height,
                                    label,
                                    clock_period,
                                });
                                self.selected_comp_id = Some(new_id);
                                self.next_component_id += 1;
                                self.compile();
                            }
                            ActiveTool::PlaceAnnotation => {
                                let snapped_pos = Vec2::new((mouse_pos_world.x / 20.0).round() * 20.0, (mouse_pos_world.y / 20.0).round() * 20.0);
                                self.annotations.push(TextAnnotation {
                                    text: "Double-click to edit".to_string(),
                                    pos: snapped_pos,
                                });
                                self.selected_annotation_idx = Some(self.annotations.len() - 1);
                                self.selected_comp_id = None;
                            }
                        }
                    }
                }
            } else if is_mouse_button_down(MouseButton::Left) {
                // Drag component
                if let Some(comp_id) = self.dragging_comp_id {
                    if let Some(comp) = self.components.iter_mut().find(|c| c.id == comp_id) {
                        let target_pos = mouse_pos_world + self.drag_offset;
                        comp.pos = Vec2::new((target_pos.x / 20.0).round() * 20.0, (target_pos.y / 20.0).round() * 20.0);
                    }
                }
                // Drag annotation
                if let Some(idx) = self.dragging_annotation_idx {
                    if idx < self.annotations.len() {
                        let target_pos = mouse_pos_world + self.drag_offset;
                        self.annotations[idx].pos = Vec2::new((target_pos.x / 20.0).round() * 20.0, (target_pos.y / 20.0).round() * 20.0);
                    }
                }
            } else if is_mouse_button_released(MouseButton::Left) {
                // End drag
                self.dragging_comp_id = None;
                self.dragging_annotation_idx = None;

                // Handle wiring connection release
                if let Some((src_id, src_port)) = self.active_wire_drag {
                    // Look for target input port
                    let mut connection_made = false;
                    for comp in &self.components {
                        if comp.id == src_id { continue; }
                        let (inputs_count, _) = self.get_component_ports_count(comp.comp_type);
                        
                        for i in 0..inputs_count {
                            let port_pos = comp.input_port_pos(i, inputs_count);
                            if port_pos.distance(mouse_pos_world) < 8.0 {
                                // Add wire connection, remove duplicates targeting this port
                                self.connections.retain(|c| !(c.tgt_comp_id == comp.id && c.tgt_port == i));
                                self.connections.push(VisualConnection {
                                    src_comp_id: src_id,
                                    src_port,
                                    tgt_comp_id: comp.id,
                                    tgt_port: i,
                                });
                                connection_made = true;
                                break;
                            }
                        }
                        if connection_made { break; }
                    }
                    self.active_wire_drag = None;
                    if connection_made {
                        self.compile();
                    }
                }
            }
        }

        // 4. Delete selected component (only in main canvas)
        if !egui_wants_pointer && self.inspection_path.is_empty() {
            if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
                if let Some(id) = self.selected_comp_id {
                    self.components.retain(|c| c.id != id);
                    self.connections.retain(|c| c.src_comp_id != id && c.tgt_comp_id != id);
                    self.selected_comp_id = None;
                    self.compile();
                }
            }
        }

        // 5. Run continuous simulation ticks with multi-domain clocks (batch-then-propagate)
        if self.is_playing {
            for _ in 0..self.ticks_per_frame {
                self.sim_tick_counter = self.sim_tick_counter.wrapping_add(1);
                
                for clock in &mut self.active_clocks {
                    clock.counter += 1;
                    let half_period = (clock.period / 2).max(1);
                    if clock.counter >= half_period {
                        clock.counter = 0;
                        let current_state = self.simulator.states[clock.gate_idx];
                        self.simulator.set_input(clock.gate_idx, !current_state);
                    }
                }
                
                let _ = self.simulator.propagate_events(100);
            }
        }
    }

    fn draw_manhattan_wire(&self, src_pos: Vec2, tgt_pos: Vec2, wire_state: bool) {
        let color = if wire_state {
            Color::new(0.00, 0.70, 1.00, 1.0) // electric cyan
        } else {
            Color::new(0.24, 0.27, 0.30, 1.0) // muted slate gray
        };
        let thickness = if wire_state { 2.2 } else { 1.3 } * self.zoom;
        
        if tgt_pos.x >= src_pos.x + 20.0 * self.zoom {
            // Forward route: 3 segments (H -> V -> H)
            let mid_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0;
            draw_line(src_pos.x, src_pos.y, mid_x, src_pos.y, thickness, color);
            draw_line(mid_x, src_pos.y, mid_x, tgt_pos.y, thickness, color);
            draw_line(mid_x, tgt_pos.y, tgt_pos.x, tgt_pos.y, thickness, color);
        } else {
            // Backward/feedback route: 5 segments (H-stub right -> V clearing lane -> H-stub left -> V -> H-stub right to target)
            let stub_src = src_pos.x + 20.0 * self.zoom;
            let stub_tgt = tgt_pos.x - 20.0 * self.zoom;
            
            let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0;
            if (tgt_pos.y - src_pos.y).abs() < 10.0 * self.zoom {
                mid_y += 35.0 * self.zoom;
            }
            
            draw_line(src_pos.x, src_pos.y, stub_src, src_pos.y, thickness, color);
            draw_line(stub_src, src_pos.y, stub_src, mid_y, thickness, color);
            draw_line(stub_src, mid_y, stub_tgt, mid_y, thickness, color);
            draw_line(stub_tgt, mid_y, stub_tgt, tgt_pos.y, thickness, color);
            draw_line(stub_tgt, tgt_pos.y, tgt_pos.x, tgt_pos.y, thickness, color);
        }
        
        // Draw concentric terminal circle/indicator at target
        draw_circle(tgt_pos.x, tgt_pos.y, 4.0 * self.zoom, color);
        draw_circle(tgt_pos.x, tgt_pos.y, 2.0 * self.zoom, Color::new(0.09, 0.10, 0.12, 1.0));
    }

    pub fn draw(&mut self) {
        // Clear background with dark aesthetic slate-navy
        clear_background(Color::new(0.09, 0.10, 0.12, 1.0));

        // Draw grid
        let cell_size = 40.0 * self.zoom;
        let offset_x = self.pan.x % cell_size;
        let offset_y = self.pan.y % cell_size;

        for x in (0..=(screen_width() as i32 / cell_size as i32 + 1)).map(|i| i as f32 * cell_size + offset_x) {
            draw_line(x, 0.0, x, screen_height(), 1.0, Color::new(0.16, 0.18, 0.20, 0.15));
        }
        for y in (0..=(screen_height() as i32 / cell_size as i32 + 1)).map(|i| i as f32 * cell_size + offset_y) {
            draw_line(0.0, y, screen_width(), y, 1.0, Color::new(0.16, 0.18, 0.20, 0.15));
        }

        if !self.inspection_path.is_empty() {
            self.draw_inspection_view();
            return;
        }

        // 1. Draw Wires / Connections
        for wire in &self.connections {
            let src_comp = self.components.iter().find(|c| c.id == wire.src_comp_id);
            let tgt_comp = self.components.iter().find(|c| c.id == wire.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (_, src_outputs) = self.get_component_ports_count(src.comp_type);
                let (tgt_inputs, _) = self.get_component_ports_count(tgt.comp_type);

                let src_pos = self.to_screen_space(src.output_port_pos(wire.src_port, src_outputs));
                let tgt_pos = self.to_screen_space(tgt.input_port_pos(wire.tgt_port, tgt_inputs));

                // Query state using port mapping table
                let wire_state = if let Some(&gate_idx) = self.port_to_sim_gate_map.get(&(wire.src_comp_id, wire.src_port)) {
                    self.simulator.get_state(gate_idx)
                } else if src.comp_type == ComponentType::Input {
                    if let Some(&gate_idx) = self.visual_to_sim_map.get(&src.id) {
                        self.simulator.get_state(gate_idx)
                    } else {
                        false
                    }
                } else {
                    false
                };

                self.draw_manhattan_wire(src_pos, tgt_pos, wire_state);
            }
        }

        // Draw active wire drag preview
        if let Some((src_id, src_port)) = self.active_wire_drag {
            if let Some(src) = self.components.iter().find(|c| c.id == src_id) {
                let (_, src_outputs) = self.get_component_ports_count(src.comp_type);
                let start_pos = self.to_screen_space(src.output_port_pos(src_port, src_outputs));
                let mouse_pos: Vec2 = mouse_position().into();
                
                draw_line(
                    start_pos.x,
                    start_pos.y,
                    mouse_pos.x,
                    mouse_pos.y,
                    2.0,
                    Color::new(0.5, 0.8, 1.0, 0.6), // Light blue preview wire
                );
            }
        }

        // 1.5. Draw Text Annotations
        for (idx, ann) in self.annotations.iter().enumerate() {
            let screen_pos = self.to_screen_space(ann.pos);
            let font_size = (15.0 * self.zoom).max(8.0);
            let is_selected = self.selected_annotation_idx == Some(idx);
            let color = if is_selected {
                Color::new(0.3, 0.75, 1.0, 0.95)
            } else {
                Color::new(0.7, 0.73, 0.75, 0.8)
            };
            draw_text(&ann.text, screen_pos.x, screen_pos.y, font_size, color);
            
            if is_selected {
                let text_w = measure_text(&ann.text, None, font_size as u16, 1.0).width;
                let pad = 4.0 * self.zoom;
                draw_rectangle_lines(
                    screen_pos.x - pad,
                    screen_pos.y - font_size - pad + 3.0 * self.zoom,
                    text_w + pad * 2.0,
                    font_size + pad * 2.0,
                    1.5 * self.zoom,
                    Color::new(0.3, 0.75, 1.0, 0.6),
                );
            }
        }

        // 2. Draw Components
        for comp in &self.components {
            let screen_pos = self.to_screen_space(comp.pos);
            let screen_width = comp.width * self.zoom;
            let screen_height = comp.height * self.zoom;

            // Determine body color based on component type and activity
            let is_input_active = if comp.comp_type == ComponentType::Input {
                if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp.id) {
                    self.simulator.get_state(gate_idx)
                } else {
                    false
                }
            } else if comp.comp_type == ComponentType::Output {
                // If it is output, check its input connection
                let mut output_active = false;
                if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp.id) {
                    output_active = self.simulator.get_state(gate_idx);
                }
                output_active
            } else {
                false
            };

            let bg_color = Color::new(0.12, 0.13, 0.15, 0.95);
            let border_color = Color::new(0.20, 0.23, 0.26, 1.0);

            // Draw component box
            draw_rectangle(screen_pos.x, screen_pos.y, screen_width, screen_height, bg_color);
            draw_rectangle_lines(screen_pos.x, screen_pos.y, screen_width, screen_height, 1.5 * self.zoom, border_color);

            // Draw Top Accent Stripe
            let accent_color = match comp.comp_type {
                ComponentType::Nand => Color::new(1.0, 0.55, 0.15, 1.0), // Amber orange
                ComponentType::Clock => Color::new(0.00, 0.70, 1.00, 1.0), // Electric sky blue
                ComponentType::Input | ComponentType::Output => {
                    if is_input_active {
                        Color::new(0.15, 0.85, 0.40, 1.0) // Active green
                    } else {
                        Color::new(0.35, 0.38, 0.40, 1.0) // Muted gray
                    }
                }
                ComponentType::SubChip(_) => Color::new(0.40, 0.45, 0.85, 1.0), // Royal indigo
            };
            let stripe_height = 4.0 * self.zoom;
            draw_rectangle(screen_pos.x, screen_pos.y, screen_width, stripe_height, accent_color);

            // Draw glowing selection border if selected
            if self.selected_comp_id == Some(comp.id) {
                let offset = 3.0 * self.zoom;
                draw_rectangle_lines(
                    screen_pos.x - offset,
                    screen_pos.y - offset,
                    screen_width + offset * 2.0,
                    screen_height + offset * 2.0,
                    1.5 * self.zoom,
                    Color::new(0.3, 0.75, 1.0, 0.95), // Bright glowing blue
                );
            }

            // Draw text label
            let font_size = (13.0 * self.zoom).max(6.0);
            let text_size = measure_text(&comp.label, None, font_size as u16, 1.0);
            let text_x = screen_pos.x + (screen_width - text_size.width) / 2.0;
            let text_y = screen_pos.y + (screen_height + text_size.height) / 2.0;
            
            let text_color = if is_input_active {
                Color::new(0.85, 1.0, 0.90, 1.0)
            } else {
                Color::new(0.85, 0.88, 0.90, 1.0)
            };
            draw_text(&comp.label, text_x, text_y, font_size, text_color);

            // Draw port circles
            let (inputs_count, outputs_count) = self.get_component_ports_count(comp.comp_type);
            
            // Input ports on left
            for i in 0..inputs_count {
                let port_pos = self.to_screen_space(comp.input_port_pos(i, inputs_count));
                
                let mut input_active = false;
                for wire in &self.connections {
                    if wire.tgt_comp_id == comp.id && wire.tgt_port == i {
                        if let Some(&gate_idx) = self.port_to_sim_gate_map.get(&(wire.src_comp_id, wire.src_port)) {
                            input_active = self.simulator.get_state(gate_idx);
                        } else if let Some(src_comp) = self.components.iter().find(|c| c.id == wire.src_comp_id) {
                            if src_comp.comp_type == ComponentType::Input {
                                if let Some(&gate_idx) = self.visual_to_sim_map.get(&src_comp.id) {
                                    input_active = self.simulator.get_state(gate_idx);
                                }
                            }
                        }
                    }
                }

                let port_color = if input_active {
                    Color::new(0.00, 0.70, 1.00, 1.0) // Electric cyan
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0) // Muted slate gray
                };
                draw_circle(port_pos.x, port_pos.y, 4.0 * self.zoom, port_color);
                draw_circle(port_pos.x, port_pos.y, 2.0 * self.zoom, Color::new(0.12, 0.13, 0.15, 1.0));
            }
            
            // Output ports on right
            for o in 0..outputs_count {
                let port_pos = self.to_screen_space(comp.output_port_pos(o, outputs_count));
                
                // Get output value to color output node circle
                let output_active = if let Some(&gate_idx) = self.port_to_sim_gate_map.get(&(comp.id, o)) {
                    self.simulator.get_state(gate_idx)
                } else if comp.comp_type == ComponentType::Input {
                    is_input_active
                } else {
                    false
                };

                let port_color = if output_active {
                    Color::new(0.00, 0.70, 1.00, 1.0) // Electric cyan
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0) // Muted slate gray
                };

                draw_circle(port_pos.x, port_pos.y, 4.0 * self.zoom, port_color);
                draw_circle(port_pos.x, port_pos.y, 2.0 * self.zoom, Color::new(0.12, 0.13, 0.15, 1.0));
            }

            // Draw custom port names inside sub-chip boundary boxes
            if let ComponentType::SubChip(idx) = comp.comp_type {
                if let Some(bp) = self.library.get(idx) {
                    let text_size_px = (10.0 * self.zoom).max(5.0);
                    for i in 0..inputs_count {
                        let port_pos = self.to_screen_space(comp.input_port_pos(i, inputs_count));
                        let name = bp.input_names.get(i).cloned().unwrap_or_else(|| format!("{}", i));
                        draw_text(&name, port_pos.x + 6.0 * self.zoom, port_pos.y + 3.0 * self.zoom, text_size_px, Color::new(0.5, 0.55, 0.6, 1.0));
                    }
                    for o in 0..outputs_count {
                        let port_pos = self.to_screen_space(comp.output_port_pos(o, outputs_count));
                        let name = bp.output_names.get(o).cloned().unwrap_or_else(|| format!("{}", o));
                        let text_w = measure_text(&name, None, text_size_px as u16, 1.0).width;
                        draw_text(&name, port_pos.x - 6.0 * self.zoom - text_w, port_pos.y + 3.0 * self.zoom, text_size_px, Color::new(0.5, 0.55, 0.6, 1.0));
                    }
                }
            }
        }

        // Draw instructions at top-left
        draw_text("Left Click: Place/Connect/Toggle | Drag: Move | Right Click/Del: Delete | Scroll: Zoom | Right Drag: Pan", 15.0, 20.0, 14.0, Color::new(0.6, 0.65, 0.7, 0.8));
    }

    fn draw_inspection_view(&self) {
        let (blueprint, internal_components) = match self.get_inspected_blueprint_and_components() {
            Some(res) => res,
            None => {
                draw_text("Failed to load inspection blueprint!", screen_width() / 2.0 - 150.0, screen_height() / 2.0, 20.0, RED);
                return;
            }
        };

        let inputs_count = blueprint.inputs;
        let outputs_count = blueprint.outputs;

        let border_y_start = 100.0;
        let spacing_y = 60.0;

        let get_chip_input_pos = |idx: usize| -> Vec2 {
            Vec2::new(50.0, border_y_start + idx as f32 * spacing_y)
        };
        let get_chip_output_pos = |idx: usize| -> Vec2 {
            Vec2::new(750.0, border_y_start + idx as f32 * spacing_y)
        };

        // Draw outer chip boundary labels & circles
        for i in 0..inputs_count {
            let world_pos = get_chip_input_pos(i);
            let screen_pos = self.to_screen_space(world_pos);
            let state = self.get_node_state_at_path(&TraceNode::ChipInput(i), &self.inspection_path);
            let port_color = if state {
                Color::new(0.00, 0.70, 1.00, 1.0)
            } else {
                Color::new(0.24, 0.27, 0.30, 1.0)
            };
            
            draw_circle(screen_pos.x, screen_pos.y, 6.0 * self.zoom, port_color);
            draw_circle(screen_pos.x, screen_pos.y, 3.0 * self.zoom, Color::new(0.09, 0.10, 0.12, 1.0));
            let label_text = blueprint.input_names.get(i).cloned().unwrap_or_else(|| format!("IN {}", i));
            draw_text(&label_text, screen_pos.x - 45.0 * self.zoom, screen_pos.y + 4.0 * self.zoom, (12.0 * self.zoom).max(6.0), Color::new(0.6, 0.65, 0.7, 1.0));
        }

        for j in 0..outputs_count {
            let world_pos = get_chip_output_pos(j);
            let screen_pos = self.to_screen_space(world_pos);
            let state = self.get_node_state_at_path(&TraceNode::ChipOutput(j), &self.inspection_path);
            let port_color = if state {
                Color::new(0.00, 0.70, 1.00, 1.0)
            } else {
                Color::new(0.24, 0.27, 0.30, 1.0)
            };
            
            draw_circle(screen_pos.x, screen_pos.y, 6.0 * self.zoom, port_color);
            draw_circle(screen_pos.x, screen_pos.y, 3.0 * self.zoom, Color::new(0.09, 0.10, 0.12, 1.0));
            let label_text = blueprint.output_names.get(j).cloned().unwrap_or_else(|| format!("OUT {}", j));
            draw_text(&label_text, screen_pos.x + 15.0 * self.zoom, screen_pos.y + 4.0 * self.zoom, (12.0 * self.zoom).max(6.0), Color::new(0.6, 0.65, 0.7, 1.0));
        }

        // Draw connections inside the blueprint
        for conn in &blueprint.connections {
            let src_pos = match conn.source {
                SourcePort::ChipInput(idx) => self.to_screen_space(get_chip_input_pos(idx)),
                SourcePort::ComponentOutput { component_idx, port_idx } => {
                    self.to_screen_space(self.get_bp_comp_output_port_pos(component_idx, port_idx, &internal_components))
                }
            };

            let tgt_pos = match conn.target {
                TargetPort::ChipOutput(idx) => self.to_screen_space(get_chip_output_pos(idx)),
                TargetPort::ComponentInput { component_idx, port_idx } => {
                    self.to_screen_space(self.get_bp_comp_input_port_pos(component_idx, port_idx, &internal_components))
                }
            };

            let src_node = match conn.source {
                SourcePort::ChipInput(i) => TraceNode::ChipInput(i),
                SourcePort::ComponentOutput { component_idx, port_idx } => {
                    TraceNode::CompOutput { component_idx, port_idx }
                }
            };

            let state = self.get_node_state_at_path(&src_node, &self.inspection_path);
            self.draw_manhattan_wire(src_pos, tgt_pos, state);
        }

        // Draw internal components
        for (comp_idx, comp) in internal_components.iter().enumerate() {
            let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
            let max_ports = inputs_count.max(outputs_count);
            let height = 40.0 + (max_ports as f32 * 16.0);
            let width = if let ComponentType::SubChip(_) = comp.component_type { 100.0 } else { 70.0 };

            let comp_pos = Vec2::new(comp.pos.0, comp.pos.1);
            let screen_pos = self.to_screen_space(comp_pos);
            let screen_width = width * self.zoom;
            let screen_height = height * self.zoom;

            let bg_color = Color::new(0.12, 0.13, 0.15, 0.95);
            let border_color = Color::new(0.20, 0.23, 0.26, 1.0);

            draw_rectangle(screen_pos.x, screen_pos.y, screen_width, screen_height, bg_color);
            draw_rectangle_lines(screen_pos.x, screen_pos.y, screen_width, screen_height, 1.5 * self.zoom, border_color);

            // Draw Top Accent Stripe
            let accent_color = match comp.component_type {
                ComponentType::Nand => Color::new(1.0, 0.55, 0.15, 1.0),
                ComponentType::Clock => Color::new(0.00, 0.70, 1.00, 1.0),
                ComponentType::Input | ComponentType::Output => {
                    // Check output port 0 to see if it is active in the flat simulation state path
                    let state = self.get_node_state_at_path(
                        &TraceNode::CompOutput { component_idx: comp_idx, port_idx: 0 },
                        &self.inspection_path
                    );
                    if state {
                        Color::new(0.15, 0.85, 0.40, 1.0)
                    } else {
                        Color::new(0.35, 0.38, 0.40, 1.0)
                    }
                }
                ComponentType::SubChip(_) => Color::new(0.40, 0.45, 0.85, 1.0),
            };
            let stripe_height = 4.0 * self.zoom;
            draw_rectangle(screen_pos.x, screen_pos.y, screen_width, stripe_height, accent_color);

            // Draw label
            let label = self.get_component_label(comp.component_type);
            let font_size = (13.0 * self.zoom).max(6.0);
            let text_size = measure_text(&label, None, font_size as u16, 1.0);
            let text_x = screen_pos.x + (screen_width - text_size.width) / 2.0;
            let text_y = screen_pos.y + (screen_height + text_size.height) / 2.0;
            draw_text(&label, text_x, text_y, font_size, Color::new(0.85, 0.88, 0.90, 1.0));

            // Draw input circles
            for i in 0..inputs_count {
                let w_pos = self.get_bp_comp_input_port_pos(comp_idx, i, &internal_components);
                let port_pos = self.to_screen_space(w_pos);
                let state = self.get_node_state_at_path(&TraceNode::CompInput { component_idx: comp_idx, port_idx: i }, &self.inspection_path);
                let port_color = if state {
                    Color::new(0.00, 0.70, 1.00, 1.0)
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0)
                };
                draw_circle(port_pos.x, port_pos.y, 4.0 * self.zoom, port_color);
                draw_circle(port_pos.x, port_pos.y, 2.0 * self.zoom, Color::new(0.12, 0.13, 0.15, 1.0));
            }

            // Draw output circles
            for o in 0..outputs_count {
                let w_pos = self.get_bp_comp_output_port_pos(comp_idx, o, &internal_components);
                let port_pos = self.to_screen_space(w_pos);

                let state = self.get_node_state_at_path(&TraceNode::CompOutput { component_idx: comp_idx, port_idx: o }, &self.inspection_path);
                let port_color = if state {
                    Color::new(0.00, 0.70, 1.00, 1.0)
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0)
                };

                draw_circle(port_pos.x, port_pos.y, 4.0 * self.zoom, port_color);
                draw_circle(port_pos.x, port_pos.y, 2.0 * self.zoom, Color::new(0.12, 0.13, 0.15, 1.0));
            }
        }

        // Draw info overlay
        let title = format!("LOOK INSIDE: {}", blueprint.name);
        draw_text(&title, 15.0, 20.0, 16.0, Color::new(0.3, 0.75, 1.0, 0.95));
        draw_text("Inspection Mode (Read-Only) | Drag mouse wheel/right-click to pan | Scroll to zoom", 15.0, 40.0, 12.0, Color::new(0.5, 0.55, 0.6, 0.8));
    }

    fn get_bp_comp_input_port_pos(&self, comp_idx: usize, port_idx: usize, comps: &[Component]) -> Vec2 {
        let comp = &comps[comp_idx];
        let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
        let max_ports = inputs_count.max(outputs_count);
        let height = 40.0 + (max_ports as f32 * 16.0);
        
        let x = comp.pos.0;
        let spacing = height / (inputs_count + 1) as f32;
        let y = comp.pos.1 + spacing * (port_idx + 1) as f32;
        Vec2::new(x, y)
    }

    fn get_bp_comp_output_port_pos(&self, comp_idx: usize, port_idx: usize, comps: &[Component]) -> Vec2 {
        let comp = &comps[comp_idx];
        let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
        let max_ports = inputs_count.max(outputs_count);
        let height = 40.0 + (max_ports as f32 * 16.0);
        let width = if let ComponentType::SubChip(_) = comp.component_type { 100.0 } else { 70.0 };
        
        let x = comp.pos.0 + width;
        let spacing = height / (outputs_count + 1) as f32;
        let y = comp.pos.1 + spacing * (port_idx + 1) as f32;
        Vec2::new(x, y)
    }

    pub fn draw_gui(&mut self) {
        let mut egui_wants_pointer = false;
        egui_macroquad::ui(|ctx| {
            egui_wants_pointer = ctx.wants_pointer_input() || ctx.wants_keyboard_input();
            // Dark elegant theme styling overrides
            let mut style = (*ctx.style()).clone();
            style.visuals.dark_mode = true;
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(32, 60, 48);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(36, 42, 45);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(18, 20, 22);
            ctx.set_style(style);

            // 1. Sidebar catalog panel
            if self.inspection_path.is_empty() {
                egui::SidePanel::left("parts_catalog")
                    .resizable(false)
                    .default_width(180.0)
                    .show(ctx, |ui| {
                        ui.add_space(10.0);
                        ui.heading("Parts Catalog");
                        ui.separator();
                        ui.add_space(5.0);

                        // Library Primitives
                        ui.label("Primitives");
                        if ui.selectable_label(self.selected_tool == Some(ActiveTool::PlaceComponent(ComponentType::Input)), "Input Pin").clicked() {
                            self.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Input));
                        }
                        if ui.selectable_label(self.selected_tool == Some(ActiveTool::PlaceComponent(ComponentType::Output)), "Output Pin").clicked() {
                            self.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Output));
                        }
                        if ui.selectable_label(self.selected_tool == Some(ActiveTool::PlaceComponent(ComponentType::Nand)), "NAND Gate").clicked() {
                            self.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Nand));
                        }
                        if ui.selectable_label(self.selected_tool == Some(ActiveTool::PlaceComponent(ComponentType::Clock)), "Clock Input").clicked() {
                            self.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Clock));
                        }
                        if ui.selectable_label(self.selected_tool == Some(ActiveTool::PlaceAnnotation), "Text Annotation").clicked() {
                            self.selected_tool = Some(ActiveTool::PlaceAnnotation);
                        }
                        
                        if ui.button("Clear Selection").clicked() {
                            self.selected_tool = None;
                        }

                        ui.add_space(20.0);
                        ui.label("Custom Chips");
                        ui.separator();

                        for (idx, bp) in self.library.iter().enumerate() {
                            let is_sel = self.selected_tool == Some(ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            if ui.selectable_label(is_sel, &bp.name).clicked() {
                                self.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::SubChip(idx)));
                            }
                        }
                    });
            }

            // 2. Control Toolbar (Top Panel)
            egui::TopBottomPanel::top("control_bar").show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if !self.inspection_path.is_empty() {
                        if ui.button("← Exit Inspection").clicked() {
                            self.inspection_path.pop();
                            self.selected_comp_id = None;
                        }
                        ui.separator();
                    }
                    ui.heading("Digital Logic Sim");
                    ui.add_space(30.0);

                    // Simulation Ticker controls
                    if ui.button(if self.is_playing { "Pause" } else { "Play" }).clicked() {
                        self.is_playing = !self.is_playing;
                    }

                    if ui.button("Step Tick").clicked() {
                        let _ = self.simulator.propagate_events(50);
                    }

                    ui.add_space(20.0);
                    ui.label("Sim Speed:");
                    ui.add(egui::Slider::new(&mut self.ticks_per_frame, 1..=500).text("ticks/frame"));

                    ui.add_space(20.0);
                    if ui.button("Recompile Graph").clicked() {
                        self.compile();
                    }

                    if ui.button("Clear Canvas").clicked() {
                        self.components.clear();
                        self.connections.clear();
                        self.compile();
                    }
                    ui.separator();
                    if ui.button("💾 Save").clicked() {
                        self.save_project();
                    }
                    if ui.button("📂 Load").clicked() {
                        self.load_project();
                    }
                });
            });

            // 3. Packaging Chip Panel (Right Panel)
            egui::SidePanel::right("package_panel")
                .resizable(false)
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.add_space(10.0);

                    if !self.inspection_path.is_empty() {
                        if let Some((blueprint, _)) = self.get_inspected_blueprint_and_components() {
                            ui.heading("Inspecting Block");
                            ui.colored_label(egui::Color32::from_rgb(0, 180, 255), &blueprint.name);
                            ui.separator();
                            ui.add_space(10.0);
                            
                            ui.label(format!("Inputs: {}", blueprint.inputs));
                            for (i, name) in blueprint.input_names.iter().enumerate() {
                                ui.small(format!("  Pin {}: {}", i, name));
                            }
                            
                            ui.add_space(10.0);
                            ui.label(format!("Outputs: {}", blueprint.outputs));
                            for (o, name) in blueprint.output_names.iter().enumerate() {
                                ui.small(format!("  Pin {}: {}", o, name));
                            }
                            
                            ui.add_space(10.0);
                            ui.label(format!("Internal Gates: {}", blueprint.components.len()));
                            
                            ui.add_space(30.0);
                            if ui.button("← Exit Inspection").clicked() {
                                self.inspection_path.pop();
                                self.selected_comp_id = None;
                            }
                        }
                    } else {
                         // If a component is selected, allow inspecting & editing properties
                         if let Some(sel_id) = self.selected_comp_id {
                             let mut comp_opt = None;
                             for c in &mut self.components {
                                 if c.id == sel_id {
                                     comp_opt = Some(c);
                                     break;
                                 }
                             }
                             
                             if let Some(comp) = comp_opt {
                                 ui.heading("Selected Component");
                                 ui.label(format!("ID: {}", comp.id));
                                 ui.label(format!("Type: {:?}", comp.comp_type));
                                 
                                 ui.add_space(5.0);
                                 ui.label("Label / Name:");
                                 let mut label = comp.label.clone();
                                 if ui.text_edit_singleline(&mut label).changed() {
                                     comp.label = label;
                                 }
                                 
                                 if let ComponentType::SubChip(_) = comp.comp_type {
                                     ui.add_space(5.0);
                                     if ui.button("🔍 Look Inside").clicked() {
                                         self.inspection_path.push(comp.id);
                                         self.selected_comp_id = None;
                                     }
                                 }
                                 
                                 if let ComponentType::Clock = comp.comp_type {
                                     ui.add_space(5.0);
                                     let mut period = comp.clock_period.unwrap_or(20);
                                     ui.label("Clock Period (ticks):");
                                     if ui.add(egui::Slider::new(&mut period, 2..=1000).text("ticks")).changed() {
                                         comp.clock_period = Some(period);
                                         // Directly update the compiled active_clocks array!
                                         if let Some(active_clk) = self.active_clocks.iter_mut().find(|ac| ac.visual_id == Some(comp.id)) {
                                             active_clk.period = period;
                                         }
                                     }
                                 }
                                 
                                 ui.separator();
                                 ui.add_space(15.0);
                             }
                         }

                         if let Some(idx) = self.selected_annotation_idx {
                             if let Some(ann) = self.annotations.get_mut(idx) {
                                 ui.heading("Selected Text Label");
                                 ui.label("Text Content:");
                                 ui.text_edit_multiline(&mut ann.text);
                                 
                                 ui.separator();
                                 ui.add_space(15.0);
                             }
                         }

                        ui.heading("Package Chip");
                        ui.separator();
                        ui.add_space(5.0);
                        
                        ui.label("Create a reusable custom block out of your current canvas layout.");
                        ui.add_space(10.0);

                        ui.label("Chip Name:");
                        ui.text_edit_singleline(&mut self.chip_name_input);

                        ui.add_space(15.0);

                        if ui.button("Compile & Save to Catalog").clicked() {
                            if let Some(new_bp) = self.package_current_canvas() {
                                self.library.push(new_bp);
                                self.components.clear();
                                self.connections.clear();
                                self.compile();
                            }
                        }
                    }
                });
        });
        self.egui_wants_pointer = egui_wants_pointer;
    }

    /// Translates the current canvas components and connections into a reusable ChipBlueprint
    fn package_current_canvas(&self) -> Option<ChipBlueprint> {
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct ProjectFile {
    pub library: Vec<ChipBlueprint>,
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub next_component_id: usize,
    pub annotations: Vec<TextAnnotation>,
}

impl Editor {
    pub fn save_project(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Logic Simulator Projects", &["json"])
            .set_directory(".")
            .save_file()
        {
            let project = ProjectFile {
                library: self.library.clone(),
                components: self.components.clone(),
                connections: self.connections.clone(),
                next_component_id: self.next_component_id,
                annotations: self.annotations.clone(),
            };
            if let Ok(serialized) = serde_json::to_string_pretty(&project) {
                if let Ok(mut file) = std::fs::File::create(path) {
                    use std::io::Write;
                    let _ = file.write_all(serialized.as_bytes());
                }
            }
        }
    }

    pub fn load_project(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Logic Simulator Projects", &["json"])
            .set_directory(".")
            .pick_file()
        {
            if let Ok(mut file) = std::fs::File::open(path) {
                let mut contents = String::new();
                use std::io::Read;
                if file.read_to_string(&mut contents).is_ok() {
                    if let Ok(project) = serde_json::from_str::<ProjectFile>(&contents) {
                        self.library = project.library;
                        self.components = project.components;
                        self.connections = project.connections;
                        self.next_component_id = project.next_component_id;
                        self.annotations = project.annotations;
                        self.selected_comp_id = None;
                        self.selected_annotation_idx = None;
                        self.inspection_path.clear();
                        self.compile();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod editor_tests {
    use super::*;

    #[test]
    fn test_custom_port_naming_collision() {
        let mut editor = Editor::new();
        editor.library.clear();
        
        editor.components.push(VisualComponent {
            id: 1,
            comp_type: ComponentType::Input,
            pos: Vec2::new(0.0, 0.0),
            width: 70.0,
            height: 40.0,
            label: "X".to_string(),
            clock_period: None,
        });
        editor.components.push(VisualComponent {
            id: 2,
            comp_type: ComponentType::Input,
            pos: Vec2::new(0.0, 50.0),
            width: 70.0,
            height: 40.0,
            label: "X".to_string(),
            clock_period: None,
        });
        editor.components.push(VisualComponent {
            id: 3,
            comp_type: ComponentType::Input,
            pos: Vec2::new(0.0, 100.0),
            width: 70.0,
            height: 40.0,
            label: "IN".to_string(),
            clock_period: None,
        });

        editor.components.push(VisualComponent {
            id: 4,
            comp_type: ComponentType::Output,
            pos: Vec2::new(200.0, 0.0),
            width: 70.0,
            height: 40.0,
            label: "Y".to_string(),
            clock_period: None,
        });
        editor.components.push(VisualComponent {
            id: 5,
            comp_type: ComponentType::Output,
            pos: Vec2::new(200.0, 50.0),
            width: 70.0,
            height: 40.0,
            label: "Y".to_string(),
            clock_period: None,
        });

        editor.components.push(VisualComponent {
            id: 6,
            comp_type: ComponentType::Nand,
            pos: Vec2::new(100.0, 25.0),
            width: 70.0,
            height: 40.0,
            label: "NAND".to_string(),
            clock_period: None,
        });

        let bp = editor.package_current_canvas().expect("Failed to package canvas");
        
        assert_eq!(bp.input_names.len(), 3);
        assert_eq!(bp.input_names[0], "X");
        assert_eq!(bp.input_names[1], "X_1");
        assert_eq!(bp.input_names[2], "IN_2");

        assert_eq!(bp.output_names.len(), 2);
        assert_eq!(bp.output_names[0], "Y");
        assert_eq!(bp.output_names[1], "Y_1");
    }
}
