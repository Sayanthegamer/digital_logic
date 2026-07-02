use crate::engine::ComponentType;
use macroquad::prelude::*;

use super::Editor;
use super::types::*;

impl Editor {
    pub fn to_world_space(&self, screen_pos: Vec2) -> Vec2 {
        (screen_pos - self.canvas.pan) / self.canvas.zoom
    }

    pub fn to_screen_space(&self, world_pos: Vec2) -> Vec2 {
        world_pos * self.canvas.zoom + self.canvas.pan
    }

    pub fn update(&mut self) {
        let mouse_pos_screen = mouse_position().into();
        let mouse_pos_world = self.to_world_space(mouse_pos_screen);

        let mouse_delta = if self.canvas.last_mouse_pos == Vec2::ZERO {
            Vec2::ZERO
        } else {
            mouse_pos_screen - self.canvas.last_mouse_pos
        };

        let egui_wants_pointer = self.ui.egui_wants_pointer;

        // 1. Touch Input Abstraction (Mobile)
        self.handle_touch_input(egui_wants_pointer);

        // 2. Keyboard & Tool Shortcuts
        self.handle_keyboard_shortcuts(egui_wants_pointer);

        // 3. Magnetic Port Hover Detection
        self.update_hovered_port(mouse_pos_screen, mouse_pos_world, egui_wants_pointer);

        // 4. Zoom with mouse wheel
        self.handle_mouse_zoom(mouse_pos_screen, egui_wants_pointer);

        // 5. Pan with right drag
        self.handle_right_drag_pan(mouse_delta, egui_wants_pointer);

        // 6. Interactions: Left click / drag
        self.handle_canvas_interactions(
            mouse_pos_world,
            mouse_delta,
            egui_wants_pointer,
        );

        // 7. Run continuous simulation ticks
        self.run_simulation_ticks();

        // 8. Resolution Change Revert Timer
        self.update_resolution_revert_timer();

        self.canvas.last_mouse_pos = mouse_pos_screen;
    }

    fn handle_touch_input(&mut self, egui_wants_pointer: bool) {
        let touch_events = touches();
        let is_multi_touch = touch_events.len() >= 2;

        if !egui_wants_pointer && is_multi_touch {
            let t1 = &touch_events[0];
            let t2 = &touch_events[1];

            let current_dist = t1.position.distance(t2.position);
            let current_center = (t1.position + t2.position) * 0.5;

            if let Some(last_dist) = self.canvas.last_touch_dist
                && last_dist > 0.0
            {
                let prev_zoom = self.canvas.zoom;
                self.canvas.zoom *= current_dist / last_dist;
                self.canvas.zoom = self.canvas.zoom.clamp(0.15, 4.0);
                // Zoom towards the center of the pinch
                self.canvas.pan = current_center
                    - (current_center - self.canvas.pan) * (self.canvas.zoom / prev_zoom);
            }

            if let Some(last_center) = self.canvas.last_touch_center {
                let delta = current_center - last_center;
                self.canvas.pan += delta;
            }

            self.canvas.last_touch_dist = Some(current_dist);
            self.canvas.last_touch_center = Some(current_center);

            // Clear single-touch interactions when multi-touch begins
            self.canvas.dragging_comp_id = None;
            self.canvas.active_wire_drag = None;
            self.canvas.selection_box_start = None;
        } else {
            self.canvas.last_touch_dist = None;
            self.canvas.last_touch_center = None;
        }
    }

