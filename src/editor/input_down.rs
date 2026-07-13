use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn handle_canvas_left_drag(&mut self, mouse_pos_world: Vec2, mouse_delta: Vec2) {
        self.canvas.drag_dist_pixels += mouse_delta.length();
        // Drag component (multi-selection snapped drag)
        if let Some(_comp_id) = self.canvas.dragging_comp_id {
            let translation = mouse_pos_world - self.canvas.drag_offset;
            let snapped_translation = Vec2::new(
                (translation.x / 20.0).round() * 20.0,
                (translation.y / 20.0).round() * 20.0,
            );

            let shift_pressed =
                is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

            let mut needs_snapshot = false;
            if shift_pressed && !self.canvas.drag_snapshot_pushed {
                for &id in self.canvas.drag_start_positions.keys() {
                    if self.components.iter().any(|c| {
                        c.id == id && c.comp_type == crate::engine::ComponentType::Junction
                    }) {
                        needs_snapshot = true;
                        break;
                    }
                }
            }
            if needs_snapshot {
                self.push_history_snapshot();
                self.canvas.drag_snapshot_pushed = true;
            }

            let mut grid_updates = Vec::new();
            for (&id, &start_pos) in &self.canvas.drag_start_positions {
                if let Some(c) = self.components.iter_mut().find(|x| x.id == id) {
                    if c.comp_type == crate::engine::ComponentType::Junction
                        && shift_pressed
                    {
                        let start_size = self
                            .canvas
                            .drag_start_sizes
                            .get(&id)
                            .copied()
                            .unwrap_or(Vec2::new(12.0, 12.0));
                        let center = start_pos + start_size / 2.0;
                        let is_right = self.canvas.drag_offset.x > center.x;
                        let is_bottom = self.canvas.drag_offset.y > center.y;

                        let old_rect = Rect::new(c.pos.x, c.pos.y, c.width, c.height);

                        // Stretching logic instead of moving
                        // We stretch horizontally or vertically depending on dominant translation
                        if translation.x.abs() > translation.y.abs() {
                            if is_right {
                                c.pos.x = start_pos.x;
                                c.width = (start_size.x + snapped_translation.x)
                                    .clamp(12.0, 2000.0);
                            } else {
                                let new_width = (start_size.x - snapped_translation.x)
                                    .clamp(12.0, 2000.0);
                                let actual_delta = start_size.x - new_width;
                                c.pos.x = start_pos.x + actual_delta;
                                c.width = new_width;
                            }
                            c.height = start_size.y;
                            c.pos.y = start_pos.y;
                        } else {
                            if is_bottom {
                                c.pos.y = start_pos.y;
                                c.height = (start_size.y + snapped_translation.y)
                                    .clamp(12.0, 2000.0);
                            } else {
                                let new_height = (start_size.y - snapped_translation.y)
                                    .clamp(12.0, 2000.0);
                                let actual_delta = start_size.y - new_height;
                                c.pos.y = start_pos.y + actual_delta;
                                c.height = new_height;
                            }
                            c.width = start_size.x;
                            c.pos.x = start_pos.x;
                        }
                        
                        let new_rect = Rect::new(c.pos.x, c.pos.y, c.width, c.height);
                        grid_updates.push((id, old_rect, new_rect));
                    } else {
                        let old_rect = Rect::new(c.pos.x, c.pos.y, c.width, c.height);
                        c.pos = start_pos + snapped_translation;
                        let new_rect = Rect::new(c.pos.x, c.pos.y, c.width, c.height);
                        grid_updates.push((id, old_rect, new_rect));
                    }
                }
            }
            
            for (id, old_rect, new_rect) in grid_updates {
                self.canvas.spatial_grid.update(id, old_rect, new_rect);
            }
        }
        // Drag annotation
        if let Some(idx) = self.canvas.dragging_annotation_idx
            && idx < self.annotations.len()
        {
            let target_pos = mouse_pos_world + self.canvas.drag_offset;
            self.annotations[idx].pos = Vec2::new(
                (target_pos.x / 20.0).round() * 20.0,
                (target_pos.y / 20.0).round() * 20.0,
            );
        }

        // Drag wire nudge
        if let Some(wire) = self.canvas.dragging_wire {
            let delta = if mouse_delta.x.abs() > mouse_delta.y.abs() {
                mouse_delta.x
            } else {
                mouse_delta.y
            };
            if delta.abs() > 0.0 {
                let key = wire.color_key();
                let current = self.wire_nudges.get(&key).copied().unwrap_or(0.0);
                self.wire_nudges.insert(key, current + delta / self.canvas.zoom);
            }
        }
    }
}
