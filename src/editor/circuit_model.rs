use crate::editor::types::{VisualComponent, VisualConnection, TextAnnotation};
use crate::editor::color_coding::ColorOverrides;
use std::collections::HashMap;
use crate::editor::wire_junctions::VerticalSeg;

pub struct CircuitModel {
    pub components: Vec<VisualComponent>,
    pub connections: Vec<VisualConnection>,
    pub next_component_id: usize,
    pub annotations: Vec<TextAnnotation>,
    pub comp_map: HashMap<usize, usize>,
    pub color_overrides: ColorOverrides,
    pub wire_nudges: HashMap<VisualConnection, f32>,
    pub wire_offsets: HashMap<VisualConnection, f32>,
    pub wire_lane_grid: HashMap<(i32, i32), Vec<(usize, VerticalSeg, VisualConnection)>>,
}

impl Default for CircuitModel {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            connections: Vec::new(),
            next_component_id: 1,
            annotations: Vec::new(),
            comp_map: HashMap::new(),
            color_overrides: ColorOverrides::default(),
            wire_nudges: HashMap::new(),
            wire_offsets: HashMap::new(),
            wire_lane_grid: HashMap::new(),
        }
    }
}
