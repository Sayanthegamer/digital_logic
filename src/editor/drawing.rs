use crate::editor::theme;
use crate::engine::ComponentType;
use macroquad::prelude::*;

use crate::editor::types::VisualConnection;
use super::Editor;

use crate::editor::drawing_shapes::*;

#[inline]
fn get_seg_color(active: bool) -> Color {
    if active {
        theme::COMP_SEVENSEG.mq()
    } else {
        theme::COMP_SEVENSEG.mq_with_alpha(0.1)
    }
}

impl Editor {
    pub fn is_bus_connection(&self, conn: &VisualConnection) -> bool {
        let src_comp = self.get_component(conn.src_comp_id);
        let tgt_comp = self.get_component(conn.tgt_comp_id);
        src_comp.map_or(false, |c| c.comp_type == ComponentType::BusJoiner && conn.src_port == 0)
            || tgt_comp.map_or(false, |c| c.comp_type == ComponentType::BusSplitter && conn.tgt_port == 0)
    }

    pub fn draw(&mut self) {
        // Clear background with dark slate-navy
        clear_background(theme::BG_CANVAS.mq());

        // Draw grid - performance optimized camera fading
        if self.canvas.zoom >= 0.25 {
            let cell_size = 40.0 * self.canvas.zoom;
            let offset_x = self.canvas.pan.x % cell_size;
            let offset_y = self.canvas.pan.y % cell_size;

            let grid_alpha = if self.canvas.zoom < 0.6 {
                ((self.canvas.zoom - 0.25) / 0.35) * 0.15
            } else {
                0.15
            };

            let grid_color = theme::BORDER.mq_with_alpha(grid_alpha);

            for x in (0..=(screen_width() as i32 / cell_size as i32 + 1))
                .map(|i| i as f32 * cell_size + offset_x)
            {
                draw_line(x, 0.0, x, screen_height(), 1.0, grid_color);
            }
            for y in (0..=(screen_height() as i32 / cell_size as i32 + 1))
                .map(|i| i as f32 * cell_size + offset_y)
            {
                draw_line(0.0, y, screen_width(), y, 1.0, grid_color);
            }
        }

        if !self.canvas.inspection_path.is_empty() {
            self.draw_inspection_view();
            return;
        }

        let top_left = self.to_world_space(vec2(0.0, 0.0));
        let bottom_right = self.to_world_space(vec2(screen_width(), screen_height()));
        let viewport_pad = 500.0;
        let viewport_rect = Rect::new(
            top_left.x - viewport_pad,
            top_left.y - viewport_pad,
            (bottom_right.x - top_left.x) + viewport_pad * 2.0,
            (bottom_right.y - top_left.y) + viewport_pad * 2.0,
        );

        // 0. Build O(1) lookup map for components to avoid O(N^2) rendering bottleneck
        let mut comp_map = std::collections::HashMap::with_capacity(self.circuit.components.len());
        for comp in &self.circuit.components {
            comp_map.insert(comp.id, comp);
        }

        // Precompute wire intersections so we can skip drawing the upper wire line through bridge arcs
        let intersections = self.find_wire_intersections();
        let mut upper_gaps: std::collections::HashMap<crate::editor::types::VisualConnection, Vec<(Vec2, f32)>> = std::collections::HashMap::new();
        for int in &intersections {
            if int.junction_type == crate::editor::wire_junctions::JunctionType::Crossing {
                if let Some(upper) = int.upper_conn {
                    upper_gaps.entry(upper).or_default().push((int.point, int.lower_thickness));
                }
            }
        }

        // 1. Draw Wires / Connections
        let visible_wires = self.canvas.wire_spatial_grid.query_rect(viewport_rect);
        
        for wire in &self.circuit.connections {
            if !visible_wires.contains(wire) {
                continue;
            }
            let src_comp = comp_map.get(&wire.src_comp_id).copied();
            let tgt_comp = comp_map.get(&wire.tgt_comp_id).copied();

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (src_p, tgt_p) = self.get_connection_ports(wire, src, tgt);
                let src_pos = self.to_screen_space(src_p);
                let tgt_pos = self.to_screen_space(tgt_p);

                // Query state using port mapping table
                let wire_state = if let Some(&gate_idx) = self
                    .engine
                    .port_to_sim_gate_map
                    .get(&(wire.src_comp_id, wire.src_port))
                {
                    self.engine.simulator.get_raw_state(gate_idx)
                } else if src.comp_type == ComponentType::Input {
                    if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&src.id) {
                        self.engine.simulator.get_raw_state(gate_idx)
                    } else {
                        0b00
                    }
                } else {
                    0b00
                };

                let is_selected = self.canvas.selected_connections.contains(wire);
                let wire_color_override = self.circuit.color_overrides.get_wire_color(wire);
                let offset = self.get_connection_routing_offset(wire);
                let is_bus = self.is_bus_connection(wire);
                let empty_gaps = &[];
                let gaps = upper_gaps.get(wire).map(|v| v.as_slice()).unwrap_or(empty_gaps);

                self.draw_manhattan_wire(
                    src_pos,
                    tgt_pos,
                    offset,
                    wire.tgt_port,
                    wire_state,
                    is_selected,
                    wire_color_override,
                    is_bus,
                    gaps,
                );
            }
        }

        // 1.1 Draw Wire Junction/Crossing Indicators
        self.draw_wire_junctions(&intersections);

        // Draw active wire drag preview
        if let Some((src_id, src_port, src_is_input)) = self.canvas.active_wire_drag
            && let Some(src) = self.get_component(src_id)
        {
            let (src_inputs, src_outputs) = self.get_component_ports_count_with_width(src.comp_type, Some(src.bus_width()));
            let start_pos = if src_is_input {
                self.to_screen_space(src.input_port_pos(src_port, src_inputs))
            } else {
                self.to_screen_space(src.output_port_pos(src_port, src_outputs))
            };
            let mut end_pos: Vec2 = mouse_position().into();

            // Magnetic Snapping
            if let Some((tgt_id, tgt_port, tgt_is_input)) = self.canvas.hovered_port
                && tgt_is_input != src_is_input
                && tgt_id != src_id
                && let Some(tgt_comp) = self.get_component(tgt_id)
            {
                let (tgt_inputs, tgt_outputs) = self.get_component_ports_count_with_width(tgt_comp.comp_type, Some(tgt_comp.bus_width()));
                end_pos = if tgt_is_input {
                    self.to_screen_space(tgt_comp.input_port_pos(tgt_port, tgt_inputs))
                } else {
                    self.to_screen_space(tgt_comp.output_port_pos(tgt_port, tgt_outputs))
                };
            }

            draw_line(
                start_pos.x,
                start_pos.y,
                end_pos.x,
                end_pos.y,
                3.0 * self.canvas.zoom,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.8), // Light blue preview wire
            );

            // Draw end circle
            draw_circle(
                end_pos.x,
                end_pos.y,
                4.0 * self.canvas.zoom,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.8),
            );
        }

        // 1.5. Draw Text Annotations
        for (idx, ann) in self.circuit.annotations.iter().enumerate() {
            let screen_pos = self.to_screen_space(ann.pos);
            let target_font_size = (15.0 * self.canvas.zoom).max(8.0);
            let base_font_size = 32.0;
            let font_scale = target_font_size / base_font_size;

            // Frustum culling for annotations
            let text_w = measure_text(
                &ann.text,
                self.font.as_ref(),
                base_font_size as u16,
                font_scale,
            )
            .width;
            if screen_pos.x + text_w < 0.0
                || screen_pos.x > screen_width()
                || screen_pos.y < 0.0
                || screen_pos.y - target_font_size > screen_height()
            {
                continue;
            }

            let is_selected = self.canvas.selected_annotation_idx == Some(idx);
            let color = if is_selected {
                theme::ACCENT_PRIMARY.mq_with_alpha(0.95)
            } else {
                theme::TEXT_SECONDARY.mq_with_alpha(0.8)
            };
            draw_text_ex(
                &ann.text,
                screen_pos.x,
                screen_pos.y,
                TextParams {
                    font: self.font.as_ref(),
                    font_size: base_font_size as u16,
                    font_scale,
                    color,
                    ..Default::default()
                },
            );

            if is_selected {
                let pad = 4.0 * self.canvas.zoom;
                draw_rectangle_lines(
                    screen_pos.x - pad,
                    screen_pos.y - target_font_size - pad + 3.0 * self.canvas.zoom,
                    text_w + pad * 2.0,
                    target_font_size + pad * 2.0,
                    1.5 * self.canvas.zoom,
                    theme::ACCENT_PRIMARY.mq_with_alpha(0.6),
                );
            }
        }

        // 1.9 Pre-calculate input port states to avoid catastrophic O(N^2 * E) rendering loop
        let mut input_port_states = std::collections::HashMap::with_capacity(self.circuit.connections.len());
        for wire in &self.circuit.connections {
            let state = self.get_wire_state(wire.src_comp_id, wire.src_port);
            input_port_states.insert((wire.tgt_comp_id, wire.tgt_port), state);
        }

        // 2. Draw Components
        // (viewport_rect is already computed above)

        let mut visible_comp_ids: Vec<usize> = self.canvas.spatial_grid.query_rect(viewport_rect).into_iter().collect();
        visible_comp_ids.sort_unstable(); // Restore deterministic drawing Z-order
        


        for &comp_id in &visible_comp_ids {
            let Some(comp) = comp_map.get(&comp_id) else { continue };
            
            let screen_pos = self.to_screen_space(comp.pos);
            let comp_width = comp.width * self.canvas.zoom;
            let comp_height = comp.height * self.canvas.zoom;

            // Determine body color based on component type and activity
            let is_input_active = if comp.comp_type == ComponentType::Input {
                if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&comp.id) {
                    self.engine.simulator.get_state(gate_idx)
                } else {
                    false
                }
            } else if comp.comp_type == ComponentType::Output {
                let mut output_active = false;
                if let Some(&gate_idx) = self.engine.visual_to_sim_map.get(&comp.id) {
                    output_active = self.engine.simulator.get_state(gate_idx);
                }
                output_active
            } else {
                false
            };

            if comp.comp_type == ComponentType::Junction {
                // Render Junctions using their actual width/height so stretch visuals match hit area.
                let center_x = screen_pos.x + comp_width / 2.0;
                let center_y = screen_pos.y + comp_height / 2.0;

                let corner_radius = 6.0 * self.canvas.zoom;
                let thickness = 1.5 * self.canvas.zoom;

                // Bar body
                draw_rounded_rect(
                    screen_pos.x,
                    screen_pos.y,
                    comp_width,
                    comp_height,
                    corner_radius,
                    theme::TEXT_PRIMARY.mq_with_alpha(0.25),
                );
                draw_rounded_rect_lines(
                    screen_pos.x,
                    screen_pos.y,
                    comp_width,
                    comp_height,
                    corner_radius,
                    thickness,
                    theme::BORDER.mq(),
                );

                // Center dot for clarity
                let radius = 4.5 * self.canvas.zoom;
                draw_circle(center_x, center_y, radius, theme::TEXT_PRIMARY.mq());

                if self.canvas.selected_comp_id == Some(comp.id)
                    || self.canvas.selected_comp_ids.contains(&comp.id)
                {
                    let offset = 3.0 * self.canvas.zoom;
                    draw_rounded_rect_lines(
                        screen_pos.x - offset,
                        screen_pos.y - offset,
                        comp_width + offset * 2.0,
                        comp_height + offset * 2.0,
                        corner_radius + offset,
                        thickness,
                        theme::ACCENT_PRIMARY.mq_with_alpha(0.8),
                    );
                }

                continue;
            }

            // Draw component box with rounded corners and drop shadow
            // Drop shadow removed for performance / Blueprint style

            let accent_color = if let Some(color_override) = comp.color.map(|c| Color::new(c[0], c[1], c[2], c[3])).or_else(|| self.circuit.color_overrides.get_component_color(comp.id)) {
                color_override
            } else {
                match comp.comp_type {
                    ComponentType::Nand => theme::COMP_NAND.mq(),
                    ComponentType::Clock => theme::ACCENT_PRIMARY.mq(),
                    ComponentType::Input | ComponentType::Output => {
                        if is_input_active {
                            theme::ACCENT_ACTIVE.mq()
                        } else {
                            theme::ACCENT_GENERIC.mq()
                        }
                    }
                    ComponentType::SubChip(_) => theme::COMP_SUBCHIP.mq(),
                    ComponentType::SevenSegment => theme::COMP_SEVENSEG.mq(),
                    ComponentType::TriStateBuffer => theme::COMP_NAND.mq(),
                    ComponentType::Junction
                    | ComponentType::BusJoiner
                    | ComponentType::BusSplitter => theme::ACCENT_GENERIC.mq(),
                }
            };

            let corner_radius = 6.0 * self.canvas.zoom;

            // Ultra-Low LOD (Zoom < 0.1): Just draw flat solid rectangle
            if self.canvas.zoom < 0.1 {
                draw_rectangle(
                    screen_pos.x,
                    screen_pos.y,
                    comp_width,
                    comp_height,
                    accent_color.clone(),
                );
                continue;
            }

            // Blueprint Style: Opaque background with thick colored borders
            draw_rounded_rect(
                screen_pos.x,
                screen_pos.y,
                comp_width,
                comp_height,
                corner_radius,
                theme::BG_CANVAS.mq(),
            );
            draw_rounded_rect_lines(
                screen_pos.x,
                screen_pos.y,
                comp_width,
                comp_height,
                corner_radius,
                2.0 * self.canvas.zoom,
                accent_color,
            );

            // Draw glowing selection border if selected
            if self.canvas.selected_comp_id == Some(comp.id)
                || self.canvas.selected_comp_ids.contains(&comp.id)
            {
                let offset = 3.0 * self.canvas.zoom;
                draw_rounded_rect_lines(
                    screen_pos.x - offset,
                    screen_pos.y - offset,
                    comp_width + offset * 2.0,
                    comp_height + offset * 2.0,
                    corner_radius + offset,
                    1.5 * self.canvas.zoom,
                    theme::ACCENT_PRIMARY.mq_with_alpha(0.8), // Glowing cyan
                );
            }

            // Draw text label / Semantic state
            if comp.comp_type != ComponentType::SevenSegment && self.canvas.zoom >= 0.35 {
                let base_font_size = 13.0;
                
                let display_label = match comp.comp_type {
                    ComponentType::Input | ComponentType::Output => {
                        format!("{} [{}]", comp.label, if is_input_active { "1" } else { "0" })
                    }
                    _ => comp.label.clone(),
                };

                let text_size = measure_text(
                    &display_label,
                    self.font.as_ref(),
                    base_font_size as u16,
                    self.canvas.zoom,
                );
                let text_x = screen_pos.x + (comp_width - text_size.width) / 2.0;
                let text_y = screen_pos.y + (comp_height + text_size.height) / 2.0;

                draw_text_ex(
                    &display_label,
                    text_x,
                    text_y,
                    TextParams {
                        font: self.font.as_ref(),
                        font_size: base_font_size as u16,
                        font_scale: self.canvas.zoom,
                        color: theme::TEXT_PRIMARY.mq(),
                        ..Default::default()
                    },
                );
            }

            // Low LOD (Zoom < 0.25): Skip drawing individual ports, just draw wire connections
            if self.canvas.zoom < 0.25 {
                continue;
            }

            // Draw port circles
            let (inputs_count, outputs_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
            let port_radius = 4.0 * self.canvas.zoom;

            let mut seg_states = [false; 8];

            // Input ports on left
            #[allow(clippy::needless_range_loop)]
            for i in 0..inputs_count {
                let port_pos = self.to_screen_space(comp.input_port_pos(i, inputs_count));

                let input_active = input_port_states.get(&(comp.id, i)).copied().unwrap_or(false);

                if comp.comp_type == ComponentType::SevenSegment && i < 8 {
                    seg_states[i] = input_active;
                }

                let port_color = if input_active {
                    theme::ACCENT_PRIMARY.mq() // Electric cyan
                } else {
                    theme::ACCENT_INACTIVE.mq() // Muted slate gray
                };

                if input_active {
                    draw_circle(
                        port_pos.x,
                        port_pos.y,
                        port_radius + 2.0 * self.canvas.zoom,
                        theme::ACCENT_PRIMARY.mq_with_alpha(0.2),
                    );
                }
                draw_circle(port_pos.x, port_pos.y, port_radius, port_color);
                draw_circle(
                    port_pos.x,
                    port_pos.y,
                    2.0 * self.canvas.zoom,
                    theme::BG_PANEL.mq(),
                );
            }

            // Output ports on right
            for o in 0..outputs_count {
                let port_pos = self.to_screen_space(comp.output_port_pos(o, outputs_count));

                let output_active =
                    if let Some(&gate_idx) = self.engine.port_to_sim_gate_map.get(&(comp.id, o)) {
                        self.engine.simulator.get_state(gate_idx)
                    } else if comp.comp_type == ComponentType::Input {
                        is_input_active
                    } else {
                        false
                    };

                let port_color = if output_active {
                    theme::ACCENT_PRIMARY.mq() // Electric cyan
                } else {
                    theme::ACCENT_INACTIVE.mq() // Muted slate gray
                };

                if output_active {
                    draw_circle(
                        port_pos.x,
                        port_pos.y,
                        port_radius + 2.0 * self.canvas.zoom,
                        theme::ACCENT_PRIMARY.mq_with_alpha(0.2),
                    );
                }
                draw_circle(port_pos.x, port_pos.y, port_radius, port_color);
                draw_circle(
                    port_pos.x,
                    port_pos.y,
                    2.0 * self.canvas.zoom,
                    theme::BG_PANEL.mq(),
                );
            }

            // Draw custom port names inside sub-chip boundary boxes
            if let ComponentType::SubChip(idx) = comp.comp_type
                && let Some(bp) = self.engine.library.get(idx)
                && self.canvas.zoom >= 0.35
            {
                let target_text_size_px = (10.0 * self.canvas.zoom).max(5.0);
                let base_font_size = 32.0;
                let font_scale = target_text_size_px / base_font_size;

                for i in 0..inputs_count {
                    let port_pos = self.to_screen_space(comp.input_port_pos(i, inputs_count));
                    let name = bp
                        .input_names
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("{}", i));
                    draw_text_ex(
                        &name,
                        port_pos.x + 6.0 * self.canvas.zoom,
                        port_pos.y + 3.0 * self.canvas.zoom,
                        TextParams {
                            font: self.font.as_ref(),
                            font_size: base_font_size as u16,
                            font_scale,
                            color: theme::TEXT_SECONDARY.mq(),
                            ..Default::default()
                        },
                    );
                }
                for o in 0..outputs_count {
                    let port_pos = self.to_screen_space(comp.output_port_pos(o, outputs_count));
                    let name = bp
                        .output_names
                        .get(o)
                        .cloned()
                        .unwrap_or_else(|| format!("{}", o));
                    let text_w =
                        measure_text(&name, self.font.as_ref(), base_font_size as u16, font_scale)
                            .width;
                    draw_text_ex(
                        &name,
                        port_pos.x - 6.0 * self.canvas.zoom - text_w,
                        port_pos.y + 3.0 * self.canvas.zoom,
                        TextParams {
                            font: self.font.as_ref(),
                            font_size: base_font_size as u16,
                            font_scale,
                            color: theme::TEXT_SECONDARY.mq(),
                            ..Default::default()
                        },
                    );
                }
            }

            if comp.comp_type == ComponentType::SevenSegment {
                let cx = screen_pos.x + comp_width / 2.0;
                let cy = screen_pos.y + comp_height / 2.0;
                let w = 15.0 * self.canvas.zoom;
                let h = 15.0 * self.canvas.zoom;
                let thick = 4.0 * self.canvas.zoom;

                // Segment A (top)
                draw_line(
                    cx - w,
                    cy - 2.0 * h,
                    cx + w,
                    cy - 2.0 * h,
                    thick,
                    get_seg_color(seg_states[0]),
                );
                // Segment B (top right)
                draw_line(
                    cx + w,
                    cy - 2.0 * h,
                    cx + w,
                    cy,
                    thick,
                    get_seg_color(seg_states[1]),
                );
                // Segment C (bottom right)
                draw_line(
                    cx + w,
                    cy,
                    cx + w,
                    cy + 2.0 * h,
                    thick,
                    get_seg_color(seg_states[2]),
                );
                // Segment D (bottom)
                draw_line(
                    cx - w,
                    cy + 2.0 * h,
                    cx + w,
                    cy + 2.0 * h,
                    thick,
                    get_seg_color(seg_states[3]),
                );
                // Segment E (bottom left)
                draw_line(
                    cx - w,
                    cy,
                    cx - w,
                    cy + 2.0 * h,
                    thick,
                    get_seg_color(seg_states[4]),
                );
                // Segment F (top left)
                draw_line(
                    cx - w,
                    cy - 2.0 * h,
                    cx - w,
                    cy,
                    thick,
                    get_seg_color(seg_states[5]),
                );
                // Segment G (middle)
                draw_line(cx - w, cy, cx + w, cy, thick, get_seg_color(seg_states[6]));

                // Segment "minus" (port 7)
                // Port ordering: A, B, C, D, E, F, G, minus
                // Positioned to the left of the digit (as a negative sign indicator)
                draw_line(
                    cx - w - 20.0 * self.canvas.zoom,
                    cy,
                    cx - w - 10.0 * self.canvas.zoom,
                    cy,
                    thick,
                    get_seg_color(seg_states[7]),
                );
            }
        }

        // Draw selection box (Windows style)
        if let Some(start) = self.canvas.selection_box_start {
            let start_screen = self.to_screen_space(start);
            let end_screen = Vec2::from(mouse_position());

            let x_min = start_screen.x.min(end_screen.x);
            let x_max = start_screen.x.max(end_screen.x);
            let y_min = start_screen.y.min(end_screen.y);
            let y_max = start_screen.y.max(end_screen.y);

            let w = x_max - x_min;
            let h = y_max - y_min;

            // Draw filled selection box with translucent blue
            draw_rectangle(
                x_min,
                y_min,
                w,
                h,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.15),
            );
            // Draw selection box border line
            draw_rectangle_lines(
                x_min,
                y_min,
                w,
                h,
                1.5,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.6),
            );
        }

        // Draw alignment guides
        for &(start, end) in &self.canvas.alignment_guides {
            let p1 = self.to_screen_space(start);
            let p2 = self.to_screen_space(end);
            draw_line(
                p1.x,
                p1.y,
                p2.x,
                p2.y,
                1.5,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.8),
            );
        }
        // Draw magnetic hover ring
        if let Some((comp_id, port_idx, is_input)) = self.canvas.hovered_port
            && let Some(comp) = self.get_component(comp_id)
        {
            let (inputs_count, outputs_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
            let pos = if is_input {
                self.to_screen_space(comp.input_port_pos(port_idx, inputs_count))
            } else {
                self.to_screen_space(comp.output_port_pos(port_idx, outputs_count))
            };

            // Draw a pulsing glowing ring
            let t = get_time() as f32;
            let pulse = (t * 8.0).sin() * 0.5 + 0.5; // 0.0 to 1.0
            let radius = 6.0 * self.canvas.zoom + pulse * 4.0 * self.canvas.zoom;

            draw_circle_lines(
                pos.x,
                pos.y,
                radius,
                2.0 * self.canvas.zoom,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.8 - pulse * 0.4),
            );
            draw_circle(
                pos.x,
                pos.y,
                radius,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.2 - pulse * 0.1),
            );
        }
        // Draw keyboard tab focus indicator
        if let Some((comp_id, port_opt)) = self.canvas.tab_focus {
            if let Some(comp) = self.get_component(comp_id) {
                if let Some((port_idx, is_input)) = port_opt {
                    // Draw a square bracket around the port
                    let (in_count, out_count) = self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));
                    let pos = if is_input {
                        self.to_screen_space(comp.input_port_pos(port_idx, in_count))
                    } else {
                        self.to_screen_space(comp.output_port_pos(port_idx, out_count))
                    };
                    draw_rectangle_lines(pos.x - 6.0, pos.y - 6.0, 12.0, 12.0, 2.0, theme::ACCENT_ACTIVE.mq());
                } else {
                    // Draw a dashed highlight around the component
                    let screen_pos = self.to_screen_space(comp.pos);
                    let w = comp.width * self.canvas.zoom;
                    let h = comp.height * self.canvas.zoom;
                    draw_rectangle_lines(screen_pos.x - 4.0, screen_pos.y - 4.0, w + 8.0, h + 8.0, 2.0, theme::ACCENT_ACTIVE.mq());
                }
            }
        }

        // Draw instructions at top-left
        draw_text_ex(
            "Left Click: Place/Connect/Toggle | Drag: Move | Right Click/Del: Delete | Scroll: Zoom | Right Drag: Pan",
            15.0,
            20.0,
            TextParams {
                font: self.font.as_ref(),
                font_size: 14,
                color: theme::TEXT_SECONDARY.mq_with_alpha(0.8),
                ..Default::default()
            },
        );
    }
}
