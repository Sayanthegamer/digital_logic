use crate::engine::{
    ChipBlueprint, Component, ComponentType, OutputSource, SourcePort, TargetPort, TraceNode,
};
use macroquad::prelude::*;

use super::Editor;

impl Editor {
    pub fn get_inspected_blueprint_and_components(
        &self,
    ) -> Option<(&ChipBlueprint, Vec<Component>)> {
        if self.inspection_path.is_empty() {
            return None;
        }

        let bp_idx = self.get_blueprint_idx_for_path(&self.inspection_path)?;
        let blueprint = &self.library[bp_idx];
        Some((blueprint, blueprint.components.clone()))
    }

    pub fn get_blueprint_idx_for_path(&self, path: &[usize]) -> Option<usize> {
        if path.is_empty() {
            return None;
        }
        let first_comp_id = path[0];
        let curr_comp = self.components.iter().find(|c| c.id == first_comp_id)?;
        let mut curr_bp_idx = match curr_comp.comp_type {
            ComponentType::SubChip(idx) => idx,
            _ => return None,
        };

        for &comp_idx in path.iter().skip(1) {
            let blueprint = &self.library[curr_bp_idx];
            if comp_idx < blueprint.components.len() {
                let next_comp = &blueprint.components[comp_idx];
                curr_bp_idx = match next_comp.component_type {
                    ComponentType::SubChip(idx) => idx,
                    _ => return None,
                };
            } else {
                return None;
            }
        }
        Some(curr_bp_idx)
    }

    pub fn get_node_state_at_path(&self, node: &TraceNode, path: &[usize]) -> bool {
        if path.is_empty() {
            match node {
                TraceNode::ChipInput(idx) => {
                    let inputs: Vec<&super::types::VisualComponent> = self
                        .components
                        .iter()
                        .filter(|c| c.comp_type == ComponentType::Input)
                        .collect();
                    if let Some(comp) = inputs.get(*idx)
                        && let Some(&g_idx) = self.visual_to_sim_map.get(&comp.id)
                    {
                        return self.simulator.get_state(g_idx);
                    }
                }
                TraceNode::CompOutput {
                    component_idx,
                    port_idx,
                } => {
                    if let Some(&g_idx) =
                        self.port_to_sim_gate_map.get(&(*component_idx, *port_idx))
                    {
                        return self.simulator.get_state(g_idx);
                    }
                }
                TraceNode::CompInput {
                    component_idx,
                    port_idx,
                } => {
                    if let Some(conn) = self
                        .connections
                        .iter()
                        .find(|c| c.tgt_comp_id == *component_idx && c.tgt_port == *port_idx)
                    {
                        let src_node = TraceNode::CompOutput {
                            component_idx: conn.src_comp_id,
                            port_idx: conn.src_port,
                        };
                        return self.get_node_state_at_path(&src_node, &[]);
                    }
                }
                _ => {}
            }
            return false;
        }

        let parent_path = &path[..path.len() - 1];
        let comp_id_in_parent = path[path.len() - 1];

        if let Some(bp_idx) = self.get_blueprint_idx_for_path(path) {
            let blueprint = &self.library[bp_idx];
            let driver = self.trace_local_driver(node, blueprint, path);

            match driver {
                OutputSource::DrivenByGate(g_idx) => self.simulator.get_state(g_idx),
                OutputSource::PassedThrough(in_idx) => {
                    let parent_node = TraceNode::CompInput {
                        component_idx: comp_id_in_parent,
                        port_idx: in_idx,
                    };
                    self.get_node_state_at_path(&parent_node, parent_path)
                }
                OutputSource::Floating => false,
            }
        } else {
            false
        }
    }

