use macroquad::prelude::Vec2;
use std::collections::{HashMap, HashSet};
use crate::engine::{Simulator, ChipBlueprint, OutputSource, CompiledClock};
use crate::editor::types::{ActiveTool, VisualConnection};

use crate::editor::types::{VisualComponent, TextAnnotation};

#[derive(Clone)]
pub struct CanvasSnapshot {
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub annotations: Vec<TextAnnotation>,
    pub next_component_id: usize,
}

pub struct HistoryManager {
    pub undo_stack: Vec<CanvasSnapshot>,
    pub redo_stack: Vec<CanvasSnapshot>,
    pub max_steps: usize,
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_steps: 50,
        }
    }
}

pub struct EngineState {
    pub library: Vec<ChipBlueprint>,
    pub simulator: Simulator,
    pub visual_to_sim_map: HashMap<usize, usize>, // Visual ID -> Sim gate index
    pub port_to_sim_gate_map: HashMap<(usize, usize), usize>, // (Visual ID, port_idx) -> Sim gate index
    pub instance_to_sim_map: HashMap<(Vec<usize>, usize), usize>,
    pub instance_outputs: HashMap<(Vec<usize>, usize), Vec<OutputSource>>,
    pub active_clocks: Vec<CompiledClock>,
    pub is_playing: bool,
    pub ticks_per_frame: usize,
    pub sim_tick_counter: usize,
    pub propagation_error: Option<String>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            library: Vec::new(),
            simulator: Simulator::new(),
            visual_to_sim_map: HashMap::new(),
            port_to_sim_gate_map: HashMap::new(),
            instance_to_sim_map: HashMap::new(),
            instance_outputs: HashMap::new(),
            active_clocks: Vec::new(),
            is_playing: true,
            ticks_per_frame: 1,
            sim_tick_counter: 0,
            propagation_error: None,
        }
    }
}

pub struct UiState {
    pub show_settings: bool,
    pub is_fullscreen: bool,
    pub resolution_idx: usize,
    pub ui_scale: f32,
    pub temp_is_fullscreen: bool,
    pub temp_resolution_idx: usize,
    pub temp_ui_scale: f32,
    pub resolution_revert_timer: Option<f32>,
    pub prev_is_fullscreen: bool,
    pub prev_resolution_idx: usize,
    pub show_menu_mobile: bool,
    pub catalog_page: usize,
    pub catalog_scroll_request: Option<f32>,
    pub controls_scroll_request: Option<f32>,
    pub chip_name_input: String,
    pub catalog_search_text: String,
    pub egui_wants_pointer: bool,

    /// Screen-space canvas viewport rect (x, y, w, h) after egui panels are laid out.
    /// Used for camera fit/recenter calculations.
    pub canvas_viewport: Option<(f32, f32, f32, f32)>,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_settings: false,
            is_fullscreen: false,
            resolution_idx: 2, // 1280x720 by default
            ui_scale: 1.0,
            temp_is_fullscreen: false,
            temp_resolution_idx: 2,
            temp_ui_scale: 1.0,
            resolution_revert_timer: None,
            prev_is_fullscreen: false,
            prev_resolution_idx: 2,
            show_menu_mobile: false,
            catalog_page: 0,
            catalog_scroll_request: None,
            controls_scroll_request: None,
            chip_name_input: "MY_CHIP".to_string(),
            catalog_search_text: String::new(),
            egui_wants_pointer: false,
            canvas_viewport: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditingTarget {
    MainCanvas,
    LibraryChip(usize),
}

pub struct CanvasState {
    pub pan: Vec2,
    pub zoom: f32,
    pub last_mouse_pos: Vec2,
    pub selected_tool: Option<ActiveTool>,
    pub active_wire_drag: Option<(usize, usize)>,
    pub hovered_port: Option<(usize, usize, bool)>,
    pub dragging_comp_id: Option<usize>,
    pub drag_offset: Vec2,
    pub drag_dist_pixels: f32,
    pub selected_comp_ids: HashSet<usize>,
    pub selected_comp_id: Option<usize>,
    pub selected_connections: HashSet<VisualConnection>,
    pub selection_box_start: Option<Vec2>,
    pub drag_start_positions: HashMap<usize, Vec2>,
    pub drag_start_sizes: HashMap<usize, Vec2>,
    /// True once we've pushed an undo snapshot for the current drag gesture.
    pub drag_snapshot_pushed: bool,
    
    // Annotations interaction
    pub selected_annotation_idx: Option<usize>,
    pub dragging_annotation_idx: Option<usize>,
    pub last_click_time: f64,
    pub last_clicked_annotation_idx: Option<usize>,
    pub focus_annotation_text: bool,
    
    // Touch
    pub last_touch_dist: Option<f32>,
    pub last_touch_center: Option<Vec2>,
    
    // Inspection
    pub inspection_path: Vec<usize>,
    
    // Sub-chip editing state
    pub editing_target: EditingTarget,
    pub stashed_main_canvas: Option<CanvasSnapshot>,
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            pan: Vec2::new(200.0, 100.0),
            zoom: 1.0,
            last_mouse_pos: Vec2::ZERO,
            selected_tool: None,
            active_wire_drag: None,
            hovered_port: None,
            dragging_comp_id: None,
            drag_offset: Vec2::ZERO,
            drag_dist_pixels: 0.0,
            selected_comp_ids: HashSet::new(),
            selected_comp_id: None,
            selected_connections: HashSet::new(),
            selection_box_start: None,
            drag_start_positions: HashMap::new(),
            drag_start_sizes: HashMap::new(),
            drag_snapshot_pushed: false,
            selected_annotation_idx: None,
            dragging_annotation_idx: None,
            last_click_time: 0.0,
            last_clicked_annotation_idx: None,
            focus_annotation_text: false,
            last_touch_dist: None,
            last_touch_center: None,
            inspection_path: Vec::new(),
            editing_target: EditingTarget::MainCanvas,
            stashed_main_canvas: None,
        }
    }
}
