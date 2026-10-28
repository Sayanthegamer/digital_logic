use crate::editor::math::Color;

#[inline(always)]
pub(crate) fn draw_rect(painter: &egui::Painter, x: f32, y: f32, w: f32, h: f32, color: Color) {
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
    painter.rect_filled(rect, 0.0, color.to_egui());
}

#[inline(always)]
pub(crate) fn draw_rect_lines(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    thickness: f32,
    color: Color,
) {
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(thickness, color.to_egui()),
        egui::StrokeKind::Middle,
    );
}

#[inline(always)]
pub(crate) fn draw_rounded_rect(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    color: Color,
) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
    painter.rect_filled(rect, r, color.to_egui());
}

#[inline(always)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_rounded_rect_lines(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    r: f32,
    thickness: f32,
    color: Color,
) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let rect = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h));
    painter.rect_stroke(
        rect,
        r,
        egui::Stroke::new(thickness, color.to_egui()),
        egui::StrokeKind::Middle,
    );
}

#[inline(always)]
pub(crate) fn draw_line_seg(
    painter: &egui::Painter,
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    thickness: f32,
    color: Color,
) {
    painter.line_segment(
        [egui::pos2(x1, y1), egui::pos2(x2, y2)],
        egui::Stroke::new(thickness, color.to_egui()),
    );
}

#[inline(always)]
pub(crate) fn draw_polyline(
    painter: &egui::Painter,
    points: Vec<egui::Pos2>,
    thickness: f32,
    color: Color,
) {
    if points.len() >= 2 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(thickness, color.to_egui()),
        ));
    }
}

#[inline(always)]
pub(crate) fn draw_circle_filled(painter: &egui::Painter, x: f32, y: f32, r: f32, color: Color) {
    painter.circle_filled(egui::pos2(x, y), r.max(0.0), color.to_egui());
}

#[inline(always)]
pub(crate) fn draw_circle_lines(
    painter: &egui::Painter,
    x: f32,
    y: f32,
    r: f32,
    thickness: f32,
    color: Color,
) {
    painter.circle_stroke(
        egui::pos2(x, y),
        r.max(0.0),
        egui::Stroke::new(thickness, color.to_egui()),
    );
}

#[inline(always)]
pub(crate) fn draw_text_label(
    painter: &egui::Painter,
    text: &str,
    x: f32,
    y_baseline: f32,
    font_size: f32,
    color: Color,
) {
    painter.text(
        egui::pos2(x, y_baseline - font_size * 0.85),
        egui::Align2::LEFT_TOP,
        text,
        egui::FontId::proportional(font_size.max(4.0)),
        color.to_egui(),
    );
}

#[inline(always)]
pub(crate) fn draw_text_centered(
    painter: &egui::Painter,
    text: &str,
    center_x: f32,
    center_y: f32,
    font_size: f32,
    color: Color,
) {
    painter.text(
        egui::pos2(center_x, center_y),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(font_size.max(4.0)),
        color.to_egui(),
    );
}

#[inline(always)]
pub(crate) fn draw_text_left_center(
    painter: &egui::Painter,
    text: &str,
    x: f32,
    center_y: f32,
    font_size: f32,
    color: Color,
) {
    painter.text(
        egui::pos2(x, center_y),
        egui::Align2::LEFT_CENTER,
        text,
        egui::FontId::proportional(font_size.max(4.0)),
        color.to_egui(),
    );
}

#[inline(always)]
pub(crate) fn draw_text_right_center(
    painter: &egui::Painter,
    text: &str,
    x: f32,
    center_y: f32,
    font_size: f32,
    color: Color,
) {
    painter.text(
        egui::pos2(x, center_y),
        egui::Align2::RIGHT_CENTER,
        text,
        egui::FontId::proportional(font_size.max(4.0)),
        color.to_egui(),
    );
}
