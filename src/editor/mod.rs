mod canvas;
pub mod circuit_model;
pub mod color_coding;
pub mod constants;
mod drawing;
mod drawing_shapes;
mod drawing_wires;
pub mod global_library;
pub mod gui;
mod history;
mod input;
mod input_context_menu;
mod input_delete;
mod input_down;
mod input_hover;
mod input_interactions;
mod input_keyboard;
mod input_navigation;
mod input_press;
mod input_release;
mod input_simulation;
mod inspection_logic;
mod inspection_ui;
mod layout;
pub mod math;
mod persistence;
mod spatial_hash;
pub mod state;
mod stress_test;
pub mod svg_exporter;
pub mod theme;
pub mod types;
mod ui_catalog;
mod ui_global_library;
mod ui_main_menu;
mod ui_properties;
mod wire_junctions;

#[cfg(test)]
mod tests;

// Re-export public types
pub use math::{Color, GRAY, GREEN, RED, Rect, Vec2, vec2};
pub use persistence::ProjectFile;
use state::{CanvasState, EngineState, UiState};
pub use types::*;

use crate::engine::{ChipBlueprint, Component, ComponentType, Connection, SourcePort, TargetPort};

pub struct Editor {
    pub circuit: circuit_model::CircuitModel,

    // Sub-states
    pub engine: EngineState,
    pub ui: UiState,
    pub canvas: CanvasState,
    pub history: state::HistoryManager,

    // Global chip library (persisted across projects)
    pub global_library: global_library::GlobalLibrary,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new()
    }
}

fn remap_visual_components_and_connections(
    components: &mut Vec<types::VisualComponent>,
    connections: &mut Vec<types::VisualConnection>,
    map: &[Option<usize>],
) {
    let mut removed_ids = std::collections::HashSet::new();
    components.retain_mut(|comp| {
        if let ComponentType::SubChip(ref mut idx) = comp.comp_type {
            match map.get(*idx).copied().flatten() {
                Some(new_idx) => {
                    *idx = new_idx;
                    true
                }
                None => {
                    removed_ids.insert(comp.id);
                    false
                }
            }
        } else {
            true
        }
    });
    if !removed_ids.is_empty() {
        connections.retain(|conn| {
            !removed_ids.contains(&conn.src_comp_id) && !removed_ids.contains(&conn.tgt_comp_id)
        });
    }
}

fn remap_blueprint_subchips(bp: &mut ChipBlueprint, map: &[Option<usize>]) {
    for comp in &mut bp.components {
        if let ComponentType::SubChip(ref mut idx) = comp.component_type
            && let Some(Some(new_idx)) = map.get(*idx)
        {
            *idx = *new_idx;
        }
    }
}

impl Editor {
    pub fn new() -> Self {
        let mut editor = Self {
            circuit: circuit_model::CircuitModel::default(),
            engine: EngineState::default(),
            ui: UiState::default(),
            canvas: CanvasState::default(),
            history: state::HistoryManager::default(),
            global_library: global_library::load_global_library(),
        };

        // If global library has chips, use them; otherwise set up defaults and save
        if editor.global_library.to_flat_list().is_empty() {
            editor.setup_default_library();
            // Save default chips to global library
            editor
                .global_library
                .import_from_project(&editor.engine.library);
            global_library::save_global_library(&editor.global_library);
        } else {
            editor.engine.library = editor.global_library.to_flat_list();
        }
        editor.rebuild_comp_map();
        editor.compile();
        editor
    }

    #[inline(always)]
    pub fn screen_width(&self) -> f32 {
        self.ui.screen_size.0.max(1.0)
    }

    #[inline(always)]
    pub fn screen_height(&self) -> f32 {
        self.ui.screen_size.1.max(1.0)
    }

    /// Deterministic text width/height measurement (used for layout & hit-testing).
    #[inline(always)]
    pub fn measure_text_dims(&self, text: &str, font_size: f32) -> (f32, f32) {
        let width = (text.chars().count() as f32) * font_size * 0.56;
        (width, font_size)
    }