    fn handle_keyboard_shortcuts(&mut self, egui_wants_pointer: bool) {
        if !egui_wants_pointer {
            if is_key_pressed(KeyCode::Space) {
                self.engine.is_playing = !self.engine.is_playing;
            }
            if is_key_pressed(KeyCode::C) {
                self.compile();
            }
            if is_key_pressed(KeyCode::Escape) {
                self.canvas.selected_tool = None;
                self.canvas.selected_comp_id = None;
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::S)
            {
                self.save_project();
            }
            if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
                && is_key_pressed(KeyCode::L)
            {
                self.load_project();
            }

            // Tool Shortcuts
            if is_key_pressed(KeyCode::Key1) || is_key_pressed(KeyCode::I) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Input));
            }
            if is_key_pressed(KeyCode::Key2) || is_key_pressed(KeyCode::O) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Output));
            }
            if is_key_pressed(KeyCode::Key3) || is_key_pressed(KeyCode::N) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Nand));
            }
            if is_key_pressed(KeyCode::Key4) || is_key_pressed(KeyCode::K) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceComponent(ComponentType::Clock));
            }
            if is_key_pressed(KeyCode::Key5) || is_key_pressed(KeyCode::T) {
                self.canvas.selected_tool = Some(ActiveTool::PlaceAnnotation);
            }
        }
    }

    fn update_hovered_port(&mut self, mouse_pos_screen: Vec2, mouse_pos_world: Vec2, egui_wants_pointer: bool) {
        self.canvas.hovered_port = None;
        if !egui_wants_pointer && self.canvas.inspection_path.is_empty() {
            let mut hovered_chip = false;
            for comp in &self.components {
                // Junctions are connectable across their whole body; don't block snapping when hovering them.
                if comp.comp_type == ComponentType::Junction {
                    continue;
                }

                // Define the chip's inner body (excluding a small margin for ports)
                if mouse_pos_world.x >= comp.pos.x + 8.0
                    && mouse_pos_world.x <= comp.pos.x + comp.width - 8.0
                    && mouse_pos_world.y >= comp.pos.y + 4.0
                    && mouse_pos_world.y <= comp.pos.y + comp.height - 4.0
                {
                    hovered_chip = true;
                    break;
                }
            }

            if !hovered_chip {
                let mut closest_dist = 25.0; // Reduced screen radius for snapping
                for comp in &self.components {
                    // Special-case Junction: allow hovering anywhere along the bar.
                    if comp.comp_type == ComponentType::Junction {
                        let a = self.to_screen_space(comp.input_port_pos(0, 1));
                        let b = self.to_screen_space(comp.output_port_pos(0, 1));
                        let ab = b - a;
                        let denom = ab.length_squared();
                        let t = if denom > 0.0 {
                            ((mouse_pos_screen - a).dot(ab) / denom).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let closest = a + ab * t;
                        let dist = closest.distance(mouse_pos_screen);

                        if dist < closest_dist {
                            closest_dist = dist;
                            // When dragging a wire, prefer the input port; otherwise prefer the output port.
                            let want_input = self.canvas.active_wire_drag.is_some();
                            self.canvas.hovered_port = Some((comp.id, 0, want_input));
                        }
                        continue;
                    }

                    let (inputs_count, outputs_count) =
                        self.get_component_ports_count(comp.comp_type);

                    // Check inputs
                    for i in 0..inputs_count {
                        let port_pos = comp.input_port_pos(i, inputs_count);
                        let screen_port_pos = self.to_screen_space(port_pos);
                        let dist = screen_port_pos.distance(mouse_pos_screen);
                        if dist < closest_dist {
                            closest_dist = dist;
                            self.canvas.hovered_port = Some((comp.id, i, true));
                        }
                    }

                    // Check outputs
                    for o in 0..outputs_count {
                        let port_pos = comp.output_port_pos(o, outputs_count);
                        let screen_port_pos = self.to_screen_space(port_pos);
                        let dist = screen_port_pos.distance(mouse_pos_screen);
                        if dist < closest_dist {
                            closest_dist = dist;
                            self.canvas.hovered_port = Some((comp.id, o, false));
                        }
                    }
                }
            }
        }
    }

    fn handle_mouse_zoom(&mut self, mouse_pos_screen: Vec2, egui_wants_pointer: bool) {
        if !egui_wants_pointer {
            let scroll = mouse_wheel().1;
            if scroll != 0.0 {
                let prev_zoom = self.canvas.zoom;
                if scroll > 0.0 {
                    self.canvas.zoom *= 1.15;
                } else {
                    self.canvas.zoom /= 1.15;
                }
                self.canvas.zoom = self.canvas.zoom.clamp(0.15, 4.0);

                // Pan adjustment to zoom on mouse cursor
                self.canvas.pan = mouse_pos_screen
                    - (mouse_pos_screen - self.canvas.pan) * (self.canvas.zoom / prev_zoom);
            }
        }
    }

    fn handle_right_drag_pan(&mut self, mouse_delta: Vec2, egui_wants_pointer: bool) {
        if !egui_wants_pointer && is_mouse_button_down(MouseButton::Right) {
            self.canvas.pan += mouse_delta;
            self.canvas.selected_tool = None;
        }
    }

    fn handle_canvas_interactions(
        &mut self,
        mouse_pos_world: Vec2,
        mouse_delta: Vec2,
        egui_wants_pointer: bool,
    ) {
        let touch_events = touches();
        let is_multi_touch = touch_events.len() >= 2;

        if !egui_wants_pointer && !is_multi_touch && self.canvas.inspection_path.is_empty() {
            if is_mouse_button_pressed(MouseButton::Left) {
                // Click on a port or component
                let mut clicked_something = false;

                let bypass_wiring = is_key_down(KeyCode::LeftShift)
                    || is_key_down(KeyCode::RightShift)
                    || is_key_down(KeyCode::LeftControl)
                    || is_key_down(KeyCode::RightControl);

                // Check ports first (wiring starts here)
                if !bypass_wiring
                    && let Some((comp_id, port_idx, is_input)) = self.canvas.hovered_port
                    && !is_input
                {
                    self.canvas.active_wire_drag = Some((comp_id, port_idx));
                    clicked_something = true;
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
                            if let Some(c) = self.components.iter().find(|x| x.id == id) {
                                self.canvas.drag_start_positions.insert(id, c.pos);
                                self.canvas
                                    .drag_start_sizes
                                    .insert(id, Vec2::new(c.width, c.height));
                            }
                        }
                        clicked_something = true;
                    } else {
                        // Check if we clicked an annotation
                        let mut clicked_ann = None;
                        for (idx, ann) in self.annotations.iter().enumerate() {
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
                            self.canvas.drag_offset = self.annotations[idx].pos - mouse_pos_world;
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
                                    self.components.iter().map(|c| (c.id, c)).collect();
                                for conn in &self.connections {
                                    let (src_comp_opt, tgt_comp_opt) = (
                                        comp_by_id.get(&conn.src_comp_id),
                                        comp_by_id.get(&conn.tgt_comp_id),
                                    );
                                    if let (Some(&src), Some(&tgt)) = (src_comp_opt, tgt_comp_opt) {
                                        let (_, outputs) =
                                            self.get_component_ports_count(src.comp_type);
                                        let (inputs, _) =
                                            self.get_component_ports_count(tgt.comp_type);
                                        let src_pos = src.output_port_pos(conn.src_port, outputs);
                                        let tgt_pos = tgt.input_port_pos(conn.tgt_port, inputs);

                                        if self.hit_test_manhattan_wire(
                                            src_pos,
                                            tgt_pos,
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
                                _ => 70.0,
                            };

                            if comp_type == ComponentType::Junction {
                                width = 12.0;
                                height = 12.0;
                            }

                            let label = self.get_component_label(comp_type);

                            let clock_period = match comp_type {
                                ComponentType::Clock => Some(20),
                                _ => None,
                            };

                            let new_id = self.next_component_id;
                            let target_pos = mouse_pos_world - Vec2::new(width / 2.0, height / 2.0);
                            let snapped_pos = Vec2::new(
                                (target_pos.x / 20.0).round() * 20.0,
                                (target_pos.y / 20.0).round() * 20.0,
                            );

                            self.components.push(VisualComponent {
                                id: new_id,
                                comp_type,
                                pos: snapped_pos,
                                width,
                                height,
                                label,
                                clock_period,
                            });
                            self.canvas.selected_comp_id = Some(new_id);
                            self.canvas.selected_comp_ids.clear();
                            self.canvas.selected_comp_ids.insert(new_id);
                            self.next_component_id += 1;
                            self.compile();
                        }
                        ActiveTool::PlaceAnnotation => {
                            self.push_history_snapshot();
                            let snapped_pos = Vec2::new(
                                (mouse_pos_world.x / 20.0).round() * 20.0,
                                (mouse_pos_world.y / 20.0).round() * 20.0,
                            );
                            self.annotations.push(TextAnnotation {
                                text: "Double-click to edit".to_string(),
                                pos: snapped_pos,
                            });
                            self.canvas.selected_annotation_idx = Some(self.annotations.len() - 1);
                            self.canvas.selected_comp_id = None;
                        }
                    }
                }
            } else if is_mouse_button_down(MouseButton::Left) {
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

                                // Stretching logic instead of moving
                                // We stretch horizontally or vertically depending on dominant translation
                                if translation.x.abs() > translation.y.abs() {
                                    if is_right {
                                        c.pos.x = start_pos.x;
                                        c.width = (start_size.x + snapped_translation.x).clamp(12.0, 2000.0);
                                    } else {
                                        let new_width =
                                            (start_size.x - snapped_translation.x).clamp(12.0, 2000.0);
                                        let actual_delta = start_size.x - new_width;
                                        c.pos.x = start_pos.x + actual_delta;
                                        c.width = new_width;
                                    }
                                    c.height = start_size.y;
                                    c.pos.y = start_pos.y;
                                } else {
                                    if is_bottom {
                                        c.pos.y = start_pos.y;
                                        c.height = (start_size.y + snapped_translation.y).clamp(12.0, 2000.0);
                                    } else {
                                        let new_height =
                                            (start_size.y - snapped_translation.y).clamp(12.0, 2000.0);
                                        let actual_delta = start_size.y - new_height;
                                        c.pos.y = start_pos.y + actual_delta;
                                        c.height = new_height;
                                    }
                                    c.width = start_size.x;
                                    c.pos.x = start_pos.x;
                                }
                            } else {
                                c.pos = start_pos + snapped_translation;
                            }
                        }
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
            } else if is_mouse_button_released(MouseButton::Left) {
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
                        let max_steps = (self.engine.simulator.gates.len() * 10).max(1000);
                        match self.engine.simulator.propagate_events(max_steps) {
                            Ok(_) => self.engine.propagation_error = None,
                            Err(e) => self.engine.propagation_error = Some(e),
                        }
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

                        let zoom = self.canvas.zoom;
                        let wire_bounds = |src_pos: Vec2, tgt_pos: Vec2| -> Rect {
                            // Mirror the Manhattan routing used by rendering/hit-testing.
                            let mut points: Vec<Vec2> = vec![src_pos, tgt_pos];

                            if tgt_pos.x >= src_pos.x + 20.0 * zoom {
                                let mid_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0;
                                points.push(Vec2::new(mid_x, src_pos.y));
                                points.push(Vec2::new(mid_x, tgt_pos.y));
                            } else {
                                let stub_src = src_pos.x + 20.0 * zoom;
                                let stub_tgt = tgt_pos.x - 20.0 * zoom;

                                let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0;
                                if (tgt_pos.y - src_pos.y).abs() < 10.0 * zoom {
                                    mid_y += 35.0 * zoom;
                                }

                                points.push(Vec2::new(stub_src, src_pos.y));
                                points.push(Vec2::new(stub_src, mid_y));
                                points.push(Vec2::new(stub_tgt, mid_y));
                                points.push(Vec2::new(stub_tgt, tgt_pos.y));
                            }

                            let (mut min_x, mut max_x) = (f32::INFINITY, f32::NEG_INFINITY);
                            let (mut min_y, mut max_y) = (f32::INFINITY, f32::NEG_INFINITY);
                            for p in points {
                                min_x = min_x.min(p.x);
                                max_x = max_x.max(p.x);
                                min_y = min_y.min(p.y);
                                max_y = max_y.max(p.y);
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
                                let (_, outputs) = self.get_component_ports_count(src.comp_type);
                                let (inputs, _) = self.get_component_ports_count(tgt.comp_type);
                                let src_pos = src.output_port_pos(conn.src_port, outputs);
                                let tgt_pos = tgt.input_port_pos(conn.tgt_port, inputs);

                                let wire_rect = wire_bounds(src_pos, tgt_pos);
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
                if let Some((src_id, src_port)) = self.canvas.active_wire_drag {
                    let mut connection_made = false;

                    if let Some((tgt_id, tgt_port, is_input)) = self.canvas.hovered_port
                        && is_input
                        && tgt_id != src_id
                    {
                        self.push_history_snapshot();
                        let mut is_junction = false;
                        for comp in &self.components {
                            if comp.id == tgt_id
                                && comp.comp_type == crate::engine::ComponentType::Junction
                            {
                                is_junction = true;
                                break;
                            }
                        }
                        if !is_junction {
                            self.connections
                                .retain(|c| !(c.tgt_comp_id == tgt_id && c.tgt_port == tgt_port));
                        }
                        self.connections.push(VisualConnection {
                            src_comp_id: src_id,
                            src_port,
                            tgt_comp_id: tgt_id,
                            tgt_port,
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

        // 4. Delete selected components or annotations (only in main canvas)
        if !egui_wants_pointer
            && self.canvas.inspection_path.is_empty()
            && (is_key_pressed(KeyCode::Delete) || is_key_pressed(KeyCode::Backspace))
        {
            if self.canvas.selected_annotation_idx.is_some()
                || !self.canvas.selected_comp_ids.is_empty()
                || !self.canvas.selected_connections.is_empty()
                || self.canvas.selected_comp_id.is_some()
            {
                self.push_history_snapshot();
            }
            if let Some(idx) = self.canvas.selected_annotation_idx {
                if idx < self.annotations.len() {
                    self.annotations.remove(idx);
                }
                self.canvas.selected_annotation_idx = None;
            } else if !self.canvas.selected_comp_ids.is_empty()
                || !self.canvas.selected_connections.is_empty()
            {
                self.components
                    .retain(|c| !self.canvas.selected_comp_ids.contains(&c.id));
                self.connections.retain(|c| {
                    !self.canvas.selected_comp_ids.contains(&c.src_comp_id)
                        && !self.canvas.selected_comp_ids.contains(&c.tgt_comp_id)
                        && !self.canvas.selected_connections.contains(c)
                });
                self.canvas.selected_comp_ids.clear();
                self.canvas.selected_connections.clear();
                self.canvas.selected_comp_id = None;
                self.compile();
            } else if let Some(id) = self.canvas.selected_comp_id {
                self.components.retain(|c| c.id != id);
                self.connections
                    .retain(|c| c.src_comp_id != id && c.tgt_comp_id != id);
                self.canvas.selected_comp_id = None;
                self.compile();
            }
        }
    }

    fn run_simulation_ticks(&mut self) {
        if self.engine.is_playing {
            for _ in 0..self.engine.ticks_per_frame {
                self.engine.sim_tick_counter = self.engine.sim_tick_counter.wrapping_add(1);

                for clock in &mut self.engine.active_clocks {
                    clock.counter += 1;
                    let half_period = (clock.period / 2).max(1);
                    if clock.counter >= half_period {
                        clock.counter = 0;
                        let current_state = self.engine.simulator.get_state(clock.gate_idx);
                        self.engine
                            .simulator
                            .set_input(clock.gate_idx, !current_state);
                    }
                }

                let max_steps = (self.engine.simulator.gates.len() * 10).max(1000);
                match self.engine.simulator.propagate_events(max_steps) {
                    Ok(_) => self.engine.propagation_error = None,
                    Err(e) => {
                        self.engine.propagation_error = Some(e);
                        break;
                    }
                }
            }
        }
    }

    fn update_resolution_revert_timer(&mut self) {
        if let Some(mut timer) = self.ui.resolution_revert_timer {
            timer -= get_frame_time();
            if timer <= 0.0 {
                // Revert to prev settings
                self.ui.is_fullscreen = self.ui.prev_is_fullscreen;
                self.ui.resolution_idx = self.ui.prev_resolution_idx;
                self.ui.temp_is_fullscreen = self.ui.prev_is_fullscreen;
                self.ui.temp_resolution_idx = self.ui.prev_resolution_idx;

                macroquad::window::set_fullscreen(self.ui.is_fullscreen);
                let resolutions = &[
                    (800, 600),
                    (1024, 768),
                    (1280, 720),
                    (1600, 900),
                    (1920, 1080),
                ];
                let r = resolutions[self.ui.resolution_idx];
                macroquad::window::request_new_screen_size(r.0 as f32, r.1 as f32);

                self.ui.resolution_revert_timer = None;
            } else {
                self.ui.resolution_revert_timer = Some(timer);
            }
        }
    }
}
