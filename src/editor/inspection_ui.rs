use crate::editor::theme;
use crate::engine::{Component, ComponentType, SourcePort, TargetPort, TraceNode};
use macroquad::prelude::*;

use super::Editor;

impl Editor {
    pub(crate) fn get_bp_comp_input_port_pos(
        &self,
        comp_idx: usize,
        port_idx: usize,
        comps: &[Component],
    ) -> Vec2 {
        let comp = &comps[comp_idx];
        let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
        let max_ports = inputs_count.max(outputs_count);
        let height = 40.0 + (max_ports as f32 * 16.0);

        let x = comp.pos.0;
        let spacing = height / (inputs_count + 1) as f32;
        let y = comp.pos.1 + spacing * (port_idx + 1) as f32;
        Vec2::new(x, y)
    }

    pub(crate) fn get_bp_comp_output_port_pos(
        &self,
        comp_idx: usize,
        port_idx: usize,
        comps: &[Component],
    ) -> Vec2 {
        let comp = &comps[comp_idx];
        let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
        let max_ports = inputs_count.max(outputs_count);
        let height = 40.0 + (max_ports as f32 * 16.0);
        let (width, _) = self.get_component_dimensions(comp.component_type);

        let x = comp.pos.0 + width;
        let spacing = height / (outputs_count + 1) as f32;
        let y = comp.pos.1 + spacing * (port_idx + 1) as f32;
        Vec2::new(x, y)
    }

    pub(crate) fn draw_inspection_view(&self) {
        let (blueprint, internal_components) = match self.get_inspected_blueprint_and_components() {
            Some(res) => res,
            None => {
                draw_text(
                    "Failed to load inspection blueprint!",
                    screen_width() / 2.0 - 150.0,
                    screen_height() / 2.0,
                    20.0,
                    RED,
                );
                return;
            }
        };

        let inputs_count = blueprint.inputs;
        let outputs_count = blueprint.outputs;

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for comp in &internal_components {
            let (w, h) = self.get_component_dimensions(comp.component_type);
            min_x = min_x.min(comp.pos.0);
            max_x = max_x.max(comp.pos.0 + w);
            min_y = min_y.min(comp.pos.1);
            max_y = max_y.max(comp.pos.1 + h);
        }

        if min_x > max_x {
            min_x = 200.0;
            max_x = 600.0;
            min_y = 100.0;
            max_y = 100.0;
        }

        let input_x = min_x - 150.0;
        let output_x = max_x + 150.0;
        let spacing_y = 60.0;

        let center_y = (min_y + max_y) / 2.0;
        let inputs_height = (inputs_count.max(1) - 1) as f32 * spacing_y;
        let outputs_height = (outputs_count.max(1) - 1) as f32 * spacing_y;

        let input_y_start = center_y - (inputs_height / 2.0);
        let output_y_start = center_y - (outputs_height / 2.0);

        let get_chip_input_pos =
            |idx: usize| -> Vec2 { Vec2::new(input_x, input_y_start + idx as f32 * spacing_y) };
        let get_chip_output_pos =
            |idx: usize| -> Vec2 { Vec2::new(output_x, output_y_start + idx as f32 * spacing_y) };

        // Draw outer chip boundary labels & circles
        for i in 0..inputs_count {
            let world_pos = get_chip_input_pos(i);
            let screen_pos = self.to_screen_space(world_pos);
            let state = self
                .get_raw_node_state_at_path(&TraceNode::ChipInput(i), &self.canvas.inspection_path);
            let port_color = match state {
                0b00 => theme::ACCENT_GENERIC.mq(),
                0b01 => theme::ACCENT_INACTIVE.mq(),
                0b10 => theme::ACCENT_PRIMARY.mq(),
                _ => theme::COMP_NAND.mq(),
            };

            draw_circle(
                screen_pos.x,
                screen_pos.y,
                6.0 * self.canvas.zoom,
                port_color,
            );
            draw_circle(
                screen_pos.x,
                screen_pos.y,
                3.0 * self.canvas.zoom,
                theme::BG_CANVAS.mq(),
            );
            let label_text = blueprint
                .input_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("IN {}", i));
            draw_text(
                &label_text,
                screen_pos.x - 45.0 * self.canvas.zoom,
                screen_pos.y + 4.0 * self.canvas.zoom,
                (12.0 * self.canvas.zoom).max(6.0),
                theme::TEXT_SECONDARY.mq(),
            );
        }

