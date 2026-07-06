use macroquad::prelude::*;

use super::Editor;
use super::theme;
use super::types::VisualConnection;

/// The type of wire intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JunctionType {
    /// Wires are electrically connected at this point (shared endpoint / branch).
    Connected,
    /// Wires merely cross without connection.
    Crossing,
}

#[derive(Debug, Clone)]
pub struct WireIntersection {
    pub point: Vec2,
    pub junction_type: JunctionType,
    /// Direction of the "upper" wire at this crossing (for drawing the bridge arc).
    /// Only meaningful for Crossing type. true = horizontal upper wire, false = vertical.
    pub upper_horizontal: bool,
}

/// A wire segment with an associated connection identity.
#[derive(Debug, Clone)]
pub struct IdentifiedSegment {
    pub a: Vec2,
    pub b: Vec2,
    pub conn_idx: usize,
}

impl Editor {
    /// Compute all Manhattan wire segments for a given connection, in screen space.
    pub fn compute_wire_segments_screen(
        src_pos: Vec2,
        tgt_pos: Vec2,
        zoom: f32,
    ) -> Vec<(Vec2, Vec2)> {
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
        segments
    }

    /// Find all wire intersections (junctions and crossings) across all connections.
    pub fn find_wire_intersections(&self) -> Vec<WireIntersection> {
        let mut all_segments: Vec<IdentifiedSegment> = Vec::new();

        // Build all segments with their connection index
        for (conn_idx, wire) in self.connections.iter().enumerate() {
            let src_comp = self.components.iter().find(|c| c.id == wire.src_comp_id);
            let tgt_comp = self.components.iter().find(|c| c.id == wire.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (_, src_outputs) = self.get_component_ports_count(src.comp_type);
                let (tgt_inputs, _) = self.get_component_ports_count(tgt.comp_type);

                let src_pos = self.to_screen_space(src.output_port_pos(wire.src_port, src_outputs));
                let tgt_pos = self.to_screen_space(tgt.input_port_pos(wire.tgt_port, tgt_inputs));

                let segments = Self::compute_wire_segments_screen(src_pos, tgt_pos, self.canvas.zoom);
                for (a, b) in segments {
                    all_segments.push(IdentifiedSegment { a, b, conn_idx });
                }
            }
        }

        let mut intersections = Vec::new();
        let mut seen_points: Vec<Vec2> = Vec::new();

        let epsilon = 2.0 * self.canvas.zoom;

        // Check every pair of segments from different connections
        for i in 0..all_segments.len() {
            for j in (i + 1)..all_segments.len() {
                if all_segments[i].conn_idx == all_segments[j].conn_idx {
                    continue; // Same wire, skip
                }

                if let Some(point) = segment_intersection(
                    all_segments[i].a, all_segments[i].b,
                    all_segments[j].a, all_segments[j].b,
                    epsilon,
                ) {
                    // Check if we already have a nearby intersection point
                    let already_seen = seen_points.iter().any(|p| p.distance(point) < epsilon * 2.0);
                    if already_seen {
                        continue;
                    }
                    seen_points.push(point);

                    // Classify: are these wires connected?
                    let conn_i = &self.connections[all_segments[i].conn_idx];
                    let conn_j = &self.connections[all_segments[j].conn_idx];

                    let connected = wires_share_endpoint(conn_i, conn_j);

                    // Determine which wire is "upper" for bridge arc direction
                    let seg_i_horizontal = is_horizontal(all_segments[i].a, all_segments[i].b);

                    intersections.push(WireIntersection {
                        point,
                        junction_type: if connected {
                            JunctionType::Connected
                        } else {
                            JunctionType::Crossing
                        },
                        upper_horizontal: seg_i_horizontal,
                    });
                }
            }
        }

        intersections
    }

    /// Draw junction indicators (filled dots for connected, bridge arcs for crossings).
    pub fn draw_wire_junctions(&self, intersections: &[WireIntersection]) {
        for intersection in intersections {
            match intersection.junction_type {
                JunctionType::Connected => {
                    // Filled dot indicating electrical connection
                    let radius = 4.0 * self.canvas.zoom;
                    draw_circle(
                        intersection.point.x,
                        intersection.point.y,
                        radius,
                        theme::ACCENT_PRIMARY.mq(),
                    );
                }
                JunctionType::Crossing => {
                    // Bridge arc — small semicircle hop
                    let arc_radius = 6.0 * self.canvas.zoom;
                    draw_bridge_arc(
                        intersection.point,
                        arc_radius,
                        intersection.upper_horizontal,
                        theme::BG_CANVAS.mq(),
                        1.8 * self.canvas.zoom,
                        theme::ACCENT_GENERIC.mq(),
                    );
                }
            }
        }
    }
}