    fn trace_local_driver(
        &self,
        node: &TraceNode,
        blueprint: &ChipBlueprint,
        path: &[usize],
    ) -> OutputSource {
        match node {
            TraceNode::CompOutput {
                component_idx,
                port_idx,
            } => {
                let component = &blueprint.components[*component_idx];
                match component.component_type {
                    ComponentType::Nand | ComponentType::Clock => {
                        if let Some(&g_idx) = self
                            .instance_to_sim_map
                            .get(&(path.to_vec(), *component_idx))
                        {
                            OutputSource::DrivenByGate(g_idx)
                        } else {
                            OutputSource::Floating
                        }
                    }
                    ComponentType::SubChip(_) => {
                        if let Some(outputs) =
                            self.instance_outputs.get(&(path.to_vec(), *component_idx))
                        {
                            if *port_idx < outputs.len() {
                                outputs[*port_idx]
                            } else {
                                OutputSource::Floating
                            }
                        } else {
                            OutputSource::Floating
                        }
                    }
                    ComponentType::Input | ComponentType::Output => OutputSource::Floating,
                }
            }
            TraceNode::CompInput {
                component_idx,
                port_idx,
            } => {
                let conn = blueprint.connections.iter().find(|c| {
                    c.target
                        == TargetPort::ComponentInput {
                            component_idx: *component_idx,
                            port_idx: *port_idx,
                        }
                });

                if let Some(c) = conn {
                    match c.source {
                        SourcePort::ChipInput(i) => OutputSource::PassedThrough(i),
                        SourcePort::ComponentOutput {
                            component_idx: src_c,
                            port_idx: src_p,
                        } => self.trace_local_driver(
                            &TraceNode::CompOutput {
                                component_idx: src_c,
                                port_idx: src_p,
                            },
                            blueprint,
                            path,
                        ),
                    }
                } else {
                    OutputSource::Floating
                }
            }
            TraceNode::ChipOutput(out_idx) => {
                let conn = blueprint
                    .connections
                    .iter()
                    .find(|c| c.target == TargetPort::ChipOutput(*out_idx));

                if let Some(c) = conn {
                    match c.source {
                        SourcePort::ChipInput(i) => OutputSource::PassedThrough(i),
                        SourcePort::ComponentOutput {
                            component_idx: src_c,
                            port_idx: src_p,
                        } => self.trace_local_driver(
                            &TraceNode::CompOutput {
                                component_idx: src_c,
                                port_idx: src_p,
                            },
                            blueprint,
                            path,
                        ),
                    }
                } else {
                    OutputSource::Floating
                }
            }
            TraceNode::ChipInput(idx) => OutputSource::PassedThrough(*idx),
        }
    }

