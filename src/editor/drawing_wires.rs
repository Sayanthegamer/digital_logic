use macroquad::prelude::*;
use crate::editor::theme;

use super::Editor;

impl Editor {
    pub(crate) fn draw_manhattan_wire_segments(
        src_pos: Vec2,
        tgt_pos: Vec2,
        thickness: f32,
        color: Color,
        zoom: f32,
    ) {
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

    pub(crate) fn draw_manhattan_wire(&self, src_pos: Vec2, tgt_pos: Vec2, wire_state: u8, is_selected: bool) {
        let (color, thickness, is_active) = match wire_state {
            0b00 => (theme::ACCENT_GENERIC.mq(), 1.3 * self.canvas.zoom, false),
            0b01 => (theme::ACCENT_INACTIVE.mq(), 1.6 * self.canvas.zoom, false),
            0b10 => (theme::ACCENT_PRIMARY.mq(), 2.2 * self.canvas.zoom, true),
            0b11 | _ => (theme::COMP_NAND.mq(), 2.8 * self.canvas.zoom, true),
        };

        // Active glow bloom effect under active wires
        if is_active || is_selected {
            let glow_color = if is_selected {
                theme::ACCENT_PRIMARY.mq_with_alpha(0.4) // Strong cyan glow for selection
            } else {
                color.clone() // Glow matches the wire color
            };
            let glow_color = Color::new(glow_color.r, glow_color.g, glow_color.b, 0.2);
            let glow_thickness = thickness + (if is_selected { 6.0 } else { 4.0 }) * self.canvas.zoom;
            Self::draw_manhattan_wire_segments(
                src_pos,
                tgt_pos,
                glow_thickness,
                glow_color,
                self.canvas.zoom,
            );
        }

        Self::draw_manhattan_wire_segments(src_pos, tgt_pos, thickness, color, self.canvas.zoom);

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
        point: Vec2,
        threshold: f32,
    ) -> bool {
        let zoom = self.canvas.zoom;
        let mut segments = Vec::new();

        if tgt_pos.x >= src_pos.x + 20.0 * zoom {
            let mid_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0;
            segments.push((Vec2::new(src_pos.x, src_pos.y), Vec2::new(mid_x, src_pos.y)));
            segments.push((Vec2::new(mid_x, src_pos.y), Vec2::new(mid_x, tgt_pos.y)));
            segments.push((Vec2::new(mid_x, tgt_pos.y), Vec2::new(tgt_pos.x, tgt_pos.y)));
        } else {
            let stub_src = src_pos.x + 20.0 * zoom;
            let stub_tgt = tgt_pos.x - 20.0 * zoom;

            let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0;
            if (tgt_pos.y - src_pos.y).abs() < 10.0 * zoom {
                mid_y += 35.0 * zoom;
            }

            segments.push((Vec2::new(src_pos.x, src_pos.y), Vec2::new(stub_src, src_pos.y)));
            segments.push((Vec2::new(stub_src, src_pos.y), Vec2::new(stub_src, mid_y)));
            segments.push((Vec2::new(stub_src, mid_y), Vec2::new(stub_tgt, mid_y)));
            segments.push((Vec2::new(stub_tgt, mid_y), Vec2::new(stub_tgt, tgt_pos.y)));
            segments.push((Vec2::new(stub_tgt, tgt_pos.y), Vec2::new(tgt_pos.x, tgt_pos.y)));
        }

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
