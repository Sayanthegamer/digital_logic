use crate::engine::ComponentType;
use macroquad::prelude::*;

use super::Editor;

fn draw_rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    if r <= 0.0 {
        draw_rectangle(x, y, w, h, color);
        return;
    }
    // Main horizontal center
    draw_rectangle(x + r, y, w - 2.0 * r, h, color);
    // Left vertical strip
    draw_rectangle(x, y + r, r, h - 2.0 * r, color);
    // Right vertical strip
    draw_rectangle(x + w - r, y + r, r, h - 2.0 * r, color);
    
    // Four corner circles
    draw_circle(x + r, y + r, r, color);
    draw_circle(x + w - r, y + r, r, color);
    draw_circle(x + r, y + h - r, r, color);
    draw_circle(x + w - r, y + h - r, r, color);
}

fn draw_rounded_rect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, thickness: f32, color: Color) {
    let r = r.min(w / 2.0).min(h / 2.0);
    if r <= 0.0 {
        draw_rectangle_lines(x, y, w, h, thickness, color);
        return;
    }
    // Draw 4 straight edge lines
    draw_line(x + r, y, x + w - r, y, thickness, color);
    draw_line(x + r, y + h, x + w - r, y + h, thickness, color);
    draw_line(x, y + r, x, y + h - r, thickness, color);
    draw_line(x + w, y + r, x + w, y + h - r, thickness, color);
    
    // Draw 4 corner arcs
    let segments = 6;
    let draw_arc = |cx: f32, cy: f32, start_angle: f32, end_angle: f32| {
        let mut last_point = Vec2::new(
            cx + r * start_angle.cos(),
            cy + r * start_angle.sin()
        );
        for i in 1..=segments {
            let angle = start_angle + (end_angle - start_angle) * (i as f32 / segments as f32);
            let point = Vec2::new(cx + r * angle.cos(), cy + r * angle.sin());
            draw_line(last_point.x, last_point.y, point.x, point.y, thickness, color);
            last_point = point;
        }
    };
    
    use std::f32::consts::PI;
    draw_arc(x + r, y + r, PI, 1.5 * PI);
    draw_arc(x + w - r, y + r, 1.5 * PI, 2.0 * PI);
    draw_arc(x + w - r, y + h - r, 0.0, 0.5 * PI);
    draw_arc(x + r, y + h - r, 0.5 * PI, PI);
}

impl Editor {
    fn draw_manhattan_wire_segments(src_pos: Vec2, tgt_pos: Vec2, thickness: f32, color: Color, zoom: f32) {
        if tgt_pos.x >= src_pos.x + 20.0 * zoom {
            let mid_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0;
            draw_line(src_pos.x, src_pos.y, mid_x, src_pos.y, thickness, color);
            draw_line(mid_x, src_pos.y, mid_x, tgt_pos.y, thickness, color);
            draw_line(mid_x, tgt_pos.y, tgt_pos.x, tgt_pos.y, thickness, color);
        } else {
            let stub_src = src_pos.x + 20.0 * zoom;
            let stub_tgt = tgt_pos.x - 20.0 * zoom;
            
            let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0;
            if (tgt_pos.y - src_pos.y).abs() < 10.0 * zoom {
                mid_y += 35.0 * zoom;
            }
            
            draw_line(src_pos.x, src_pos.y, stub_src, src_pos.y, thickness, color);
            draw_line(stub_src, src_pos.y, stub_src, mid_y, thickness, color);
            draw_line(stub_src, mid_y, stub_tgt, mid_y, thickness, color);
            draw_line(stub_tgt, mid_y, stub_tgt, tgt_pos.y, thickness, color);
            draw_line(stub_tgt, tgt_pos.y, tgt_pos.x, tgt_pos.y, thickness, color);
        }
    }

    pub(crate) fn draw_manhattan_wire(&self, src_pos: Vec2, tgt_pos: Vec2, wire_state: bool) {
        let color = if wire_state {
            Color::new(0.00, 0.70, 1.00, 1.0) // electric cyan
        } else {
            Color::new(0.24, 0.27, 0.30, 1.0) // muted slate gray
        };
        let thickness = if wire_state { 2.2 } else { 1.3 } * self.zoom;
        
        // Active glow bloom effect under active wires
        if wire_state {
            let glow_color = Color::new(0.00, 0.70, 1.00, 0.15);
            let glow_thickness = thickness + 4.0 * self.zoom;
            Self::draw_manhattan_wire_segments(src_pos, tgt_pos, glow_thickness, glow_color, self.zoom);
        }
        
        Self::draw_manhattan_wire_segments(src_pos, tgt_pos, thickness, color, self.zoom);
        
        // Draw concentric terminal circle/indicator at target
        let port_radius = 4.0 * self.zoom;
        if wire_state {
            draw_circle(tgt_pos.x, tgt_pos.y, port_radius + 2.0 * self.zoom, Color::new(0.00, 0.70, 1.00, 0.2));
        }
        draw_circle(tgt_pos.x, tgt_pos.y, port_radius, color);
        draw_circle(tgt_pos.x, tgt_pos.y, 2.0 * self.zoom, Color::new(0.09, 0.10, 0.12, 1.0));
    }