        for j in 0..outputs_count {
            let world_pos = get_chip_output_pos(j);
            let screen_pos = self.to_screen_space(world_pos);
            let state = self.get_raw_node_state_at_path(
                &TraceNode::ChipOutput(j),
                &self.canvas.inspection_path,
            );
            let port_color = match state {
                0b00 => theme::ACCENT_GENERIC.mq(),
                0b01 => theme::ACCENT_INACTIVE.mq(),
                0b10 => theme::ACCENT_PRIMARY.mq(),
                _ => theme::COMP_NAND.mq(),
            };

            draw_circle(
                screen_pos.x,
                screen_pos.y,
                6.0 * self.canvas.zoom,
                port_color,
            );
            draw_circle(
                screen_pos.x,
                screen_pos.y,
                3.0 * self.canvas.zoom,
                theme::BG_CANVAS.mq(),
            );
            let label_text = blueprint
                .output_names
                .get(j)
                .cloned()
                .unwrap_or_else(|| format!("OUT {}", j));
            draw_text(
                &label_text,
                screen_pos.x + 15.0 * self.canvas.zoom,
                screen_pos.y + 4.0 * self.canvas.zoom,
                (12.0 * self.canvas.zoom).max(6.0),
                theme::TEXT_SECONDARY.mq(),
            );
        }

        // Draw connections inside the blueprint
        for conn in &blueprint.connections {
            let src_pos = match conn.source {
                SourcePort::ChipInput(idx) => self.to_screen_space(get_chip_input_pos(idx)),
                SourcePort::ComponentOutput {
                    component_idx,
                    port_idx,
                } => self.to_screen_space(self.get_bp_comp_output_port_pos(
                    component_idx,
                    port_idx,
                    &internal_components,
                )),
            };

            let tgt_pos = match conn.target {
                TargetPort::ChipOutput(idx) => self.to_screen_space(get_chip_output_pos(idx)),
                TargetPort::ComponentInput {
                    component_idx,
                    port_idx,
                } => self.to_screen_space(self.get_bp_comp_input_port_pos(
                    component_idx,
                    port_idx,
                    &internal_components,
                )),
            };

            let src_node = match conn.source {
                SourcePort::ChipInput(i) => TraceNode::ChipInput(i),
                SourcePort::ComponentOutput {
                    component_idx,
                    port_idx,
                } => TraceNode::CompOutput {
                    component_idx,
                    port_idx,
                },
            };

            let state = self.get_raw_node_state_at_path(&src_node, &self.canvas.inspection_path);
            self.draw_manhattan_wire(src_pos, tgt_pos, state, false);
        }

