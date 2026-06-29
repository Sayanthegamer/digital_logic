pub mod types;
mod canvas;
mod input;
mod drawing;
mod inspection;
mod gui;
mod persistence;

#[cfg(test)]
mod tests;

// Re-export public types
pub use types::*;
pub use persistence::ProjectFile;

use crate::engine::{
    ChipBlueprint, Component, ComponentType, Connection, CompiledClock,
    OutputSource, Simulator, SourcePort, TargetPort,
};
use macroquad::prelude::*;
use std::collections::HashMap;

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
    pub last_mouse_pos: Vec2,
    
    // Interaction States
    pub selected_tool: Option<ActiveTool>,
    pub active_wire_drag: Option<(usize, usize)>, // (src_comp_id, src_port_idx)
    pub dragging_comp_id: Option<usize>,
    pub drag_offset: Vec2,
    pub drag_dist_pixels: f32,
    
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

    // Error Reporting
    pub propagation_error: Option<String>,
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
            last_mouse_pos: Vec2::ZERO,
            selected_tool: None,
            active_wire_drag: None,
            dragging_comp_id: None,
            drag_offset: Vec2::ZERO,
            drag_dist_pixels: 0.0,
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
            propagation_error: None,
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
}
