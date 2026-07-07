use crate::editor::color_coding::ContextMenuTarget;
use crate::editor::types::{ActiveTool, VisualConnection};
use crate::engine::{ChipBlueprint, CompiledClock, OutputSource, Simulator};
use macroquad::prelude::Vec2;
use std::collections::{HashMap, HashSet};

use crate::editor::types::{TextAnnotation, VisualComponent};

#[derive(Clone)]
pub struct CanvasSnapshot {
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub annotations: Vec<TextAnnotation>,
    pub next_component_id: usize,
    pub pan: Vec2,
    pub zoom: f32,
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppMode {
    MainMenu,
    Editor,
    ManageChips,
    GlobalLibraryManager,
    Credits,
}

pub struct UiState {
    pub mode: AppMode,
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
    pub egui_wants_keyboard: bool,

    /// Screen-space canvas viewport rect (x, y, w, h) after egui panels are laid out.
    /// Used for camera fit/recenter calculations.
    pub canvas_viewport: Option<(f32, f32, f32, f32)>,

    // Custom chips DND state
    pub dragging_catalog_idx: Option<usize>,
    pub drag_hovered_idx: Option<usize>,

    // Global library manager state
    pub global_lib_selected_folder: Option<usize>,

    // Context menu state (right-click on chip/wire)
    pub show_context_menu: Option<ContextMenuTarget>,
    pub context_menu_pos: (f32, f32),
    pub context_menu_color: [f32; 4],
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            mode: AppMode::MainMenu,
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
            egui_wants_keyboard: false,
            canvas_viewport: None,
            dragging_catalog_idx: None,
            drag_hovered_idx: None,
            global_lib_selected_folder: None,
            show_context_menu: None,
            context_menu_pos: (0.0, 0.0),
            context_menu_color: [0.4, 0.45, 0.85, 1.0],
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
    pub active_wire_drag: Option<(usize, usize, bool)>,
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
    pub inspection_camera_stack: Vec<(Vec2, f32)>,

    // Sub-chip editing state
    pub editing_target: EditingTarget,
    pub stashed_main_canvas: Option<CanvasSnapshot>,

    // Right-click context menu detection
    pub right_drag_dist: f32,
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
            inspection_camera_stack: Vec::new(),
            editing_target: EditingTarget::MainCanvas,
            stashed_main_canvas: None,
            right_drag_dist: 0.0,
        }
    }
}