    fn get_bp_comp_input_port_pos(
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

    fn get_bp_comp_output_port_pos(
        &self,
        comp_idx: usize,
        port_idx: usize,
        comps: &[Component],
    ) -> Vec2 {
        let comp = &comps[comp_idx];
        let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
        let max_ports = inputs_count.max(outputs_count);
        let height = 40.0 + (max_ports as f32 * 16.0);
        let width = if let ComponentType::SubChip(_) = comp.component_type {
            100.0
        } else {
            70.0
        };

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

        let border_y_start = 100.0;
        let spacing_y = 60.0;

        let get_chip_input_pos =
            |idx: usize| -> Vec2 { Vec2::new(50.0, border_y_start + idx as f32 * spacing_y) };
        let get_chip_output_pos =
            |idx: usize| -> Vec2 { Vec2::new(750.0, border_y_start + idx as f32 * spacing_y) };

        // Draw outer chip boundary labels & circles
        for i in 0..inputs_count {
            let world_pos = get_chip_input_pos(i);
            let screen_pos = self.to_screen_space(world_pos);
            let state =
                self.get_node_state_at_path(&TraceNode::ChipInput(i), &self.inspection_path);
            let port_color = if state {
                Color::new(0.00, 0.70, 1.00, 1.0)
            } else {
                Color::new(0.24, 0.27, 0.30, 1.0)
            };

            draw_circle(screen_pos.x, screen_pos.y, 6.0 * self.zoom, port_color);
            draw_circle(
                screen_pos.x,
                screen_pos.y,
                3.0 * self.zoom,
                Color::new(0.09, 0.10, 0.12, 1.0),
            );
            let label_text = blueprint
                .input_names
                .get(i)
                .cloned()
                .unwrap_or_else(|| format!("IN {}", i));
            draw_text(
                &label_text,
                screen_pos.x - 45.0 * self.zoom,
                screen_pos.y + 4.0 * self.zoom,
                (12.0 * self.zoom).max(6.0),
                Color::new(0.6, 0.65, 0.7, 1.0),
            );
        }

        for j in 0..outputs_count {
            let world_pos = get_chip_output_pos(j);
            let screen_pos = self.to_screen_space(world_pos);
            let state =
                self.get_node_state_at_path(&TraceNode::ChipOutput(j), &self.inspection_path);
            let port_color = if state {
                Color::new(0.00, 0.70, 1.00, 1.0)
            } else {
                Color::new(0.24, 0.27, 0.30, 1.0)
            };

            draw_circle(screen_pos.x, screen_pos.y, 6.0 * self.zoom, port_color);
            draw_circle(
                screen_pos.x,
                screen_pos.y,
                3.0 * self.zoom,
                Color::new(0.09, 0.10, 0.12, 1.0),
            );
            let label_text = blueprint
                .output_names
                .get(j)
                .cloned()
                .unwrap_or_else(|| format!("OUT {}", j));
            draw_text(
                &label_text,
                screen_pos.x + 15.0 * self.zoom,
                screen_pos.y + 4.0 * self.zoom,
                (12.0 * self.zoom).max(6.0),
                Color::new(0.6, 0.65, 0.7, 1.0),
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

            let state = self.get_node_state_at_path(&src_node, &self.inspection_path);
            self.draw_manhattan_wire(src_pos, tgt_pos, state);
        }

        // Draw internal components
        for (comp_idx, comp) in internal_components.iter().enumerate() {
            let (inputs_count, outputs_count) = self.get_component_ports_count(comp.component_type);
            let max_ports = inputs_count.max(outputs_count);
            let height = 40.0 + (max_ports as f32 * 16.0);
            let width = if let ComponentType::SubChip(_) = comp.component_type {
                100.0
            } else {
                70.0
            };

            let comp_pos = Vec2::new(comp.pos.0, comp.pos.1);
            let screen_pos = self.to_screen_space(comp_pos);
            let screen_width = width * self.zoom;
            let screen_height = height * self.zoom;

            let bg_color = Color::new(0.12, 0.13, 0.15, 0.95);
            let border_color = Color::new(0.20, 0.23, 0.26, 1.0);

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
                1.5 * self.zoom,
                border_color,
            );

            // Draw Top Accent Stripe
            let accent_color = match comp.component_type {
                ComponentType::Nand => Color::new(1.0, 0.55, 0.15, 1.0),
                ComponentType::Clock => Color::new(0.00, 0.70, 1.00, 1.0),
                ComponentType::Input | ComponentType::Output => {
                    // Check output port 0 to see if it is active in the flat simulation state path
                    let state = self.get_node_state_at_path(
                        &TraceNode::CompOutput {
                            component_idx: comp_idx,
                            port_idx: 0,
                        },
                        &self.inspection_path,
                    );
                    if state {
                        Color::new(0.15, 0.85, 0.40, 1.0)
                    } else {
                        Color::new(0.35, 0.38, 0.40, 1.0)
                    }
                }
                ComponentType::SubChip(_) => Color::new(0.40, 0.45, 0.85, 1.0),
            };
            let stripe_height = 4.0 * self.zoom;
            draw_rectangle(
                screen_pos.x,
                screen_pos.y,
                screen_width,
                stripe_height,
                accent_color,
            );

            // Draw label
            let label = self.get_component_label(comp.component_type);
            let font_size = (13.0 * self.zoom).max(6.0);
            let text_size = measure_text(&label, None, font_size as u16, 1.0);
            let text_x = screen_pos.x + (screen_width - text_size.width) / 2.0;
            let text_y = screen_pos.y + (screen_height + text_size.height) / 2.0;
            draw_text(
                &label,
                text_x,
                text_y,
                font_size,
                Color::new(0.85, 0.88, 0.90, 1.0),
            );

            // Draw input circles
            for i in 0..inputs_count {
                let w_pos = self.get_bp_comp_input_port_pos(comp_idx, i, &internal_components);
                let port_pos = self.to_screen_space(w_pos);
                let state = self.get_node_state_at_path(
                    &TraceNode::CompInput {
                        component_idx: comp_idx,
                        port_idx: i,
                    },
                    &self.inspection_path,
                );
                let port_color = if state {
                    Color::new(0.00, 0.70, 1.00, 1.0)
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0)
                };
                draw_circle(port_pos.x, port_pos.y, 4.0 * self.zoom, port_color);
                draw_circle(
                    port_pos.x,
                    port_pos.y,
                    2.0 * self.zoom,
                    Color::new(0.12, 0.13, 0.15, 1.0),
                );
            }

            // Draw output circles
            for o in 0..outputs_count {
                let w_pos = self.get_bp_comp_output_port_pos(comp_idx, o, &internal_components);
                let port_pos = self.to_screen_space(w_pos);

                let state = self.get_node_state_at_path(
                    &TraceNode::CompOutput {
                        component_idx: comp_idx,
                        port_idx: o,
                    },
                    &self.inspection_path,
                );
                let port_color = if state {
                    Color::new(0.00, 0.70, 1.00, 1.0)
                } else {
                    Color::new(0.24, 0.27, 0.30, 1.0)
                };

                draw_circle(port_pos.x, port_pos.y, 4.0 * self.zoom, port_color);
                draw_circle(
                    port_pos.x,
                    port_pos.y,
                    2.0 * self.zoom,
                    Color::new(0.12, 0.13, 0.15, 1.0),
                );
            }
        }

        // Draw info overlay
        let title = format!("LOOK INSIDE: {}", blueprint.name);
        draw_text(&title, 15.0, 20.0, 16.0, Color::new(0.3, 0.75, 1.0, 0.95));
        draw_text(
            "Inspection Mode (Read-Only) | Drag mouse wheel/right-click to pan | Scroll to zoom",
            15.0,
            40.0,
            12.0,
            Color::new(0.5, 0.55, 0.6, 0.8),
        );
    }
}