    pub fn draw(&mut self) {
        // Clear background with dark slate-navy
        clear_background(Color::new(0.09, 0.10, 0.12, 1.0));

        // Draw grid - performance optimized camera fading
        if self.zoom >= 0.25 {
            let cell_size = 40.0 * self.zoom;
            let offset_x = self.pan.x % cell_size;
            let offset_y = self.pan.y % cell_size;
            
            let grid_alpha = if self.zoom < 0.6 {
                ((self.zoom - 0.25) / 0.35) * 0.15
            } else {
                0.15
            };
            
            let grid_color = Color::new(0.16, 0.18, 0.20, grid_alpha);

            for x in (0..=(screen_width() as i32 / cell_size as i32 + 1)).map(|i| i as f32 * cell_size + offset_x) {
                draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
            }
            for y in (0..=(screen_height() as i32 / cell_size as i32 + 1)).map(|i| i as f32 * cell_size + offset_y) {
                draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
            }
        }

        if !self.inspection_path.is_empty() {
            self.draw_inspection_view();
            return;
        }

        // 1. Draw Wires / Connections
        for wire in &self.connections {
            let src_comp = self.components.iter().find(|c| c.id == wire.src_comp_id);
            let tgt_comp = self.components.iter().find(|c| c.id == wire.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (_, src_outputs) = self.get_component_ports_count(src.comp_type);
                let (tgt_inputs, _) = self.get_component_ports_count(tgt.comp_type);

                let src_pos = self.to_screen_space(src.output_port_pos(wire.src_port, src_outputs));
                let tgt_pos = self.to_screen_space(tgt.input_port_pos(wire.tgt_port, tgt_inputs));

                // Wire frustum culling: skip drawing wire if it is completely off-screen
                let pad = 50.0 * self.zoom;
                let min_x = src_pos.x.min(tgt_pos.x) - pad;
                let max_x = src_pos.x.max(tgt_pos.x) + pad;
                let min_y = src_pos.y.min(tgt_pos.y) - pad;
                let max_y = src_pos.y.max(tgt_pos.y) + pad;
                
                if max_x < 0.0 || min_x > screen_width() || max_y < 0.0 || min_y > screen_height() {
                    continue;
                }

                // Query state using port mapping table
                let wire_state = if let Some(&gate_idx) = self.port_to_sim_gate_map.get(&(wire.src_comp_id, wire.src_port)) {
                    self.simulator.get_state(gate_idx)
                } else if src.comp_type == ComponentType::Input {
                    if let Some(&gate_idx) = self.visual_to_sim_map.get(&src.id) {
                        self.simulator.get_state(gate_idx)
                    } else {
                        false
                    }
                } else {
                    false
                };

                self.draw_manhattan_wire(src_pos, tgt_pos, wire_state);
            }
        }

        // Draw active wire drag preview
        if let Some((src_id, src_port)) = self.active_wire_drag
            && let Some(src) = self.components.iter().find(|c| c.id == src_id) {
                let (_, src_outputs) = self.get_component_ports_count(src.comp_type);
                let start_pos = self.to_screen_space(src.output_port_pos(src_port, src_outputs));
                let mouse_pos: Vec2 = mouse_position().into();
                
                draw_line(
                    start_pos.x,
                    start_pos.y,
                    mouse_pos.x,
                    mouse_pos.y,
                    2.0,
                    Color::new(0.5, 0.8, 1.0, 0.6), // Light blue preview wire
                );
            }

        // 1.5. Draw Text Annotations
        for (idx, ann) in self.annotations.iter().enumerate() {
            let screen_pos = self.to_screen_space(ann.pos);
            let font_size = (15.0 * self.zoom).max(8.0);
            
            // Frustum culling for annotations
            let text_w = measure_text(&ann.text, None, font_size as u16, 1.0).width;
            if screen_pos.x + text_w < 0.0
                || screen_pos.x > screen_width()
                || screen_pos.y < 0.0
                || screen_pos.y - font_size > screen_height()
            {
                continue;
            }

            let is_selected = self.selected_annotation_idx == Some(idx);
            let color = if is_selected {
                Color::new(0.3, 0.75, 1.0, 0.95)
            } else {
                Color::new(0.7, 0.73, 0.75, 0.8)
            };
            draw_text(&ann.text, screen_pos.x, screen_pos.y, font_size, color);
            
            if is_selected {
                let pad = 4.0 * self.zoom;
                draw_rectangle_lines(
                    screen_pos.x - pad,
                    screen_pos.y - font_size - pad + 3.0 * self.zoom,
                    text_w + pad * 2.0,
                    font_size + pad * 2.0,
                    1.5 * self.zoom,
                    Color::new(0.3, 0.75, 1.0, 0.6),
                );
            }
        }

        // 2. Draw Components
        for comp in &self.components {
            let screen_pos = self.to_screen_space(comp.pos);
            let comp_width = comp.width * self.zoom;
            let comp_height = comp.height * self.zoom;

            // Frustum Culling for components
            if screen_pos.x + comp_width < 0.0
                || screen_pos.x > screen_width()
                || screen_pos.y + comp_height < 0.0
                || screen_pos.y > screen_height()
            {
                continue;
            }

            // Determine body color based on component type and activity
            let is_input_active = if comp.comp_type == ComponentType::Input {
                if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp.id) {
                    self.simulator.get_state(gate_idx)
                } else {
                    false
                }
            } else if comp.comp_type == ComponentType::Output {
                let mut output_active = false;
                if let Some(&gate_idx) = self.visual_to_sim_map.get(&comp.id) {
                    output_active = self.simulator.get_state(gate_idx);
                }
                output_active
            } else {
                false
            };

            let bg_color = Color::new(0.12, 0.13, 0.15, 0.95);
            let border_color = Color::new(0.20, 0.23, 0.26, 1.0);

            // Draw component box with rounded corners and drop shadow
            let corner_radius = 6.0 * self.zoom;
            
            // Drop shadow
            draw_rounded_rect(
                screen_pos.x + 3.0 * self.zoom,
                screen_pos.y + 3.0 * self.zoom,
                comp_width,
                comp_height,
                corner_radius,
                Color::new(0.0, 0.0, 0.0, 0.25),
            );
            
            // Background & Border
            draw_rounded_rect(screen_pos.x, screen_pos.y, comp_width, comp_height, corner_radius, bg_color);
            draw_rounded_rect_lines(screen_pos.x, screen_pos.y, comp_width, comp_height, corner_radius, 1.5 * self.zoom, border_color);

            // Draw Top Accent Stripe
            let accent_color = match comp.comp_type {
                ComponentType::Nand => Color::new(1.0, 0.55, 0.15, 1.0), // Amber orange
                ComponentType::Clock => Color::new(0.00, 0.70, 1.00, 1.0), // Electric sky blue
                ComponentType::Input | ComponentType::Output => {
                    if is_input_active {
                        Color::new(0.15, 0.85, 0.40, 1.0) // Active green
                    } else {
                        Color::new(0.35, 0.38, 0.40, 1.0) // Muted gray
                    }
                }
                ComponentType::SubChip(_) => Color::new(0.40, 0.45, 0.85, 1.0), // Royal indigo
            };
            let stripe_height = 4.0 * self.zoom;
            draw_rounded_rect(screen_pos.x, screen_pos.y, comp_width, stripe_height, corner_radius, accent_color);
            // Re-fill bottom half of accent to keep it a top bar
            draw_rectangle(screen_pos.x, screen_pos.y + stripe_height / 2.0, comp_width, stripe_height / 2.0, accent_color);

            // Draw glowing selection border if selected
            if self.selected_comp_id == Some(comp.id) {
                let offset = 3.0 * self.zoom;
                draw_rounded_rect_lines(
                    screen_pos.x - offset,
                    screen_pos.y - offset,
                    comp_width + offset * 2.0,
                    comp_height + offset * 2.0,
                    corner_radius + offset,
                    1.5 * self.zoom,
                    Color::new(0.00, 0.70, 1.00, 0.8), // Glowing cyan
                );
            }

            // Draw text label
            let font_size = (13.0 * self.zoom).max(6.0);
            let text_size = measure_text(&comp.label, None, font_size as u16, 1.0);
            let text_x = screen_pos.x + (comp_width - text_size.width) / 2.0;
            let text_y = screen_pos.y + (comp_height + text_size.height) / 2.0;
            
            let text_color = if is_input_active {
                Color::new(0.85, 1.0, 0.90, 1.0)
            } else {
                Color::new(0.85, 0.88, 0.90, 1.0)
            };
            draw_text(&comp.label, text_x, text_y, font_size, text_color);

            // Draw port circles
            let (inputs_count, outputs_count) = self.get_component_ports_count(comp.comp_type);
            let port_radius = 4.0 * self.zoom;
            
            // Input ports on left
            for i in 0..inputs_count {
                let port_pos = self.to_screen_space(comp.input_port_pos(i, inputs_count));
                
                let mut input_active = false;
                for wire in &self.connections {
                    if wire.tgt_comp_id == comp.id && wire.tgt_port == i {
                        if let Some(&gate_idx) = self.port_to_sim_gate_map.get(&(wire.src_comp_id, wire.src_port)) {
                            input_active = self.simulator.get_state(gate_idx);
                        } else if let Some(src_comp) = self.components.iter().find(|c| c.id == wire.src_comp_id)
                            && src_comp.comp_type == ComponentType::Input
                                && let Some(&gate_idx) = self.visual_to_sim_map.get(&src_comp.id) {
                                    input_active = self.simulator.get_state(gate_idx);
                                }
                    }
                }

                let port_color = if input_active {
                    Color::new(0.00, 0.70, 1.00, 1.0) // Electric cyan
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0) // Muted slate gray
                };
                
                if input_active {
                    draw_circle(port_pos.x, port_pos.y, port_radius + 2.0 * self.zoom, Color::new(0.00, 0.70, 1.00, 0.2));
                }
                draw_circle(port_pos.x, port_pos.y, port_radius, port_color);
                draw_circle(port_pos.x, port_pos.y, 2.0 * self.zoom, Color::new(0.12, 0.13, 0.15, 1.0));
            }
            
