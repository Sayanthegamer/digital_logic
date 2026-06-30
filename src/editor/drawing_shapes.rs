use macroquad::prelude::*;

pub(crate) fn draw_rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
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

pub(crate) fn draw_rounded_rect_lines(x: f32, y: f32, w: f32, h: f32, r: f32, thickness: f32, color: Color) {
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
        let mut last_point = Vec2::new(cx + r * start_angle.cos(), cy + r * start_angle.sin());
        for i in 1..=segments {
            let angle = start_angle + (end_angle - start_angle) * (i as f32 / segments as f32);
            let point = Vec2::new(cx + r * angle.cos(), cy + r * angle.sin());
            draw_line(
                last_point.x,
                last_point.y,
                point.x,
                point.y,
                thickness,
                color,
            );
            last_point = point;
        }
    };

    use std::f32::consts::PI;
    draw_arc(x + r, y + r, PI, 1.5 * PI);
    draw_arc(x + w - r, y + r, 1.5 * PI, 2.0 * PI);
    draw_arc(x + w - r, y + h - r, 0.0, 0.5 * PI);
    draw_arc(x + r, y + h - r, 0.5 * PI, PI);
}
