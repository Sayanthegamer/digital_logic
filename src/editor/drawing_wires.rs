use crate::editor::theme;
use macroquad::prelude::*;

use super::Editor;

impl Editor {
    pub(crate) fn draw_manhattan_wire(
        &self,
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        tgt_port: usize,
        wire_state: u8,
        is_selected: bool,
        color_override: Option<Color>,
        is_bus: bool,
        gaps: &[(Vec2, f32)],
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
            tgt_port,
            self.canvas.zoom,
        );

        let mut final_segments = Vec::new();
        let mut bridge_arcs = Vec::new();

        // Zero-allocation pre-hoisted vectors
        let mut holes: Vec<(f32, f32)> = Vec::with_capacity(gaps.len());
        let mut merged_holes: Vec<(f32, f32)> = Vec::with_capacity(gaps.len());
        let mut visible_intervals: Vec<(f32, f32)> = Vec::with_capacity(gaps.len() + 1);

        for &(a, b) in &segments {
            let line_vec = b - a;
            let len = line_vec.length();
            if len == 0.0 {
                continue;
            }
            let dir = line_vec / len;

            holes.clear();
            for &(gap, lower_thickness) in gaps {
                let t = (gap - a).dot(dir);
                let proj = a + dir * t;
                // dynamic gap radius perfectly matches the lower wire plus a fixed clearance
                let gap_radius = lower_thickness / 2.0 + 3.0 * self.canvas.zoom;
                
                if proj.distance(gap) < 2.0 * self.canvas.zoom && t > -gap_radius && t < len + gap_radius {
                    holes.push((t - gap_radius, t + gap_radius));
                }
            }

            holes.sort_by(|h1, h2| h1.0.partial_cmp(&h2.0).unwrap());
            merged_holes.clear();
            for h in &holes {
                if let Some(last) = merged_holes.last_mut() {
                    if h.0 <= last.1 {
                        last.1 = last.1.max(h.1);
                        continue;
                    }
                }
                merged_holes.push(*h);
            }

            let mut current_t: f32 = 0.0;
            visible_intervals.clear();
            for h in &merged_holes {
                if h.0 > current_t {
                    visible_intervals.push((current_t, h.0.min(len)));
                }
                current_t = current_t.max(h.1);

                // Add ONE dynamic bridge arc for the entire merged hole
                let center_t = (h.0 + h.1) / 2.0;
                let radius = (h.1 - h.0) / 2.0;
                let center = a + dir * center_t;
                bridge_arcs.push((center, radius, dir));
            }
            if current_t < len {
                visible_intervals.push((current_t, len));
            }

            for &(t1, t2) in &visible_intervals {
                if t2 > t1 {
                    let draw_start = t1 == 0.0;
                    let draw_end = t2 == len;
                    final_segments.push((a + dir * t1, a + dir * t2, draw_start, draw_end));
                }
            }
        }

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
            for &(p1, p2, draw_start, draw_end) in &final_segments {
                if draw_start { draw_circle(p1.x, p1.y, glow_thickness / 2.0, glow_color); }
                if draw_end { draw_circle(p2.x, p2.y, glow_thickness / 2.0, glow_color); }
                draw_line(p1.x, p1.y, p2.x, p2.y, glow_thickness, glow_color);
            }
            for &(center, radius, dir) in &bridge_arcs {
                Self::draw_bridge_arc(center, radius, dir, glow_thickness, glow_color);
            }
        }

        for &(p1, p2, draw_start, draw_end) in &final_segments {
            if draw_start { draw_circle(p1.x, p1.y, thickness / 2.0, color); }
            if draw_end { draw_circle(p2.x, p2.y, thickness / 2.0, color); }
            draw_line(p1.x, p1.y, p2.x, p2.y, thickness, color);
        }
        for &(center, radius, dir) in &bridge_arcs {
            Self::draw_bridge_arc(center, radius, dir, thickness, color);
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

    fn draw_bridge_arc(
        center: Vec2,
        radius: f32,
        dir: Vec2,
        thickness: f32,
        wire_color: Color,
    ) {
        let segments = 12 + (radius / 5.0) as usize; 
        
        let mut normal = Vec2::new(dir.y, -dir.x);
        if normal.y > 0.0 {
            normal = -normal;
        } else if normal.y == 0.0 && normal.x < 0.0 {
            normal = -normal;
        }

        for i in 0..segments {
            let t0 = std::f32::consts::PI * (i as f32 / segments as f32);
            let t1 = std::f32::consts::PI * ((i + 1) as f32 / segments as f32);

            let p0 = center - dir * (radius * t0.cos()) + normal * (radius * t0.sin());
            let p1 = center - dir * (radius * t1.cos()) + normal * (radius * t1.sin());

            draw_line(p0.x, p0.y, p1.x, p1.y, thickness, wire_color);
        }
    }

    pub(crate) fn hit_test_manhattan_wire(
        &self,
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        tgt_port: usize,
        point: Vec2,
        threshold: f32,
    ) -> bool {
        let segments = Self::compute_wire_segments_world(src_pos, tgt_pos, routing_offset, tgt_port);

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
