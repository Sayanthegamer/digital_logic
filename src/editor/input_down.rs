use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn handle_canvas_left_drag(&mut self, mouse_pos_world: Vec2, mouse_delta: Vec2) {
        self.canvas.drag_dist_pixels += mouse_delta.length();
        // Drag component (multi-selection snapped drag)
        if let Some(_comp_id) = self.canvas.dragging_comp_id {
            let translation = mouse_pos_world - self.canvas.drag_offset;
            let mut snapped_translation = Vec2::new(
                (translation.x / 20.0).round() * 20.0,
                (translation.y / 20.0).round() * 20.0,
            );

            // Magnetic Alignment Snapping override
            if let Some(&first_drag_id) = self.canvas.drag_start_positions.keys().next() {
                if let Some(start_pos) = self.canvas.drag_start_positions.get(&first_drag_id) {
                    if let Some(c) = self.get_component(first_drag_id) {
                        let raw_x = start_pos.x + translation.x;
                        let raw_y = start_pos.y + translation.y;
                        
                        let c_left = raw_x;
                        let c_right = raw_x + c.width;
                        let c_cx = raw_x + c.width / 2.0;

                        let c_top = raw_y;
                        let c_bottom = raw_y + c.height;
                        let c_cy = raw_y + c.height / 2.0;

                        let mut best_dx = 0.0;
                        let mut min_dx_dist = 10.0;
                        
                        let mut best_dy = 0.0;
                        let mut min_dy_dist = 10.0;

                        for other in &self.circuit.components {
                            if self.canvas.selected_comp_ids.contains(&other.id) { continue; }
                            
                            let o_left = other.pos.x;
                            let o_right = other.pos.x + other.width;
                            let o_cx = other.pos.x + other.width / 2.0;
                            
                            let o_top = other.pos.y;
                            let o_bottom = other.pos.y + other.height;
                            let o_cy = other.pos.y + other.height / 2.0;

                            let x_matches = [
                                (c_left, o_left), (c_left, o_right), (c_left, o_cx),
                                (c_right, o_left), (c_right, o_right), (c_right, o_cx),
                                (c_cx, o_left), (c_cx, o_right), (c_cx, o_cx)
                            ];
                            for &(my_x, other_x) in &x_matches {
                                let dist = (my_x - other_x).abs();
                                if dist < min_dx_dist {
                                    min_dx_dist = dist;
                                    best_dx = other_x - my_x;
                                }
                            }
                            
                            let y_matches = [
                                (c_top, o_top), (c_top, o_bottom), (c_top, o_cy),
                                (c_bottom, o_top), (c_bottom, o_bottom), (c_bottom, o_cy),
                                (c_cy, o_top), (c_cy, o_bottom), (c_cy, o_cy)
                            ];
                            for &(my_y, other_y) in &y_matches {
                                let dist = (my_y - other_y).abs();
                                if dist < min_dy_dist {
                                    min_dy_dist = dist;
                                    best_dy = other_y - my_y;
                                }
                            }
                        }

                        if min_dx_dist < 10.0 {
                            snapped_translation.x = translation.x + best_dx;
                        }
                        if min_dy_dist < 10.0 {
                            snapped_translation.y = translation.y + best_dy;
                        }
                    }
                }
            }

            let shift_pressed =
                is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);

            if !self.canvas.drag_snapshot_pushed {
                self.push_history_snapshot();
                self.canvas.drag_snapshot_pushed = true;
            }

            let drag_offset = self.canvas.drag_offset;
            let updates: Vec<_> = self.canvas.drag_start_positions.iter().map(|(&id, &start_pos)| {
                let start_size = self.canvas.drag_start_sizes.get(&id).copied().unwrap_or(Vec2::new(12.0, 12.0));
                (id, start_pos, start_size)
            }).collect();

            let mut grid_updates = Vec::new();
            for (id, start_pos, start_size) in updates {
                if let Some(c) = self.get_component_mut(id) {
                    if c.comp_type == crate::engine::ComponentType::Junction
                        && shift_pressed
                    {
                        let center = start_pos + start_size / 2.0;
                        let is_right = drag_offset.x > center.x;
                        let is_bottom = drag_offset.y > center.y;

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
            
            let mut moved_comp_ids = std::collections::HashSet::new();
            for (id, old_rect, new_rect) in grid_updates {
                self.canvas.spatial_grid.update(id, old_rect, new_rect);
                moved_comp_ids.insert(id);
            }
            if !moved_comp_ids.is_empty() {
                self.update_wires_for_components(&moved_comp_ids);
            }

            self.canvas.alignment_guides.clear();
            let mut x_guides = 0;
            let mut y_guides = 0;
            if let Some(&first_drag_id) = self.canvas.drag_start_positions.keys().next() {
                if let Some(c) = self.get_component(first_drag_id) {
                    let c_left = c.pos.x;
                    let c_right = c.pos.x + c.width;
                    let c_cx = c.pos.x + c.width / 2.0;

                    let c_top = c.pos.y;
                    let c_bottom = c.pos.y + c.height;
                    let c_cy = c.pos.y + c.height / 2.0;

                    for other in &self.circuit.components {
                        if self.canvas.selected_comp_ids.contains(&other.id) { continue; }
                        
                        let o_left = other.pos.x;
                        let o_right = other.pos.x + other.width;
                        let o_cx = other.pos.x + other.width / 2.0;
                        
                        let o_top = other.pos.y;
                        let o_bottom = other.pos.y + other.height;
                        let o_cy = other.pos.y + other.height / 2.0;

                        if x_guides < 2 {
                            let matches = [
                                (c_left, o_left), (c_left, o_right), (c_left, o_cx),
                                (c_right, o_left), (c_right, o_right), (c_right, o_cx),
                                (c_cx, o_left), (c_cx, o_right), (c_cx, o_cx)
                            ];
                            for &(my_x, other_x) in &matches {
                                if (my_x - other_x).abs() < 1.0 {
                                    self.canvas.alignment_guides.push((Vec2::new(my_x, -10000.0), Vec2::new(my_x, 10000.0)));
                                    x_guides += 1;
                                    break; // Only one guide per other component per axis
                                }
                            }
                        }
                        
                        if y_guides < 2 {
                            let matches = [
                                (c_top, o_top), (c_top, o_bottom), (c_top, o_cy),
                                (c_bottom, o_top), (c_bottom, o_bottom), (c_bottom, o_cy),
                                (c_cy, o_top), (c_cy, o_bottom), (c_cy, o_cy)
                            ];
                            for &(my_y, other_y) in &matches {
                                if (my_y - other_y).abs() < 1.0 {
                                    self.canvas.alignment_guides.push((Vec2::new(-10000.0, my_y), Vec2::new(10000.0, my_y)));
                                    y_guides += 1;
                                    break; // Only one guide per other component per axis
                                }
                            }
                        }
                        
                        if x_guides >= 2 && y_guides >= 2 {
                            break;
                        }
                    }
                }
            }
        }
        // Drag annotation
        if let Some(idx) = self.canvas.dragging_annotation_idx
            && idx < self.circuit.annotations.len()
        {
            let target_pos = mouse_pos_world + self.canvas.drag_offset;
            self.circuit.annotations[idx].pos = Vec2::new(
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
                let current = self.circuit.wire_nudges.get(&wire).copied().unwrap_or(0.0);
                self.circuit.wire_nudges.insert(wire, current + delta / self.canvas.zoom);
            }
        }
    }
}
