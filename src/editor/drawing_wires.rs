use macroquad::prelude::*;

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
            Self::draw_manhattan_wire_segments(
                src_pos,
                tgt_pos,
                glow_thickness,
                glow_color,
                self.zoom,
            );
        }

        Self::draw_manhattan_wire_segments(src_pos, tgt_pos, thickness, color, self.zoom);

        // Draw concentric terminal circle/indicator at target
        let port_radius = 4.0 * self.zoom;
        if wire_state {
            draw_circle(
                tgt_pos.x,
                tgt_pos.y,
                port_radius + 2.0 * self.zoom,
                Color::new(0.00, 0.70, 1.00, 0.2),
            );
        }
        draw_circle(tgt_pos.x, tgt_pos.y, port_radius, color);
        draw_circle(
            tgt_pos.x,
            tgt_pos.y,
            2.0 * self.zoom,
            Color::new(0.09, 0.10, 0.12, 1.0),
        );
    }
}
