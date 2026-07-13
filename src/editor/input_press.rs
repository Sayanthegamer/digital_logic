use crate::engine::ComponentType;
use macroquad::prelude::*;
use super::types::*;
use super::Editor;

impl Editor {
    pub fn handle_canvas_left_press(&mut self, mouse_pos_world: Vec2) {
        // Click on a port or component
        let mut clicked_something = false;

        let bypass_wiring = is_key_down(KeyCode::LeftShift)
            || is_key_down(KeyCode::RightShift)
            || is_key_down(KeyCode::LeftControl)
            || is_key_down(KeyCode::RightControl);

        // Check ports first (wiring starts here)
        if !bypass_wiring
            && let Some((comp_id, port_idx, is_input)) = self.canvas.hovered_port
        {
            self.canvas.active_wire_drag = Some((comp_id, port_idx, is_input));
            clicked_something = true;
        }

        // Check clicking inside components (dragging, toggling)
        if !clicked_something {
            let mut found_comp = None;
            for comp in self.circuit.components.iter().rev() {
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
                self.canvas.selected_comp_id = Some(comp.id);
                self.canvas.selected_annotation_idx = None;
                self.canvas.last_clicked_annotation_idx = None;

                // Handle multi-selection tracking
                if self.canvas.selected_comp_ids.contains(&comp.id) {
                    // Already selected, keep multi-selection
                } else if is_key_down(KeyCode::LeftShift)
                    || is_key_down(KeyCode::RightShift)
                {
                    self.canvas.selected_comp_ids.insert(comp.id);
                } else {
                    self.canvas.selected_comp_ids.clear();
                    self.canvas.selected_connections.clear();
                    self.canvas.selected_comp_ids.insert(comp.id);
                }

                // Start dragging (Input components are also draggable now!)
                self.canvas.dragging_comp_id = Some(comp.id);
                self.canvas.drag_offset = mouse_pos_world;
                self.canvas.drag_dist_pixels = 0.0;
                self.canvas.drag_snapshot_pushed = false;

                // Store starting positions of all selected components
                self.canvas.drag_start_positions.clear();
                self.canvas.drag_start_sizes.clear();
                for &id in &self.canvas.selected_comp_ids {
                    let comp_data = self.get_component(id).map(|c| (c.pos, Vec2::new(c.width, c.height)));
                    if let Some((pos, size)) = comp_data {
                        self.canvas.drag_start_positions.insert(id, pos);
                        self.canvas.drag_start_sizes.insert(id, size);
                    }
                }
                clicked_something = true;
            } else {
                // Check if we clicked an annotation
                let mut clicked_ann = None;
                for (idx, ann) in self.circuit.annotations.iter().enumerate() {
                    let text_w = measure_text(&ann.text, self.font.as_ref(), 15, 1.0).width;
                    let rect =
                        Rect::new(ann.pos.x - 5.0, ann.pos.y - 14.0, text_w + 10.0, 20.0);
                    if rect.contains(mouse_pos_world) {
                        clicked_ann = Some(idx);
                        break;
                    }
                }

                if let Some(idx) = clicked_ann {
                    let now = get_time();
                    if Some(idx) == self.canvas.last_clicked_annotation_idx
                        && now - self.canvas.last_click_time < 0.3
                    {
                        self.canvas.focus_annotation_text = true;
                    }
                    self.canvas.last_click_time = now;
                    self.canvas.last_clicked_annotation_idx = Some(idx);

                    self.canvas.selected_annotation_idx = Some(idx);
                    self.canvas.dragging_annotation_idx = Some(idx);
                    self.canvas.selected_comp_id = None;
                    self.canvas.selected_comp_ids.clear();
                    self.canvas.selected_connections.clear();
                    self.canvas.drag_offset = self.circuit.annotations[idx].pos - mouse_pos_world;
                    clicked_something = true;
                } else {
                    // Clicked empty space
                    self.canvas.selected_comp_id = None;
                    self.canvas.selected_annotation_idx = None;
                    self.canvas.last_clicked_annotation_idx = None;

                    // Start drag selection box if no placement tool is active
                    if self.canvas.selected_tool.is_none() {
                        // Check if a wire was clicked
                        let mut clicked_wire = None;
                        let comp_by_id: std::collections::HashMap<usize, &VisualComponent> =
                            self.circuit.components.iter().map(|c| (c.id, c)).collect();
                        for conn in &self.circuit.connections {
                            let (src_comp_opt, tgt_comp_opt) = (
                                comp_by_id.get(&conn.src_comp_id),
                                comp_by_id.get(&conn.tgt_comp_id),
                            );
                            if let (Some(&src), Some(&tgt)) = (src_comp_opt, tgt_comp_opt) {
                                let (_, outputs) =
                                    self.get_component_ports_count_with_width(src.comp_type, Some(src.bus_width()));
                                let (inputs, _) =
                                    self.get_component_ports_count_with_width(tgt.comp_type, Some(tgt.bus_width()));
                                let src_pos = src.output_port_pos(conn.src_port, outputs);
                                let tgt_pos = tgt.input_port_pos(conn.tgt_port, inputs);

                                let offset = self.get_connection_routing_offset(conn);
                                if self.hit_test_manhattan_wire(
                                    src_pos,
                                    tgt_pos,
                                    offset,
                                    conn.tgt_port,
                                    mouse_pos_world,
                                    10.0 / self.canvas.zoom,
                                ) {
                                    clicked_wire = Some(*conn);
                                    break;
                                }
                            }
                        }

                        if let Some(wire) = clicked_wire {
                            if is_key_down(KeyCode::LeftShift)
                                || is_key_down(KeyCode::RightShift)
                            {
                                self.canvas.selected_connections.insert(wire);
                            } else {
                                self.canvas.selected_comp_ids.clear();
                                self.canvas.selected_connections.clear();
                                self.canvas.selected_connections.insert(wire);
                            }
                            clicked_something = true;
                            self.canvas.dragging_wire = Some(wire);
                        } else {
                            self.canvas.selected_comp_ids.clear();
                            self.canvas.selected_connections.clear();
                            self.canvas.selection_box_start = Some(mouse_pos_world);
                        }
                    }
                }
            }
        }

        // If nothing was clicked and a tool is active, place the component or annotation
        if !clicked_something && let Some(tool) = self.canvas.selected_tool {
            match tool {
                ActiveTool::PlaceComponent(comp_type) => {
                    self.push_history_snapshot();
                    let (inputs, outputs) = self.get_component_ports_count(comp_type);
                    let max_ports = inputs.max(outputs);
                    let mut height = 40.0 + (max_ports as f32 * 16.0);
                    let mut width = match comp_type {
                        ComponentType::SubChip(_) => 100.0,
                        ComponentType::BusJoiner | ComponentType::BusSplitter => 50.0,
                        _ => 70.0,
                    };

                    if comp_type == ComponentType::Junction {
                        width = 12.0;
                        height = 12.0;
                    }

                    let label = self.get_component_label(comp_type);

                    let new_id = self.circuit.next_component_id;
                    let target_pos = mouse_pos_world - Vec2::new(width / 2.0, height / 2.0);
                    let snapped_pos = Vec2::new(
                        (target_pos.x / 20.0).round() * 20.0,
                        (target_pos.y / 20.0).round() * 20.0,
                    );
                    let clock_period = match comp_type {
                        ComponentType::Clock => Some(20),
                        _ => None,
                    };
                    let bus_width = match comp_type {
                        ComponentType::BusJoiner | ComponentType::BusSplitter => Some(4),
                        _ => None,
                    };

                    self.circuit.components.push(VisualComponent {
                        id: new_id,
                        comp_type,
                        pos: snapped_pos,
                        width,
                        height,
                        label,
                        clock_period,
                        bus_width,
                        color: None,
                    });
                    self.canvas.selected_comp_id = Some(new_id);
                    self.canvas.selected_comp_ids.clear();
                    self.canvas.selected_comp_ids.insert(new_id);
                    self.circuit.next_component_id += 1;
                    self.compile();
                }
                ActiveTool::PlaceAnnotation => {
                    self.push_history_snapshot();
                    let _snapped_pos = Vec2::new(
                        (mouse_pos_world.x / 20.0).round() * 20.0,
                        (mouse_pos_world.y / 20.0).round() * 20.0,
                    );
                    self.circuit.annotations.push(TextAnnotation {
                        text: "Text Note".to_string(),
                        pos: mouse_pos_world,
                    });
                    self.canvas.selected_annotation_idx = Some(self.circuit.annotations.len() - 1);
                    self.canvas.selected_comp_id = None;
                }
            }
        }
    }
}
