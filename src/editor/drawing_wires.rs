use crate::editor::theme;
use macroquad::prelude::*;

use super::Editor;

impl Editor {
    pub(crate) fn draw_manhattan_wire(
        &self,
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        wire_state: u8,
        is_selected: bool,
        color_override: Option<Color>,
        is_bus: bool,
    ) {
        let (base_color, mut thickness, is_active) = match wire_state {
            0b00 => (theme::ACCENT_GENERIC.mq(), 1.3 * self.canvas.zoom, false),
            0b01 => (theme::ACCENT_INACTIVE.mq(), 1.6 * self.canvas.zoom, false),
            0b10 => (theme::ACCENT_PRIMARY.mq(), 2.2 * self.canvas.zoom, true),
            _ => (theme::COMP_NAND.mq(), 2.8 * self.canvas.zoom, true),
        };
        if is_bus {
            thickness *= 2.2;
        }

        // Use color override if provided, otherwise use state-based color
        let color = color_override.unwrap_or(base_color);

        let segments = Self::compute_wire_segments_screen(
            src_pos,
            tgt_pos,
            routing_offset,
            self.canvas.zoom,
        );

        // Active glow bloom effect under active wires
        if is_active || is_selected {
            let glow_color = if is_selected {
                theme::ACCENT_PRIMARY.mq_with_alpha(0.4) // Strong cyan glow for selection
            } else {
                color // Glow matches the wire color
            };
            let glow_color = Color::new(glow_color.r, glow_color.g, glow_color.b, 0.2);
            let glow_thickness =
                thickness + (if is_selected { 6.0 } else { 4.0 }) * self.canvas.zoom;
            for &(a, b) in &segments {
                draw_line(a.x, a.y, b.x, b.y, glow_thickness, glow_color);
            }
        }

        for &(a, b) in &segments {
            draw_line(a.x, a.y, b.x, b.y, thickness, color);
        }

        // Draw concentric terminal circle/indicator at target
        let port_radius = 4.0 * self.canvas.zoom;
        if is_active {
            draw_circle(
                tgt_pos.x,
                tgt_pos.y,
                port_radius + 2.0 * self.canvas.zoom,
                theme::ACCENT_PRIMARY.mq_with_alpha(0.2),
            );
        }
        draw_circle(tgt_pos.x, tgt_pos.y, port_radius, color);
        draw_circle(
            tgt_pos.x,
            tgt_pos.y,
            2.0 * self.canvas.zoom,
            theme::BG_CANVAS.mq(),
        );
    }

    pub(crate) fn hit_test_manhattan_wire(
        &self,
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        point: Vec2,
        threshold: f32,
    ) -> bool {
        let segments = Self::compute_wire_segments_world(src_pos, tgt_pos, routing_offset);

        for (a, b) in segments {
            let line_vec = b - a;
            let p_vec = point - a;

            let line_len_sq = line_vec.length_squared();
            let t = if line_len_sq == 0.0 {
                0.0
            } else {
                (p_vec.dot(line_vec) / line_len_sq).clamp(0.0, 1.0)
            };

            let projection = a + line_vec * t;
            if point.distance(projection) <= threshold {
                return true;
            }
        }
        false
    }
}
