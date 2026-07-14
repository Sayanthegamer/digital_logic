use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

const CELL_SIZE: f32 = 120.0;

#[derive(Debug, Clone)]
pub struct SpatialHashGrid<T> {
    pub cells: HashMap<(i32, i32), HashSet<T>>,
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
        }
    }

    fn get_cell_coords(x: f32, y: f32) -> (i32, i32) {
        ((x / CELL_SIZE).floor() as i32, (y / CELL_SIZE).floor() as i32)
    }

    pub fn get_cells_for_rect(rect: Rect) -> Vec<(i32, i32)> {
        let min_cell = Self::get_cell_coords(rect.x, rect.y);
        let max_cell = Self::get_cell_coords(rect.x + rect.w, rect.y + rect.h);

        let mut cells = Vec::new();
        for cx in min_cell.0..=max_cell.0 {
            for cy in min_cell.1..=max_cell.1 {
                cells.push((cx, cy));
            }
        }
        cells
    }

    pub fn insert(&mut self, item: T, rect: Rect) {
        for cell in Self::get_cells_for_rect(rect) {
            self.cells.entry(cell).or_default().insert(item.clone());
        }
    }

    pub fn remove(&mut self, item: &T, rect: Rect) {
        for cell in Self::get_cells_for_rect(rect) {
            if let Some(set) = self.cells.get_mut(&cell) {
                set.remove(item);
                if set.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    /// Incremental cell reassignment (O(1) localized cost instead of full rebuild)
    pub fn update(&mut self, item: T, old_rect: Rect, new_rect: Rect) {
        let old_cells = Self::get_cells_for_rect(old_rect);
        let new_cells = Self::get_cells_for_rect(new_rect);

        // Remove from cells that are no longer occupied
        for &cell in &old_cells {
            if !new_cells.contains(&cell) {
                if let Some(set) = self.cells.get_mut(&cell) {
                    set.remove(&item);
                    if set.is_empty() {
                        self.cells.remove(&cell);
                    }
                }
            }
        }

        // Add to new cells
        for cell in new_cells {
            if !old_cells.contains(&cell) {
                self.cells.entry(cell).or_default().insert(item.clone());
            }
        }
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
    }
}

impl super::Editor {
    pub fn rebuild_spatial_grid(&mut self) {
        self.canvas.spatial_grid.clear();
        self.canvas.wire_spatial_grid.clear();
        
        let mut count = 0;
        for comp in &self.circuit.components {
            let rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
            self.canvas.spatial_grid.insert(comp.id, rect);
            count += 1;
        }

        let mut wire_count = 0;
        let connections = self.circuit.connections.clone();
        for conn in &connections {
            wire_count += 1;
            self.insert_wire_into_grid(*conn);
        }
        
        println!("rebuild_spatial_grid: inserted {} components, {} wires", count, wire_count);
    }
    
    fn insert_wire_into_grid(&mut self, conn: crate::editor::types::VisualConnection) {
        let src_comp = self.circuit.components.iter().find(|c| c.id == conn.src_comp_id);
        let tgt_comp = self.circuit.components.iter().find(|c| c.id == conn.tgt_comp_id);
        
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
        // Since removing a wire from the grid requires its old Rect, and calculating it is complex 
        // if the component already moved, we can simply rebuild the whole wire grid. 
        // It's extremely fast even for 10k wires.
        self.rebuild_wire_grid();
    }

    pub fn verify_grid(&self) {
        let mut missing = 0;
        for comp in &self.circuit.components {
            let rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
            let cells = SpatialHashGrid::<usize>::get_cells_for_rect(rect);
            for cell in cells {
                if !self.canvas.spatial_grid.cells.get(&cell).map_or(false, |set| set.contains(&comp.id)) {
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
