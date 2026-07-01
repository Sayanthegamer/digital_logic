mod canvas;
mod drawing;
mod drawing_shapes;
mod drawing_wires;
pub mod gui;
mod input;
mod inspection_logic;
mod inspection_ui;
mod persistence;
mod ui_catalog;
mod ui_properties;
pub mod theme;
pub mod types;
pub mod state;
mod history;

#[cfg(test)]
mod tests;

// Re-export public types
pub use persistence::ProjectFile;
pub use types::*;
use state::{CanvasState, EngineState, UiState};

use crate::engine::{
    ChipBlueprint, Component, ComponentType, Connection,
    SourcePort, TargetPort,
};
use macroquad::prelude::*;

pub struct Editor {
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub next_component_id: usize,
    pub annotations: Vec<TextAnnotation>,
    
    // Crisp Vector Font
    pub font: Option<Font>,

    // Sub-states
    pub engine: EngineState,
    pub ui: UiState,
    pub canvas: CanvasState,
    pub history: state::HistoryManager,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor {
    pub fn new() -> Self {
        #[cfg(not(test))]
        let font = {
            let font_bytes = include_bytes!("../Inter-Regular.ttf");
            Some(load_ttf_font_from_bytes(font_bytes).expect("Failed to load embedded Inter font"))
        };
        #[cfg(test)]
        let font = None;

        let mut editor = Self {
            components: Vec::new(),
            connections: Vec::new(),
            next_component_id: 1,
            annotations: Vec::new(),
            font,
            engine: EngineState::default(),
            ui: UiState::default(),
            canvas: CanvasState::default(),
            history: state::HistoryManager::default(),
        };

        // Add some basic chips to the library as initial examples
        editor.setup_default_library();
        editor.compile();
        editor
    }

    fn setup_default_library(&mut self) {
        // 0: AND (2 NANDs)
        self.engine.library.push(ChipBlueprint {
            name: "AND".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 150.0),
                    clock_period: None,
                }, // Comp 0
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 150.0),
                    clock_period: None,
                }, // Comp 1
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });

        // 1: OR (3 NANDs)
        self.engine.library.push(ChipBlueprint {
            name: "OR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 100.0),
                    clock_period: None,
                }, // A inverter
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 250.0),
                    clock_period: None,
                }, // B inverter
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 175.0),
                    clock_period: None,
                }, // final NAND
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });

        // 2: XOR (4 NANDs)
        self.engine.library.push(ChipBlueprint {
            name: "XOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 175.0),
                    clock_period: None,
                }, // NAND 0: Shared inputs
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 100.0),
                    clock_period: None,
                }, // NAND 1: Top branch
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 250.0),
                    clock_period: None,
                }, // NAND 2: Bottom branch
                Component {
                    component_type: ComponentType::Nand,
                    pos: (500.0, 175.0),
                    clock_period: None,
                }, // NAND 3: Combiner
            ],
            connections: vec![
                // A -> NAND 0 input 0, and NAND 1 input 0
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                },
                // B -> NAND 0 input 1, and NAND 2 input 1
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 1,
                    },
                },
                // NAND 0 output -> NAND 1 input 1, and NAND 2 input 0
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 1,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                },
                // NAND 1 output -> NAND 3 input 0
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 1,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
                // NAND 2 output -> NAND 3 input 1
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 2,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 1,
                    },
                },
                // NAND 3 output -> Chip Output 0
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });

        // 3: NOT (1 NAND)
        self.engine.library.push(ChipBlueprint {
            name: "NOT".to_string(),
            inputs: 1,
            outputs: 1,
            input_names: vec!["IN".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 150.0),
                    clock_period: None,
                }, // NAND 0
            ],
            connections: vec![
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ChipInput(0),
                    target: TargetPort::ComponentInput {
                        component_idx: 0,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 0,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        });

        // 4: NOR (4 NANDs)
        self.engine.library.push(ChipBlueprint {
            name: "NOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 100.0),
                    clock_period: None,
                }, // NAND 0 (NOT A)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 250.0),
                    clock_period: None,
                }, // NAND 1 (NOT B)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 175.0),
                    clock_period: None,
                }, // NAND 2 (OR)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (550.0, 175.0),
                    clock_period: None,
                }, // NAND 3 (NOT)
            ],
            connections: vec![
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 3, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 3, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 3, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
            ],
        });

        // 5: XNOR (5 NANDs)
        self.engine.library.push(ChipBlueprint {
            name: "XNOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component { component_type: ComponentType::Nand, pos: (200.0, 175.0), clock_period: None }, // NAND 0
                Component { component_type: ComponentType::Nand, pos: (350.0, 100.0), clock_period: None }, // NAND 1
                Component { component_type: ComponentType::Nand, pos: (350.0, 250.0), clock_period: None }, // NAND 2
                Component { component_type: ComponentType::Nand, pos: (500.0, 175.0), clock_period: None }, // NAND 3 (XOR)
                Component { component_type: ComponentType::Nand, pos: (650.0, 175.0), clock_period: None }, // NAND 4 (NOT)
            ],
            connections: vec![
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(0), target: TargetPort::ComponentInput { component_idx: 1, port_idx: 0 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 0, port_idx: 1 } },
                Connection { source: SourcePort::ChipInput(1), target: TargetPort::ComponentInput { component_idx: 2, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 1, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 0, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 2, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 1, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 3, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 2, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 3, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 3, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 4, port_idx: 0 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 3, port_idx: 0 }, target: TargetPort::ComponentInput { component_idx: 4, port_idx: 1 } },
                Connection { source: SourcePort::ComponentOutput { component_idx: 4, port_idx: 0 }, target: TargetPort::ChipOutput(0) },
            ],
        });
    }

    pub fn remap_library_chip(&mut self, from: usize, to: usize) {
        let len = self.engine.library.len();
        if from == to || from >= len || to >= len { return; }
        
        let mut new_order: Vec<usize> = (0..len).collect();
        let item = new_order.remove(from);
        new_order.insert(to, item);

        let mut map = vec![0; len];
        for (new_idx, &old_idx) in new_order.iter().enumerate() {
            map[old_idx] = new_idx;
        }

        let mut new_library = Vec::with_capacity(len);
        for &old_idx in &new_order {
            new_library.push(self.engine.library[old_idx].clone());
        }
        self.engine.library = new_library;

        // Remap in main canvas
        for comp in &mut self.components {
            if let ComponentType::SubChip(ref mut idx) = comp.comp_type {
                *idx = map[*idx];
            }
        }
        
        // Remap in library blueprints
        for bp in &mut self.engine.library {
            for comp in &mut bp.components {
                if let ComponentType::SubChip(ref mut idx) = comp.component_type {
                    *idx = map[*idx];
                }
            }
        }
        
        // Remap selected_tool if it points to a SubChip
        if let Some(crate::editor::types::ActiveTool::PlaceComponent(ComponentType::SubChip(ref mut idx))) = self.canvas.selected_tool {
            *idx = map[*idx];
        }
    }
}