    /// Executes one full frame of the eframe application (UI panels, input, simulation ticks, and 2D canvas rendering).
    pub fn step_frame(&mut self, ctx: &egui::Context) {
        let screen_rect = ctx.screen_rect();
        self.ui.screen_size = (screen_rect.width(), screen_rect.height());
        let (time, dt) = ctx.input(|i| (i.time, i.stable_dt.clamp(1e-4, 0.25)));
        self.ui.elapsed_time = time;
        self.ui.frame_dt = dt;
        self.ui.fps = (1.0 / dt).round() as i32;

        // 1. Draw full-screen 2D logic canvas in the background layer
        let bg_painter = ctx.layer_painter(egui::LayerId::background());
        self.draw_on_painter(&bg_painter);

        // 2. Process and render egui panels & overlays
        self.draw_gui(ctx);

        // 3. Snapshot frame input from egui and update editor/simulation state
        self.canvas.frame_input = state::FrameInput::from_egui(ctx, self.canvas.last_mouse_pos);
        self.update();

        // 4. Flush any pending viewport commands (fullscreen / resolution changes)
        for cmd in self.ui.pending_viewport_commands.drain(..) {
            ctx.send_viewport_cmd(cmd);
        }
    }

    pub fn rebuild_comp_map(&mut self) {
        self.circuit.comp_map.clear();
        for (idx, comp) in self.circuit.components.iter().enumerate() {
            self.circuit.comp_map.insert(comp.id, idx);
        }
    }

    pub fn get_component_idx(&self, id: usize) -> Option<usize> {
        self.circuit.comp_map.get(&id).copied()
    }

    pub fn get_component(&self, id: usize) -> Option<&VisualComponent> {
        let idx = self.get_component_idx(id)?;
        self.circuit.components.get(idx)
    }

    pub fn get_component_mut(&mut self, id: usize) -> Option<&mut VisualComponent> {
        let idx = self.get_component_idx(id)?;
        self.circuit.components.get_mut(idx)
    }

    pub fn auto_arrange_components(&mut self) {
        if self.circuit.components.is_empty() {
            return;
        }
        self.push_history_snapshot();
        for i in 0..self.circuit.components.len() {
            let comp_type = self.circuit.components[i].comp_type;
            let bus_w = self.circuit.components[i].bus_width;
            let (w, h) = self.get_component_dimensions_with_width(comp_type, bus_w);
            self.circuit.components[i].width = self.circuit.components[i].width.max(w);
            self.circuit.components[i].height = self.circuit.components[i].height.max(h);
        }
        layout::auto_arrange(&mut self.circuit.components, &self.circuit.connections);
        self.circuit.wire_offsets.clear();
        self.circuit.wire_nudges.clear();
        self.recompute_wire_offsets(None);
        self.rebuild_spatial_grid();
    }

    fn setup_default_library(&mut self) {
        self.engine.library.push(Self::create_and_blueprint());
        self.engine.library.push(Self::create_or_blueprint());
        self.engine.library.push(Self::create_xor_blueprint());
        self.engine.library.push(Self::create_not_blueprint());
        self.engine.library.push(Self::create_nor_blueprint());
        self.engine.library.push(Self::create_xnor_blueprint());
    }

