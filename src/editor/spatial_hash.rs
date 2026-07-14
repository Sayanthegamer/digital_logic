use macroquad::prelude::*;
use std::collections::{HashMap, HashSet};

const CELL_SIZE: f32 = 120.0;

#[derive(Debug, Clone)]
pub struct SpatialHashGrid {
    cells: HashMap<(i32, i32), HashSet<usize>>,
}

impl Default for SpatialHashGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl SpatialHashGrid {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    fn get_cell_coords(x: f32, y: f32) -> (i32, i32) {
        ((x / CELL_SIZE).floor() as i32, (y / CELL_SIZE).floor() as i32)
    }

    fn get_cells_for_rect(rect: Rect) -> Vec<(i32, i32)> {
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

    pub fn insert(&mut self, id: usize, rect: Rect) {
        for cell in Self::get_cells_for_rect(rect) {
            self.cells.entry(cell).or_default().insert(id);
        }
    }

    pub fn remove(&mut self, id: usize, rect: Rect) {
        for cell in Self::get_cells_for_rect(rect) {
            if let Some(set) = self.cells.get_mut(&cell) {
                set.remove(&id);
                if set.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    /// Incremental cell reassignment (O(1) localized cost instead of full rebuild)
    pub fn update(&mut self, id: usize, old_rect: Rect, new_rect: Rect) {
        let old_cells = Self::get_cells_for_rect(old_rect);
        let new_cells = Self::get_cells_for_rect(new_rect);

        // Remove from cells that are no longer occupied
        for &cell in &old_cells {
            if !new_cells.contains(&cell) {
                if let Some(set) = self.cells.get_mut(&cell) {
                    set.remove(&id);
                    if set.is_empty() {
                        self.cells.remove(&cell);
                    }
                }
            }
        }

        // Add to new cells
        for cell in new_cells {
            if !old_cells.contains(&cell) {
                self.cells.entry(cell).or_default().insert(id);
            }
        }
    }

    pub fn query_point(&self, point: Vec2) -> Vec<usize> {
        let cell = Self::get_cell_coords(point.x, point.y);
        if let Some(set) = self.cells.get(&cell) {
            set.iter().copied().collect()
        } else {
            Vec::new()
        }
    }

    pub fn query_rect(&self, rect: Rect) -> HashSet<usize> {
        let mut result = HashSet::new();
        for cell in Self::get_cells_for_rect(rect) {
            if let Some(set) = self.cells.get(&cell) {
                for &id in set {
                    result.insert(id);
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
        let mut count = 0;
        for comp in &self.circuit.components {
            let rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
            self.canvas.spatial_grid.insert(comp.id, rect);
            count += 1;
        }
        println!("rebuild_spatial_grid: inserted {} components into {} cells", count, self.canvas.spatial_grid.cells.len());
    }

    pub fn verify_grid(&self) {
        let mut missing = 0;
        for comp in &self.circuit.components {
            let rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
            let cells = SpatialHashGrid::get_cells_for_rect(rect);
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
