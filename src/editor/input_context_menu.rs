use macroquad::prelude::*;
use super::types::VisualComponent;
use super::Editor;

impl Editor {
    pub fn handle_right_click_context_menu(&mut self, mouse_pos_world: Vec2, egui_wants_pointer: bool) {
        if egui_wants_pointer || !self.canvas.inspection_path.is_empty() {
            return;
        }

        if is_mouse_button_released(MouseButton::Right) && self.canvas.right_drag_dist < 5.0 {
            // Short right-click — check if over a component
            for comp in &self.circuit.components {
                if mouse_pos_world.x >= comp.pos.x
                    && mouse_pos_world.x <= comp.pos.x + comp.width
                    && mouse_pos_world.y >= comp.pos.y
                    && mouse_pos_world.y <= comp.pos.y + comp.height
                {
                    let screen_pos = self.to_screen_space(mouse_pos_world);
                    self.ui.context_menu_pos = (screen_pos.x, screen_pos.y);
                    // Initialize picker with current override or default
                    self.ui.context_menu_color = self
                        .circuit.color_overrides
                        .get_component_color(comp.id)
                        .map(|c| [c.r, c.g, c.b, c.a])
                        .unwrap_or([0.4, 0.45, 0.85, 1.0]);
                    self.ui.show_context_menu = Some(
                        crate::editor::color_coding::ContextMenuTarget::Component(comp.id),
                    );
                    return;
                }
            }

            // Check if over a wire
            let comp_by_id: std::collections::HashMap<usize, &VisualComponent> =
                self.circuit.components.iter().map(|c| (c.id, c)).collect();
            for conn in &self.circuit.connections {
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
                    if self.hit_test_manhattan_wire(
                        src_pos,
                        tgt_pos,
                        offset,
                        conn.tgt_port,
                        mouse_pos_world,
                        10.0 / self.canvas.zoom,
                    ) {
                        let screen_pos = self.to_screen_space(mouse_pos_world);
                        self.ui.context_menu_pos = (screen_pos.x, screen_pos.y);
                        self.ui.context_menu_color = self
                            .circuit.color_overrides
                            .get_wire_color(conn)
                            .map(|c| [c.r, c.g, c.b, c.a])
                            .unwrap_or([0.4, 0.45, 0.85, 1.0]);
                        self.ui.show_context_menu = Some(
                            crate::editor::color_coding::ContextMenuTarget::Wire(*conn),
                        );
                        return;
                    }
                }
            }
        }
    }
}