/// Check if a segment is horizontal (vs vertical).
fn is_horizontal(a: Vec2, b: Vec2) -> bool {
    (a.y - b.y).abs() < (a.x - b.x).abs()
}

/// Check if two VisualConnections share any endpoint (connected junction).
fn wires_share_endpoint(a: &VisualConnection, b: &VisualConnection) -> bool {
    // They share a source or target component+port
    (a.src_comp_id == b.src_comp_id && a.src_port == b.src_port)
        || (a.tgt_comp_id == b.tgt_comp_id && a.tgt_port == b.tgt_port)
        || (a.src_comp_id == b.tgt_comp_id && a.src_port == b.tgt_port)
        || (a.tgt_comp_id == b.src_comp_id && a.tgt_port == b.src_port)
}

/// Find the intersection point of two axis-aligned (orthogonal) line segments, if any.
/// Returns None if they're parallel or don't actually cross.
fn segment_intersection(
    a1: Vec2, a2: Vec2,
    b1: Vec2, b2: Vec2,
    epsilon: f32,
) -> Option<Vec2> {
    let a_horiz = (a1.y - a2.y).abs() < epsilon;
    let b_horiz = (b1.y - b2.y).abs() < epsilon;

    // Both same orientation: check for collinear overlap
    if a_horiz == b_horiz {
        if a_horiz {
            // Both horizontal
            if (a1.y - b1.y).abs() < epsilon {
                // Same Y — check X overlap
                let a_min_x = a1.x.min(a2.x);
                let a_max_x = a1.x.max(a2.x);
                let b_min_x = b1.x.min(b2.x);
                let b_max_x = b1.x.max(b2.x);
                let overlap_min = a_min_x.max(b_min_x);
                let overlap_max = a_max_x.min(b_max_x);
                if overlap_min <= overlap_max + epsilon {
                    return Some(Vec2::new((overlap_min + overlap_max) / 2.0, a1.y));
                }
            }
        } else {
            // Both vertical
            if (a1.x - b1.x).abs() < epsilon {
                let a_min_y = a1.y.min(a2.y);
                let a_max_y = a1.y.max(a2.y);
                let b_min_y = b1.y.min(b2.y);
                let b_max_y = b1.y.max(b2.y);
                let overlap_min = a_min_y.max(b_min_y);
                let overlap_max = a_max_y.min(b_max_y);
                if overlap_min <= overlap_max + epsilon {
                    return Some(Vec2::new(a1.x, (overlap_min + overlap_max) / 2.0));
                }
            }
        }
        return None;
    }

    // One horizontal, one vertical — standard orthogonal intersection
    let (h_seg_a, h_seg_b, v_seg_a, v_seg_b) = if a_horiz {
        (a1, a2, b1, b2)
    } else {
        (b1, b2, a1, a2)
    };

    let h_y = h_seg_a.y;
    let h_min_x = h_seg_a.x.min(h_seg_b.x);
    let h_max_x = h_seg_a.x.max(h_seg_b.x);

    let v_x = v_seg_a.x;
    let v_min_y = v_seg_a.y.min(v_seg_b.y);
    let v_max_y = v_seg_a.y.max(v_seg_b.y);

    // Check if the intersection point lies within both segments
    if v_x >= h_min_x - epsilon
        && v_x <= h_max_x + epsilon
        && h_y >= v_min_y - epsilon
        && h_y <= v_max_y + epsilon
    {
        Some(Vec2::new(v_x, h_y))
    } else {
        None
    }
}

/// Draw a bridge arc (semicircle bump) at a crossing point.
/// The arc goes perpendicular to the "upper" wire direction.
fn draw_bridge_arc(
    center: Vec2,
    radius: f32,
    upper_is_horizontal: bool,
    bg_color: Color,
    thickness: f32,
    wire_color: Color,
) {
    // First, draw a background circle to "erase" the lower wire at the crossing
    draw_circle(center.x, center.y, radius, bg_color);

    // Then draw a semicircle arc for the bridge
    let segments = 12;
    let (start_angle, end_angle) = if upper_is_horizontal {
        // Horizontal wire hops over: arc goes upward (from -PI to 0)
        (-std::f32::consts::PI, 0.0_f32)
    } else {
        // Vertical wire hops over: arc goes rightward (from -PI/2 to PI/2)
        (-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2)
    };

    for i in 0..segments {
        let t0 = start_angle + (end_angle - start_angle) * (i as f32 / segments as f32);
        let t1 = start_angle + (end_angle - start_angle) * ((i + 1) as f32 / segments as f32);

        let p0 = Vec2::new(center.x + radius * t0.cos(), center.y + radius * t0.sin());
        let p1 = Vec2::new(center.x + radius * t1.cos(), center.y + radius * t1.sin());

        draw_line(p0.x, p0.y, p1.x, p1.y, thickness, wire_color);
    }
}
