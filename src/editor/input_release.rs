use crate::engine::ComponentType;
use macroquad::prelude::*;
use super::types::*;
use super::Editor;

impl Editor {
    pub fn handle_canvas_left_release(&mut self, mouse_pos_world: Vec2) {
        self.canvas.dragging_wire = None;
        // If it was an Input component and it was clicked (not dragged far)
        if let Some(comp_id) = self.canvas.dragging_comp_id
            && self.canvas.drag_dist_pixels < 5.0
        {
            let mut comp_type = None;
            for comp in &self.components {
                if comp.id == comp_id {
                    comp_type = Some(comp.comp_type);
                    break;
                }
            }
            if comp_type == Some(ComponentType::Input)
                && let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&comp_id)
            {
                let curr_val = self.engine.simulator.get_state(gate_idx);
                self.engine.simulator.set_input(gate_idx, !curr_val);
            }
        }

        // If drag selection box was active, parse selection
        if let Some(start) = self.canvas.selection_box_start {
            let end = mouse_pos_world;
            let x_min = start.x.min(end.x);
            let x_max = start.x.max(end.x);
            let y_min = start.y.min(end.y);
            let y_max = start.y.max(end.y);

            let box_w = x_max - x_min;
            let box_h = y_max - y_min;

            if box_w > 5.0 || box_h > 5.0 {
                self.canvas.selected_comp_ids.clear();
                self.canvas.selected_connections.clear();
                self.canvas.selected_comp_id = None;
                let box_rect = Rect::new(x_min, y_min, box_w, box_h);

                for comp in &self.components {
                    let comp_rect =
                        Rect::new(comp.pos.x, comp.pos.y, comp.width, comp.height);
                    if comp_rect.overlaps(&box_rect) {
                        self.canvas.selected_comp_ids.insert(comp.id);
                    }
                }

                let wire_bounds = |src_pos: Vec2, tgt_pos: Vec2, offset: f32, tgt_port: usize| -> Rect {
                    let segments = Self::compute_wire_segments_world(src_pos, tgt_pos, offset, tgt_port);
                    let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
                    let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
                    for (a, b) in segments {
                        for p in [a, b] {
                            min_x = min_x.min(p.x);
                            max_x = max_x.max(p.x);
                            min_y = min_y.min(p.y);
                            max_y = max_y.max(p.y);
                        }
                    }
                    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
                };

                let comp_by_id: std::collections::HashMap<usize, &VisualComponent> =
                    self.components.iter().map(|c| (c.id, c)).collect();
                for conn in &self.connections {
                    let (src_comp_opt, tgt_comp_opt) = (
                        comp_by_id.get(&conn.src_comp_id),
                        comp_by_id.get(&conn.tgt_comp_id),
                    );
                    if let (Some(&src), Some(&tgt)) = (src_comp_opt, tgt_comp_opt) {
                        let (_, outputs) = self.get_component_ports_count_with_width(src.comp_type, Some(src.bus_width()));
                        let (inputs, _) = self.get_component_ports_count_with_width(tgt.comp_type, Some(tgt.bus_width()));
                        let src_pos = src.output_port_pos(conn.src_port, outputs);
                        let tgt_pos = tgt.input_port_pos(conn.tgt_port, inputs);

                        let offset = self.get_connection_routing_offset(conn);
                        let wire_rect = wire_bounds(src_pos, tgt_pos, offset, conn.tgt_port);
                        if wire_rect.overlaps(&box_rect) {
                            self.canvas.selected_connections.insert(*conn);
                        }
                    }
                }
                if self.canvas.selected_comp_ids.len() == 1 {
                    self.canvas.selected_comp_id =
                        self.canvas.selected_comp_ids.iter().next().copied();
                }
            } else {
                // Clicked empty space: clear selection
                self.canvas.selected_comp_ids.clear();
                self.canvas.selected_connections.clear();
                self.canvas.selected_comp_id = None;
            }
            self.canvas.selection_box_start = None;
        }

        // End drag
        self.canvas.dragging_comp_id = None;
        self.canvas.drag_start_positions.clear();
        self.canvas.dragging_annotation_idx = None;
        self.canvas.drag_snapshot_pushed = false;

        // Handle wiring connection release
        if let Some((src_id, src_port, src_is_input)) = self.canvas.active_wire_drag {
            let mut connection_made = false;

            if let Some((tgt_id, tgt_port, tgt_is_input)) = self.canvas.hovered_port
                && tgt_is_input != src_is_input
                && tgt_id != src_id
            {
                let (out_id, out_port, in_id, in_port) = if src_is_input {
                    (tgt_id, tgt_port, src_id, src_port)
                } else {
                    (src_id, src_port, tgt_id, tgt_port)
                };

                self.push_history_snapshot();
                let mut is_junction = false;
                for comp in &self.components {
                    if comp.id == in_id
                        && comp.comp_type == crate::engine::ComponentType::Junction
                    {
                        is_junction = true;
                        break;
                    }
                }
                if !is_junction {
                    self.connections
                        .retain(|c| !(c.tgt_comp_id == in_id && c.tgt_port == in_port));
                }
                
                // Deduplicate connection
                self.connections.retain(|c| {
                    !(c.src_comp_id == out_id
                        && c.src_port == out_port
                        && c.tgt_comp_id == in_id
                        && c.tgt_port == in_port)
                });

                self.connections.push(VisualConnection {
                    src_comp_id: out_id,
                    src_port: out_port,
                    tgt_comp_id: in_id,
                    tgt_port: in_port,
                });
                connection_made = true;
            }

            self.canvas.active_wire_drag = None;
            if connection_made {
                self.compile();
            }
        }
    }
}
