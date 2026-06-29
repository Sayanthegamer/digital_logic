use crate::engine::ComponentType;
use macroquad::prelude::*;

use super::Editor;
use super::types::*;

impl Editor {
    pub fn to_world_space(&self, screen_pos: Vec2) -> Vec2 {
        (screen_pos - self.pan) / self.zoom
    }

    pub fn to_screen_space(&self, world_pos: Vec2) -> Vec2 {
        world_pos * self.zoom + self.pan
    }

    pub fn update(&mut self) {
        let mouse_pos_screen = mouse_position().into();
        let mouse_pos_world = self.to_world_space(mouse_pos_screen);
        
        let mouse_delta = if self.last_mouse_pos == Vec2::ZERO {
            Vec2::ZERO
        } else {
            mouse_pos_screen - self.last_mouse_pos
        };
        
        let egui_wants_pointer = self.egui_wants_pointer;

        // --- Touch Input Abstraction (Mobile) ---
        let touch_events = touches();
        let is_multi_touch = touch_events.len() >= 2;

        if !egui_wants_pointer && is_multi_touch {
            let t1 = &touch_events[0];
            let t2 = &touch_events[1];
            
            let current_dist = t1.position.distance(t2.position);
            let current_center = (t1.position + t2.position) * 0.5;

            if let Some(last_dist) = self.last_touch_dist {
                if last_dist > 0.0 {
                    let prev_zoom = self.zoom;
                    self.zoom *= current_dist / last_dist;
                    self.zoom = self.zoom.clamp(0.15, 4.0);
                    // Zoom towards the center of the pinch
                    self.pan = current_center - (current_center - self.pan) * (self.zoom / prev_zoom);
                }
            }

            if let Some(last_center) = self.last_touch_center {
                let delta = current_center - last_center;
                self.pan += delta;
            }

            self.last_touch_dist = Some(current_dist);
            self.last_touch_center = Some(current_center);
            
            // Clear single-touch interactions when multi-touch begins
            self.dragging_comp_id = None;
            self.active_wire_drag = None;
            self.selection_box_start = None;
        } else {
            self.last_touch_dist = None;
            self.last_touch_center = None;
        }

        // Keyboard Shortcuts
        if !egui_wants_pointer {
            if is_key_pressed(KeyCode::Space) {
                self.is_playing = !self.is_playing;
            }
            if is_key_pressed(KeyCode::C) {
                self.compile();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.selected_tool = None;
                self.selected_comp_id = None;
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::S) {
                self.save_project();
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) && is_key_pressed(KeyCode::L) {
                self.load_project();
            }
        }

        // 1. Zoom with mouse wheel
        if !egui_wants_pointer {
            let scroll = mouse_wheel().1;
            if scroll != 0.0 {
                let prev_zoom = self.zoom;
                if scroll > 0.0 {
                    self.zoom *= 1.15;
                } else {
                    self.zoom /= 1.15;
                }
                self.zoom = self.zoom.clamp(0.15, 4.0);
                
                // Pan adjustment to zoom on mouse cursor
                self.pan = mouse_pos_screen - (mouse_pos_screen - self.pan) * (self.zoom / prev_zoom);
            }
        }

        // 2. Pan with right drag
        if !egui_wants_pointer && is_mouse_button_down(MouseButton::Right) {
            self.pan += mouse_delta;
            self.selected_tool = None;
        }

        // 3. Interactions: Left click / drag (only in main canvas)
        if !egui_wants_pointer && !is_multi_touch && self.inspection_path.is_empty() {
            if is_mouse_button_pressed(MouseButton::Left) {
                // Click on a port or component
                let mut clicked_something = false;

                // Check ports first (wiring starts here)
                for comp in &self.components {
                    let (_, outputs_count) = self.get_component_ports_count(comp.comp_type);
                    
                    // Click on output ports
                    for o in 0..outputs_count {
                        let port_pos = comp.output_port_pos(o, outputs_count);
                        if port_pos.distance(mouse_pos_world) < 8.0 {
                            self.active_wire_drag = Some((comp.id, o));
                            clicked_something = true;
                            break;
                        }
                    }
                    if clicked_something { break; }
                }

                // Check clicking inside components (dragging, toggling)
                if !clicked_something {
                    let mut found_comp = None;
                    for comp in &self.components {
                        if mouse_pos_world.x >= comp.pos.x
                            && mouse_pos_world.x <= comp.pos.x + comp.width
                            && mouse_pos_world.y >= comp.pos.y
                            && mouse_pos_world.y <= comp.pos.y + comp.height
                        {
                            found_comp = Some(comp.clone());
                            break;
                        }
                    }

                    if let Some(comp) = found_comp {
                        self.selected_comp_id = Some(comp.id);
                        self.selected_annotation_idx = None;
                        
                        // Handle multi-selection tracking
                        if self.selected_comp_ids.contains(&comp.id) {
                            // Already selected, keep multi-selection
                        } else if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
                            self.selected_comp_ids.insert(comp.id);
                        } else {
                            self.selected_comp_ids.clear();
                            self.selected_comp_ids.insert(comp.id);
                        }

                        // Start dragging (Input components are also draggable now!)
                        self.dragging_comp_id = Some(comp.id);
                        self.drag_offset = mouse_pos_world;
                        self.drag_dist_pixels = 0.0;
                        
                        // Store starting positions of all selected components
                        self.drag_start_positions.clear();
                        for &id in &self.selected_comp_ids {
                            if let Some(c) = self.components.iter().find(|x| x.id == id) {
                                self.drag_start_positions.insert(id, c.pos);
                            }
                        }
                        clicked_something = true;
                    } else {
                        // Check if we clicked an annotation
                        let mut clicked_ann = None;
                        for (idx, ann) in self.annotations.iter().enumerate() {
                            let text_w = measure_text(&ann.text, None, 15, 1.0).width;
                            let rect = Rect::new(ann.pos.x - 5.0, ann.pos.y - 14.0, text_w + 10.0, 20.0);
                            if rect.contains(mouse_pos_world) {
                                clicked_ann = Some(idx);
                                break;
                            }
                        }
                        
                        if let Some(idx) = clicked_ann {
                            self.selected_annotation_idx = Some(idx);
                            self.dragging_annotation_idx = Some(idx);
                            self.selected_comp_id = None;
                            self.selected_comp_ids.clear();
                            self.drag_offset = self.annotations[idx].pos - mouse_pos_world;
                            clicked_something = true;
                        } else {
                            // Clicked empty space
                            self.selected_comp_id = None;
                            self.selected_annotation_idx = None;
                            
                            // Start drag selection box if no placement tool is active
                            if self.selected_tool.is_none() {
                                self.selected_comp_ids.clear();
                                self.selection_box_start = Some(mouse_pos_world);
                            }
                        }
                    }
                }

                // If nothing was clicked and a tool is active, place the component or annotation
                if !clicked_something {
                    if let Some(tool) = self.selected_tool {
                        match tool {
                            ActiveTool::PlaceComponent(comp_type) => {
                                let (inputs, outputs) = self.get_component_ports_count(comp_type);
                                let max_ports = inputs.max(outputs);
                                let height = 40.0 + (max_ports as f32 * 16.0);
                                let width = match comp_type {
                                    ComponentType::SubChip(_) => 100.0,
                                    _ => 70.0,
                                };
                                
                                let label = self.get_component_label(comp_type);
                                
                                let clock_period = match comp_type {
                                    ComponentType::Clock => Some(20),
                                    _ => None,
                                };

                                let new_id = self.next_component_id;
                                let target_pos = mouse_pos_world - Vec2::new(width / 2.0, height / 2.0);
                                let snapped_pos = Vec2::new((target_pos.x / 20.0).round() * 20.0, (target_pos.y / 20.0).round() * 20.0);

                                self.components.push(VisualComponent {
                                    id: new_id,
                                    comp_type,
                                    pos: snapped_pos,
                                    width,
                                    height,
                                    label,
                                    clock_period,
                                });
                                self.selected_comp_id = Some(new_id);
                                self.selected_comp_ids.clear();
                                self.selected_comp_ids.insert(new_id);
                                self.next_component_id += 1;
                                self.compile();
                            }
                            ActiveTool::PlaceAnnotation => {
                                let snapped_pos = Vec2::new((mouse_pos_world.x / 20.0).round() * 20.0, (mouse_pos_world.y / 20.0).round() * 20.0);
                                self.annotations.push(TextAnnotation {
                                    text: "Double-click to edit".to_string(),
                                    pos: snapped_pos,
                                });
                                self.selected_annotation_idx = Some(self.annotations.len() - 1);
                                self.selected_comp_id = None;
                            }
                        }
                    }
                }
            } else if is_mouse_button_down(MouseButton::Left) {
                self.drag_dist_pixels += mouse_delta.length();
                // Drag component (multi-selection snapped drag)
                if let Some(_comp_id) = self.dragging_comp_id {
                    let translation = mouse_pos_world - self.drag_offset;
                    let snapped_translation = Vec2::new(
                        (translation.x / 20.0).round() * 20.0,
                        (translation.y / 20.0).round() * 20.0,
                    );
                    for (&id, &start_pos) in &self.drag_start_positions {
                        if let Some(c) = self.components.iter_mut().find(|x| x.id == id) {
                            c.pos = start_pos + snapped_translation;
                        }
                    }
                }
                // Drag annotation
                if let Some(idx) = self.dragging_annotation_idx {
                    if idx < self.annotations.len() {
                        let target_pos = mouse_pos_world + self.drag_offset;
                        self.annotations[idx].pos = Vec2::new((target_pos.x / 20.0).round() * 20.0, (target_pos.y / 20.0).round() * 20.0);
                    }
                }
            } else if is_mouse_button_released(MouseButton::Left) {
                // If it was an Input component and it was clicked (not dragged far)
                if let Some(comp_id) = self.dragging_comp_id {
                    if self.drag_dist_pixels < 5.0 {
                        let mut comp_type = None;
                        for comp in &self.components {
                            if comp.id == comp_id {
                                comp_type = Some(comp.comp_type);
                                break;
                            }
                        }
                        if comp_type == Some(ComponentType::Input) {
                            if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp_id) {
                                let curr_val = self.simulator.get_state(gate_idx);
                                self.simulator.set_input(gate_idx, !curr_val);
                                let max_steps = (self.simulator.gates.len() * 10).max(1000);
                                match self.simulator.propagate_events(max_steps) {
                                    Ok(_) => self.propagation_error = None,
                                    Err(e) => self.propagation_error = Some(e),
                                }
                            }
                        }
                    }
                }

                // If drag selection box was active, parse selection
                if let Some(start) = self.selection_box_start {
                    let end = mouse_pos_world;
                    let x_min = start.x.min(end.x);
                    let x_max = start.x.max(end.x);
                    let y_min = start.y.min(end.y);
                    let y_max = start.y.max(end.y);
                    
                    let box_w = x_max - x_min;
                    let box_h = y_max - y_min;
                    
                    if box_w > 5.0 || box_h > 5.0 {
                        self.selected_comp_ids.clear();
                        self.selected_comp_id = None;
                        for comp in &self.components {
                            let comp_rect = Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
                            let box_rect = Rect::new(x_min, y_min, box_w, box_h);
                            if comp_rect.overlaps(&box_rect) {
                                self.selected_comp_ids.insert(comp.id);
                            }
                        }
                        if self.selected_comp_ids.len() == 1 {
                            self.selected_comp_id = self.selected_comp_ids.iter().next().copied();
                        }
                    } else {
                        // Clicked empty space: clear selection
                        self.selected_comp_ids.clear();
                        self.selected_comp_id = None;
                    }
                    self.selection_box_start = None;
                }

                // End drag
                self.dragging_comp_id = None;
                self.drag_start_positions.clear();
                self.dragging_annotation_idx = None;

                // Handle wiring connection release
                if let Some((src_id, src_port)) = self.active_wire_drag {
                    // Look for target input port
                    let mut connection_made = false;
                    for comp in &self.components {
                        if comp.id == src_id { continue; }
                        let (inputs_count, _) = self.get_component_ports_count(comp.comp_type);
                        
                        for i in 0..inputs_count {
                            let port_pos = comp.input_port_pos(i, inputs_count);
                            if port_pos.distance(mouse_pos_world) < 8.0 {
                                // Add wire connection, remove duplicates targeting this port
                                self.connections.retain(|c| !(c.tgt_comp_id == comp.id && c.tgt_port == i));
                                self.connections.push(VisualConnection {
                                    src_comp_id: src_id,
                                    src_port,
                                    tgt_comp_id: comp.id,
                                    tgt_port: i,
                                });
                                connection_made = true;
                                break;
                            }
                        }
                        if connection_made { break; }
                    }
                    self.active_wire_drag = None;
                    if connection_made {
                        self.compile();
                    }
                }
            }
        }

        // 4. Delete selected components (only in main canvas)
        if !egui_wants_pointer && self.inspection_path.is_empty()
            && (is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace)) {
                if !self.selected_comp_ids.is_empty() {
                    self.components.retain(|c| !self.selected_comp_ids.contains(&c.id));
                    self.connections.retain(|c| !self.selected_comp_ids.contains(&c.src_comp_id) && !self.selected_comp_ids.contains(&c.tgt_comp_id));
                    self.selected_comp_ids.clear();
                    self.selected_comp_id = None;
                    self.compile();
                } else if let Some(id) = self.selected_comp_id {
                    self.components.retain(|c| c.id != id);
                    self.connections.retain(|c| c.src_comp_id != id && c.tgt_comp_id != id);
                    self.selected_comp_id = None;
                    self.compile();
                }
            }

        // 5. Run continuous simulation ticks with multi-domain clocks (batch-then-propagate)
        if self.is_playing {
            for _ in 0..self.ticks_per_frame {
                self.sim_tick_counter = self.sim_tick_counter.wrapping_add(1);
                
                for clock in &mut self.active_clocks {
                    clock.counter += 1;
                    let half_period = (clock.period / 2).max(1);
                    if clock.counter >= half_period {
                        clock.counter = 0;
                        let current_state = self.simulator.states[clock.gate_idx];
                        self.simulator.set_input(clock.gate_idx, !current_state);
                    }
                }
                
                let max_steps = (self.simulator.gates.len() * 10).max(1000);
                match self.simulator.propagate_events(max_steps) {
                    Ok(_) => self.propagation_error = None,
                    Err(e) => {
                        self.propagation_error = Some(e);
                        break;
                    }
                }
            }
        }
        
        self.last_mouse_pos = mouse_pos_screen;
    }
}
