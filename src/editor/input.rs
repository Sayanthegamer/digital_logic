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
        }

        // 3. Interactions: Left click / drag (only in main canvas)
        if !egui_wants_pointer && self.inspection_path.is_empty() {
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
                        if comp.comp_type == ComponentType::Input {
                            // Toggle Input state directly in simulator using mapping table without full recompile
                            if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp.id) {
                                let curr_val = self.simulator.get_state(gate_idx);
                                self.simulator.set_input(gate_idx, !curr_val);
                                let max_steps = (self.simulator.gates.len() * 10).max(1000);
                                match self.simulator.propagate_events(max_steps) {
                                    Ok(_) => self.propagation_error = None,
                                    Err(e) => self.propagation_error = Some(e),
                                }
                            }
                            clicked_something = true;
                        } else {
                            // Start dragging
                            self.dragging_comp_id = Some(comp.id);
                            self.drag_offset = comp.pos - mouse_pos_world;
                            clicked_something = true;
                        }
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
                            self.drag_offset = self.annotations[idx].pos - mouse_pos_world;
                            clicked_something = true;
                        } else {
                            // Clicked empty space
                            self.selected_comp_id = None;
                            self.selected_annotation_idx = None;
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
                // Drag component
                if let Some(comp_id) = self.dragging_comp_id {
                    if let Some(comp) = self.components.iter_mut().find(|c| c.id == comp_id) {
                        let target_pos = mouse_pos_world + self.drag_offset;
                        comp.pos = Vec2::new((target_pos.x / 20.0).round() * 20.0, (target_pos.y / 20.0).round() * 20.0);
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
                // End drag
                self.dragging_comp_id = None;
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

        // 4. Delete selected component (only in main canvas)
        if !egui_wants_pointer && self.inspection_path.is_empty() {
            if is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace) {
                if let Some(id) = self.selected_comp_id {
                    self.components.retain(|c| c.id != id);
                    self.connections.retain(|c| c.src_comp_id != id && c.tgt_comp_id != id);
                    self.selected_comp_id = None;
                    self.compile();
                }
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