            // Output ports on right
            for o in 0..outputs_count {
                let port_pos = self.to_screen_space(comp.output_port_pos(o, outputs_count));
                
                let output_active = if let Some(&gate_idx) = self.port_to_sim_gate_map.get(&(comp.id, o)) {
                    self.simulator.get_state(gate_idx)
                } else if comp.comp_type == ComponentType::Input {
                    is_input_active
                } else {
                    false
                };

                let port_color = if output_active {
                    Color::new(0.00, 0.70, 1.00, 1.0) // Electric cyan
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0) // Muted slate gray
                };

                if output_active {
                    draw_circle(port_pos.x, port_pos.y, port_radius + 2.0 * self.zoom, Color::new(0.00, 0.70, 1.00, 0.2));
                }
                draw_circle(port_pos.x, port_pos.y, port_radius, port_color);
                draw_circle(port_pos.x, port_pos.y, 2.0 * self.zoom, Color::new(0.12, 0.13, 0.15, 1.0));
            }

            // Draw custom port names inside sub-chip boundary boxes
            if let ComponentType::SubChip(idx) = comp.comp_type
                && let Some(bp) = self.library.get(idx) {
                    let text_size_px = (10.0 * self.zoom).max(5.0);
                    for i in 0..inputs_count {
                        let port_pos = self.to_screen_space(comp.input_port_pos(i, inputs_count));
                        let name = bp.input_names.get(i).cloned().unwrap_or_else(|| format!("{}", i));
                        draw_text(&name, port_pos.x + 6.0 * self.zoom, port_pos.y + 3.0 * self.zoom, text_size_px, Color::new(0.5, 0.55, 0.6, 1.0));
                    }
                    for o in 0..outputs_count {
                        let port_pos = self.to_screen_space(comp.output_port_pos(o, outputs_count));
                        let name = bp.output_names.get(o).cloned().unwrap_or_else(|| format!("{}", o));
                        let text_w = measure_text(&name, None, text_size_px as u16, 1.0).width;
                        draw_text(&name, port_pos.x - 6.0 * self.zoom - text_w, port_pos.y + 3.0 * self.zoom, text_size_px, Color::new(0.5, 0.55, 0.6, 1.0));
                    }
                }
        }

        // Draw instructions at top-left
        draw_text("Left Click: Place/Connect/Toggle | Drag: Move | Right Click/Del: Delete | Scroll: Zoom | Right Drag: Pan", 15.0, 20.0, 14.0, Color::new(0.6, 0.65, 0.7, 0.8));
    }
}
