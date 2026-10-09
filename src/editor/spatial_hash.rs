use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

const CELL_SIZE: f32 = 120.0;

#[derive(Debug, Clone)]
pub struct SpatialHashGrid<T> {
    pub cells: HashMap<(i32, i32), HashSet<T>>,
    pub item_cells: HashMap<T, Vec<(i32, i32)>>,
}

impl<T: Eq + std::hash::Hash + Clone> Default for SpatialHashGrid<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Eq + std::hash::Hash + Clone> SpatialHashGrid<T> {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            item_cells: HashMap::new(),
        }
    }

    fn get_cell_coords(x: f32, y: f32) -> (i32, i32) {
        (
            (x / CELL_SIZE).floor() as i32,
            (y / CELL_SIZE).floor() as i32,
        )
    }

    pub fn get_cells_for_rect(rect: Rect) -> Vec<(i32, i32)> {
        let min_x = rect.x.min(rect.x + rect.w);
        let max_x = rect.x.max(rect.x + rect.w);
        let min_y = rect.y.min(rect.y + rect.h);
        let max_y = rect.y.max(rect.y + rect.h);

        let min_cell = Self::get_cell_coords(min_x, min_y);
        let max_cell = Self::get_cell_coords(max_x, max_y);

        let mut cells = Vec::new();
        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                cells.push((cx, cy));
            }
        }
        cells
    }

    pub fn insert(&mut self, item: T, rect: Rect) {
        self.remove_item(&item);
        let cells = Self::get_cells_for_rect(rect);
        for &cell in &cells {
            self.cells.entry(cell).or_default().insert(item.clone());
        }
        self.item_cells.insert(item, cells);
    }

    pub fn remove_item(&mut self, item: &T) {
        if let Some(cells) = self.item_cells.remove(item) {
            for cell in cells {
                if let Some(set) = self.cells.get_mut(&cell) {
                    set.remove(item);
                    if set.is_empty() {
                        self.cells.remove(&cell);
                    }
                }
            }
        }
    }

    pub fn remove(&mut self, item: &T, _rect: Rect) {
        self.remove_item(item);
    }

    pub fn update(&mut self, item: T, _old_rect: Rect, new_rect: Rect) {
        self.insert(item, new_rect);
    }

    pub fn query_point(&self, point: Vec2) -> Vec<T> {
        let cell = Self::get_cell_coords(point.x, point.y);
        if let Some(set) = self.cells.get(&cell) {
            set.iter().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn query_rect(&self, rect: Rect) -> HashSet<T> {
        let mut result = HashSet::new();
        for cell in Self::get_cells_for_rect(rect) {
            if let Some(set) = self.cells.get(&cell) {
                for item in set {
                    result.insert(item.clone());
                }
            }
        }
        result
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.item_cells.clear();
    }
}

impl super::Editor {
    pub fn rebuild_spatial_grid(&mut self) {
        self.canvas.spatial_grid.clear();
        self.canvas.wire_spatial_grid.clear();

        for comp in &self.circuit.components {
            let rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
            self.canvas.spatial_grid.insert(comp.id, rect);
        }

        let connections = self.circuit.connections.clone();
        for conn in &connections {
            self.insert_wire_into_grid(*conn);
        }
    }

    fn insert_wire_into_grid(&mut self, conn: crate::editor::types::VisualConnection) {
        let src_comp = self
            .circuit
            .components
            .iter()
            .find(|c| c.id == conn.src_comp_id);
        let tgt_comp = self
            .circuit
            .components
            .iter()
            .find(|c| c.id == conn.tgt_comp_id);

        if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
            let (src_pos, tgt_pos) = self.get_connection_ports(&conn, src, tgt);
            let offset = self.get_connection_routing_offset(&conn);
            let pad = offset.abs() + 20.0;

            let min_x = src_pos.x.min(tgt_pos.x) - pad;
            let max_x = src_pos.x.max(tgt_pos.x) + pad;
            let min_y = src_pos.y.min(tgt_pos.y) - pad;
            let max_y = src_pos.y.max(tgt_pos.y) + pad;

            let rect = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
            self.canvas.wire_spatial_grid.insert(conn, rect);
        }
    }

    pub fn rebuild_wire_grid(&mut self) {
        self.canvas.wire_spatial_grid.clear();
        let connections = self.circuit.connections.clone();
        for conn in &connections {
            self.insert_wire_into_grid(*conn);
        }
    }

    pub fn update_wires_for_components(&mut self, comp_ids: &std::collections::HashSet<usize>) {
        if comp_ids.is_empty() {
            return;
        }
        // Only update wires connected to the moved components
        let affected_wires: Vec<_> = self
            .circuit
            .connections
            .iter()
            .copied()
            .filter(|conn| {
                comp_ids.contains(&conn.src_comp_id) || comp_ids.contains(&conn.tgt_comp_id)
            })
            .collect();

        for conn in affected_wires {
            self.insert_wire_into_grid(conn);
        }
    }

    pub fn verify_grid(&self) {
        let mut missing = 0;
        for comp in &self.circuit.components {
            let rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
            let cells = SpatialHashGrid::<usize>::get_cells_for_rect(rect);
            for cell in cells {
                if !self
                    .canvas
                    .spatial_grid
                    .cells
                    .get(&cell)
                    .is_some_and(|set| set.contains(&comp.id))
                {
                    missing += 1;
                }
            }
        }
        if missing > 0 {
            // Note: `#![windows_subsystem = "windows"]` hides stdout, so this won't print in release mode.
            println!("Total missing from expected cells: {}", missing);
        }
    }
}
