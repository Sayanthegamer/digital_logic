use crate::engine::ComponentType;
use macroquad::prelude::*;
use super::Editor;

impl Editor {
    pub fn update_hovered_port(
        &mut self,
        mouse_pos_screen: Vec2,
        mouse_pos_world: Vec2,
        egui_wants_pointer: bool,
    ) {
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
                            // When dragging a wire, prefer target ports of the opposite type.
                            let want_input = if let Some((_, _, start_is_input)) = self.canvas.active_wire_drag {
                                !start_is_input
                            } else {
                                false
                            };
                            self.canvas.hovered_port = Some((comp.id, 0, want_input));
                        }
                        continue;
                    }

                    let (inputs_count, outputs_count) =
                        self.get_component_ports_count_with_width(comp.comp_type, Some(comp.bus_width()));

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
}