        // Draw internal components
        for (comp_idx, comp) in internal_components.iter().enumerate() {
            let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
            let (width, height) = self.get_component_dimensions(comp.component_type);

            let comp_pos = Vec2::new(comp.pos.0, comp.pos.1);
            let screen_pos = self.to_screen_space(comp_pos);
            let screen_width = width * self.canvas.zoom;
            let screen_height = height * self.canvas.zoom;

            let bg_color = theme::BG_PANEL.mq_with_alpha(0.95);
            let border_color = theme::BORDER.mq();

            draw_rectangle(
                screen_pos.x,
                screen_pos.y,
                screen_width,
                screen_height,
                bg_color,
            );
            draw_rectangle_lines(
                screen_pos.x,
                screen_pos.y,
                screen_width,
                screen_height,
                1.5 * self.canvas.zoom,
                border_color,
            );

            // Draw Top Accent Stripe
            let accent_color = match comp.component_type {
                ComponentType::Nand => theme::COMP_NAND.mq(),
                ComponentType::Clock => theme::ACCENT_PRIMARY.mq(),
                ComponentType::Input | ComponentType::Output => {
                    // Check output port 0 to see if it is active in the flat simulation state path
                    let state = self.get_raw_node_state_at_path(
                        &TraceNode::CompOutput {
                            component_idx: comp_idx,
                            port_idx: 0,
                        },
                        &self.canvas.inspection_path,
                    );
                    if (state & 0b10) != 0 {
                        theme::ACCENT_ACTIVE.mq()
                    } else {
                        theme::ACCENT_GENERIC.mq()
                    }
                }
                ComponentType::SubChip(_) => theme::COMP_SUBCHIP.mq(),
                ComponentType::SevenSegment => theme::COMP_SEVENSEG.mq(),
                ComponentType::TriStateBuffer => theme::COMP_NAND.mq(),
                ComponentType::Junction => theme::ACCENT_GENERIC.mq(),
            };
            let stripe_height = 4.0 * self.canvas.zoom;
            draw_rectangle(
                screen_pos.x,
                screen_pos.y,
                screen_width,
                stripe_height,
                accent_color,
            );

            // Draw label
            let label = self.get_component_label(comp.component_type);
            let font_size = (13.0 * self.canvas.zoom).max(6.0);
            let text_size = measure_text(&label, self.font.as_ref(), font_size as u16, 1.0);
            let text_x = screen_pos.x + (screen_width - text_size.width) / 2.0;
            let text_y = screen_pos.y + (screen_height + text_size.height) / 2.0;
            draw_text_ex(
                &label,
                text_x,
                text_y,
                TextParams {
                    font: self.font.as_ref(),
                    font_size: font_size as u16,
                    color: theme::TEXT_PRIMARY.mq(),
                    ..Default::default()
                },
            );

            // Draw input circles
            for i in 0..inputs_count {
                let w_pos = self.get_bp_comp_input_port_pos(comp_idx, i, &internal_components);
                let port_pos = self.to_screen_space(w_pos);
                let state = self.get_raw_node_state_at_path(
                    &TraceNode::CompInput {
                        component_idx: comp_idx,
                        port_idx: i,
                    },
                    &self.canvas.inspection_path,
                );
                let port_color = match state {
                    0b00 => theme::ACCENT_GENERIC.mq(),
                    0b01 => theme::ACCENT_INACTIVE.mq(),
                    0b10 => theme::ACCENT_PRIMARY.mq(),
                    _ => theme::COMP_NAND.mq(),
                };
                draw_circle(port_pos.x, port_pos.y, 4.0 * self.canvas.zoom, port_color);
                draw_circle(
                    port_pos.x,
                    port_pos.y,
                    2.0 * self.canvas.zoom,
                    theme::BG_PANEL.mq(),
                );
            }

            // Draw output circles
            for o in 0..outputs_count {
                let w_pos = self.get_bp_comp_output_port_pos(comp_idx, o, &internal_components);
                let port_pos = self.to_screen_space(w_pos);

                let state = self.get_raw_node_state_at_path(
                    &TraceNode::CompOutput {
                        component_idx: comp_idx,
                        port_idx: o,
                    },
                    &self.canvas.inspection_path,
                );
                let port_color = match state {
                    0b00 => theme::ACCENT_GENERIC.mq(),
                    0b01 => theme::ACCENT_INACTIVE.mq(),
                    0b10 => theme::ACCENT_PRIMARY.mq(),
                    _ => theme::COMP_NAND.mq(),
                };

                draw_circle(port_pos.x, port_pos.y, 4.0 * self.canvas.zoom, port_color);
                draw_circle(
                    port_pos.x,
                    port_pos.y,
                    2.0 * self.canvas.zoom,
                    theme::BG_PANEL.mq(),
                );
            }
        }

        // Draw info overlay
        let title = format!("LOOK INSIDE: {}", blueprint.name);
        draw_text(
            &title,
            15.0,
            20.0,
            16.0,
            theme::ACCENT_PRIMARY.mq_with_alpha(0.95),
        );
        draw_text(
            "Inspection Mode (Read-Only) | Drag mouse wheel/right-click to pan | Scroll to zoom",
            15.0,
            40.0,
            12.0,
            theme::TEXT_SECONDARY.mq_with_alpha(0.8),
        );
    }
}