    fn create_and_blueprint() -> ChipBlueprint {
        ChipBlueprint {
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
                    bus_width: None,
                }, // Comp 0
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 150.0),
                    clock_period: None,
                    bus_width: None,
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
        }
    }

    fn create_or_blueprint() -> ChipBlueprint {
        ChipBlueprint {
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
                    bus_width: None,
                }, // A inverter
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 250.0),
                    clock_period: None,
                    bus_width: None,
                }, // B inverter
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 175.0),
                    clock_period: None,
                    bus_width: None,
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
        }
    }

    fn create_xor_blueprint() -> ChipBlueprint {
        ChipBlueprint {
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
                    bus_width: None,
                }, // NAND 0: Shared inputs
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 1: Top branch
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 250.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 2: Bottom branch
                Component {
                    component_type: ComponentType::Nand,
                    pos: (500.0, 175.0),
                    clock_period: None,
                    bus_width: None,
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
        }
    }

    fn create_not_blueprint() -> ChipBlueprint {
        ChipBlueprint {
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
                    bus_width: None,
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
        }
    }

    fn create_nor_blueprint() -> ChipBlueprint {
        ChipBlueprint {
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
                    bus_width: None,
                }, // NAND 0 (NOT A)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 250.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 1 (NOT B)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (400.0, 175.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 2 (OR)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (550.0, 175.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 3 (NOT)
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
                    target: TargetPort::ComponentInput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
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
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        }
    }

    fn create_xnor_blueprint() -> ChipBlueprint {
        ChipBlueprint {
            name: "XNOR".to_string(),
            inputs: 2,
            outputs: 1,
            input_names: vec!["A".to_string(), "B".to_string()],
            output_names: vec!["OUT".to_string()],
            components: vec![
                Component {
                    component_type: ComponentType::Nand,
                    pos: (200.0, 175.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 0
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 100.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 1
                Component {
                    component_type: ComponentType::Nand,
                    pos: (350.0, 250.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 2
                Component {
                    component_type: ComponentType::Nand,
                    pos: (500.0, 175.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 3 (XOR)
                Component {
                    component_type: ComponentType::Nand,
                    pos: (650.0, 175.0),
                    clock_period: None,
                    bus_width: None,
                }, // NAND 4 (NOT)
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
                        component_idx: 1,
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
                    source: SourcePort::ChipInput(1),
                    target: TargetPort::ComponentInput {
                        component_idx: 2,
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
                        component_idx: 3,
                        port_idx: 0,
                    },
                },
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
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 4,
                        port_idx: 0,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 3,
                        port_idx: 0,
                    },
                    target: TargetPort::ComponentInput {
                        component_idx: 4,
                        port_idx: 1,
                    },
                },
                Connection {
                    source: SourcePort::ComponentOutput {
                        component_idx: 4,
                        port_idx: 0,
                    },
                    target: TargetPort::ChipOutput(0),
                },
            ],
        }
    }

    /// Centralised remapping helper whenever library chip indices move or are deleted.
    /// `map[old_idx] = Some(new_idx)` if retained, or `None` if deleted.
    pub fn apply_library_index_map(&mut self, map: &[Option<usize>]) {
        // 1. Remap main canvas
        remap_visual_components_and_connections(
            &mut self.circuit.components,
            &mut self.circuit.connections,
            map,
        );

        // 2. Remap stashed main canvas if currently editing a blueprint
        if let Some(ref mut stashed) = self.canvas.stashed_main_canvas {
            remap_visual_components_and_connections(
                &mut stashed.components,
                &mut stashed.connections,
                map,
            );
        }

        // 3. Remap undo and redo history stacks
        for snap in &mut self.history.undo_stack {
            remap_visual_components_and_connections(
                &mut snap.components,
                &mut snap.connections,
                map,
            );
        }
        for snap in &mut self.history.redo_stack {
            remap_visual_components_and_connections(
                &mut snap.components,
                &mut snap.connections,
                map,
            );
        }

        // 4. Remap internal SubChip references inside engine.library and global_library
        for bp in &mut self.engine.library {
            remap_blueprint_subchips(bp, map);
        }
        for bp in &mut self.global_library.ungrouped {
            remap_blueprint_subchips(bp, map);
        }
        for folder in &mut self.global_library.folders {
            for bp in &mut folder.chips {
                remap_blueprint_subchips(bp, map);
            }
        }

        // 5. Remap selected_tool if it points to a SubChip
        if let Some(crate::editor::types::ActiveTool::PlaceComponent(ComponentType::SubChip(
            old_idx,
        ))) = self.canvas.selected_tool
        {
            match map.get(old_idx).copied().flatten() {
                Some(new_idx) => {
                    self.canvas.selected_tool =
                        Some(crate::editor::types::ActiveTool::PlaceComponent(
                            ComponentType::SubChip(new_idx),
                        ));
                }
                None => {
                    self.canvas.selected_tool = None;
                }
            }
        }

        // 6. Remap editing_target if editing a library chip
        if let state::EditingTarget::LibraryChip(old_idx) = self.canvas.editing_target {
            if let Some(Some(new_idx)) = map.get(old_idx) {
                self.canvas.editing_target = state::EditingTarget::LibraryChip(*new_idx);
            } else {
                self.canvas.editing_target = state::EditingTarget::MainCanvas;
            }
        }
    }

    pub fn remap_library_chip(&mut self, from: usize, to: usize) {
        let len = self.engine.library.len();
        if from == to || from >= len || to >= len {
            return;
        }

        let mut new_order: Vec<usize> = (0..len).collect();
        let item = new_order.remove(from);
        new_order.insert(to, item);

        let mut map = vec![None; len];
        for (new_idx, &old_idx) in new_order.iter().enumerate() {
            map[old_idx] = Some(new_idx);
        }

        let mut new_library = Vec::with_capacity(len);
        for &old_idx in &new_order {
            new_library.push(self.engine.library[old_idx].clone());
        }
        self.engine.library = new_library;

        // Also reorder inside ungrouped or folder if both indices belong to the same group
        if from < self.global_library.ungrouped.len() && to < self.global_library.ungrouped.len() {
            let chip = self.global_library.ungrouped.remove(from);
            self.global_library.ungrouped.insert(to, chip);
        }

        self.apply_library_index_map(&map);
        global_library::save_global_library(&self.global_library);
    }

    pub fn delete_library_chip(&mut self, flat_idx: usize) {
        let old_len = self.engine.library.len();
        if flat_idx >= old_len {
            return;
        }

        let _ = self.global_library.remove_at_flat_index(flat_idx);
        if flat_idx < self.engine.library.len() {
            self.engine.library.remove(flat_idx);
        }

        let mut map = Vec::with_capacity(old_len);
        for i in 0..old_len {
            if i < flat_idx {
                map.push(Some(i));
            } else if i == flat_idx {
                map.push(None);
            } else {
                map.push(Some(i - 1));
            }
        }

        self.apply_library_index_map(&map);
        global_library::save_global_library(&self.global_library);
        self.compile();
    }

    pub fn get_connection_ports(
        &self,
        conn: &types::VisualConnection,
        src: &types::VisualComponent,
        tgt: &types::VisualComponent,
    ) -> (Vec2, Vec2) {
        let (_, src_outputs) =
            self.get_component_ports_count_with_width(src.comp_type, Some(src.bus_width()));
        let (tgt_inputs, _) =
            self.get_component_ports_count_with_width(tgt.comp_type, Some(tgt.bus_width()));

        let src_is_junc = src.comp_type == ComponentType::Junction;
        let tgt_is_junc = tgt.comp_type == ComponentType::Junction;

        if src_is_junc && tgt_is_junc {
            let src_pos = src.output_port_pos(conn.src_port, src_outputs);
            let tgt_pos = tgt.input_port_pos(conn.tgt_port, tgt_inputs);
            (src_pos, tgt_pos)
        } else if src_is_junc {
            let tgt_pos = tgt.input_port_pos(conn.tgt_port, tgt_inputs);
            let src_pos = self.get_junction_connect_pos(src, tgt_pos);
            (src_pos, tgt_pos)
        } else if tgt_is_junc {
            let src_pos = src.output_port_pos(conn.src_port, src_outputs);
            let tgt_pos = self.get_junction_connect_pos(tgt, src_pos);
            (src_pos, tgt_pos)
        } else {
            let src_pos = src.output_port_pos(conn.src_port, src_outputs);
            let tgt_pos = tgt.input_port_pos(conn.tgt_port, tgt_inputs);
            (src_pos, tgt_pos)
        }
    }

    fn get_junction_connect_pos(&self, junc: &types::VisualComponent, other_pos: Vec2) -> Vec2 {
        let horizontal = junc.width >= junc.height;
        if horizontal {
            let x = other_pos.x.clamp(junc.pos.x, junc.pos.x + junc.width);
            Vec2::new(x, junc.pos.y + junc.height / 2.0)
        } else {
            let y = other_pos.y.clamp(junc.pos.y, junc.pos.y + junc.height);
            Vec2::new(junc.pos.x + junc.width / 2.0, y)
        }
    }
}
