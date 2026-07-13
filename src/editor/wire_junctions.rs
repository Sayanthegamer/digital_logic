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
    pub lower_color: Color,
    pub lower_thickness: f32,
    pub upper_color: Color,
    pub upper_thickness: f32,
}

/// A wire segment with an associated connection identity.
#[derive(Debug, Clone)]
pub struct IdentifiedSegment {
    pub a: Vec2,
    pub b: Vec2,
    pub conn_idx: usize,
}

impl Editor {
    pub fn get_connection_style(&self, conn: &VisualConnection) -> (Color, f32) {
        let wire_state = self.get_raw_wire_state(conn.src_comp_id, conn.src_port);

        let (base_color, thickness, _) = match wire_state {
            0b00 => (theme::ACCENT_GENERIC.mq(), 1.3 * self.canvas.zoom, false),
            0b01 => (theme::ACCENT_INACTIVE.mq(), 1.6 * self.canvas.zoom, false),
            0b10 => (theme::ACCENT_PRIMARY.mq(), 2.2 * self.canvas.zoom, true),
            _ => (theme::COMP_NAND.mq(), 2.8 * self.canvas.zoom, true),
        };
        let color = self.circuit.color_overrides.get_wire_color(conn).unwrap_or(base_color);
        (color, thickness)
    }

    pub fn get_connection_routing_offset(&self, conn: &VisualConnection) -> f32 {
        let base_offset = self.circuit.wire_offsets.get(conn).copied().unwrap_or(0.0);
        let manual_nudge = self.circuit.wire_nudges.get(conn).copied().unwrap_or(0.0);
        base_offset + manual_nudge
    }

    pub fn recompute_wire_offsets(&mut self, affected_comps: Option<&std::collections::HashSet<usize>>) {
        if self.circuit.connections.len() > 5000 { return; }
        let mut wire_offsets = if affected_comps.is_some() {
            self.circuit.wire_offsets.clone()
        } else {
            std::collections::HashMap::new()
        };

        struct ConnectionSegments {
            conn: VisualConnection,
            vertical_segs: Vec<VerticalSeg>,
            source_y: f32,
        }
        struct VerticalSeg {
            ideal_x: f32,
            y_min: f32,
            y_max: f32,
        }

        let mut conn_data = Vec::new();

        for conn in &self.circuit.connections {
            if let Some(affected) = affected_comps {
                if !affected.contains(&conn.src_comp_id) && !affected.contains(&conn.tgt_comp_id) {
                    continue;
                }
            }

            let src_comp = self.get_component(conn.src_comp_id);
            let tgt_comp = self.get_component(conn.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (src_pos, tgt_pos) = self.get_connection_ports(conn, src, tgt);

                let mut vertical_segs = Vec::new();

                if tgt_pos.x >= src_pos.x + 20.0 {
                    // Forward routing: 1 vertical segment
                    let ideal_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0;
                    vertical_segs.push(VerticalSeg {
                        ideal_x,
                        y_min: src_pos.y.min(tgt_pos.y),
                        y_max: src_pos.y.max(tgt_pos.y),
                    });
                } else {
                    // Backward routing: 2 vertical segments
                    let stub_src = src_pos.x + 20.0;
                    let target_stagger = conn.tgt_port as f32 * 6.0;
                    let stub_tgt = tgt_pos.x - 20.0 - target_stagger;

                    let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0;
                    if (tgt_pos.y - src_pos.y).abs() < 10.0 {
                        mid_y += 35.0;
                    }

                    vertical_segs.push(VerticalSeg {
                        ideal_x: stub_src,
                        y_min: src_pos.y.min(mid_y),
                        y_max: src_pos.y.max(mid_y),
                    });
                    vertical_segs.push(VerticalSeg {
                        ideal_x: stub_tgt,
                        y_min: mid_y.min(tgt_pos.y),
                        y_max: mid_y.max(tgt_pos.y),
                    });
                }

                conn_data.push(ConnectionSegments {
                    conn: *conn,
                    vertical_segs,
                    source_y: src_pos.y,
                });
            }
        }

        // Spatial Sorting: Sort connections so lane assignment is deterministic and ordered
        // We sort by main vertical corridor ideal X, then Y span start, then Y source coordinate
        conn_data.sort_by(|a, b| {
            let a_x = a.vertical_segs.first().map(|s| s.ideal_x).unwrap_or(0.0);
            let b_x = b.vertical_segs.first().map(|s| s.ideal_x).unwrap_or(0.0);
            
            a_x.partial_cmp(&b_x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.source_y.partial_cmp(&b.source_y).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Greedy interval coloring to assign non-overlapping lanes
        let mut assigned_lanes = vec![0; conn_data.len()];

        for i in 0..conn_data.len() {
            let mut occupied_lanes = std::collections::HashSet::new();
            for j in 0..i {
                let mut conflict = false;
                for s1 in &conn_data[i].vertical_segs {
                    for s2 in &conn_data[j].vertical_segs {
                        // Check if they route in the same corridor
                        let same_corridor = (s1.ideal_x - s2.ideal_x).abs() < 15.0;
                        // Check if their Y spans overlap (with 4px margin)
                        let y_overlap = s1.y_min - 4.0 < s2.y_max && s2.y_min - 4.0 < s1.y_max;

                        if same_corridor && y_overlap {
                            conflict = true;
                            break;
                        }
                    }
                    if conflict {
                        break;
                    }
                }
                if conflict {
                    occupied_lanes.insert(assigned_lanes[j]);
                }
            }

            let mut lane = 0;
            while occupied_lanes.contains(&lane) {
                lane += 1;
            }
            assigned_lanes[i] = lane;
            
            // Convert lane number to pixel offset
            // Alternating pattern: 0, 6, -6, 12, -12...
            let pixel_offset = if lane % 2 == 0 {
                (lane / 2) as f32 * 6.0
            } else {
                -(((lane + 1) / 2) as f32) * 6.0
            };
            
            wire_offsets.insert(conn_data[i].conn, pixel_offset);
        }

        self.circuit.wire_offsets = wire_offsets;
    }

    pub fn get_blueprint_connection_routing_offset(
        &self,
        conn: &crate::engine::Connection,
        blueprint: &crate::engine::ChipBlueprint,
    ) -> f32 {
        // Find all connections sharing the same source
        let mut sharing: Vec<&crate::engine::Connection> = blueprint.connections.iter()
            .filter(|c| c.source == conn.source)
            .collect();

        let fanout_offset = if sharing.len() <= 1 {
            0.0
        } else {
            sharing.sort_by(|a, b| {
                use crate::engine::TargetPort;
                let (type_a, idx_a, port_a) = match a.target {
                    TargetPort::ChipOutput(i) => (1, i, 0),
                    TargetPort::ComponentInput { component_idx, port_idx } => (0, component_idx, port_idx),
                };
                let (type_b, idx_b, port_b) = match b.target {
                    TargetPort::ChipOutput(i) => (1, i, 0),
                    TargetPort::ComponentInput { component_idx, port_idx } => (0, component_idx, port_idx),
                };

                type_a.cmp(&type_b)
                    .then(idx_a.cmp(&idx_b))
                    .then(port_a.cmp(&port_b))
            });

            if let Some(idx) = sharing.iter().position(|c| c.target == conn.target) {
                let n = sharing.len() as f32;
                (idx as f32 - (n - 1.0) / 2.0) * 10.0
            } else {
                0.0
            }
        };

        use crate::engine::TargetPort;
        let (tgt_comp_id, tgt_port) = match conn.target {
            TargetPort::ChipOutput(i) => (8888, i),
            TargetPort::ComponentInput { component_idx, port_idx } => (component_idx, port_idx),
        };
        let hash = (tgt_comp_id + tgt_port) % 3;
        let hash_offset = (hash as f32 - 1.0) * 4.0;

        fanout_offset + hash_offset
    }

    /// Compute all Manhattan wire segments for a given connection, in screen space.
    pub fn compute_wire_segments_screen(
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        tgt_port: usize,
        zoom: f32,
    ) -> Vec<(Vec2, Vec2)> {
        let mut segments = Vec::new();
        if tgt_pos.x >= src_pos.x + 20.0 * zoom {
            let offset = routing_offset * zoom;
            let mut mid_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0 + offset;
            mid_x = mid_x.clamp(src_pos.x + 8.0 * zoom, tgt_pos.x - 8.0 * zoom);

            segments.push((Vec2::new(src_pos.x, src_pos.y), Vec2::new(mid_x, src_pos.y)));
            segments.push((Vec2::new(mid_x, src_pos.y), Vec2::new(mid_x, tgt_pos.y)));
            segments.push((Vec2::new(mid_x, tgt_pos.y), Vec2::new(tgt_pos.x, tgt_pos.y)));
        } else {
            let offset_x = routing_offset.abs() * 0.7 * zoom;
            let offset_y = routing_offset * zoom;
            let target_stagger = (tgt_port as f32 * 6.0) * zoom; // Stagger backward routing

            let stub_src = src_pos.x + 20.0 * zoom + offset_x;
            let stub_tgt = tgt_pos.x - 20.0 * zoom - offset_x - target_stagger;

            let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0 + offset_y;
            if (tgt_pos.y - src_pos.y).abs() < 10.0 * zoom {
                mid_y += 35.0 * zoom;
            }

            segments.push((Vec2::new(src_pos.x, src_pos.y), Vec2::new(stub_src, src_pos.y)));
            segments.push((Vec2::new(stub_src, src_pos.y), Vec2::new(stub_src, mid_y)));
            segments.push((Vec2::new(stub_src, mid_y), Vec2::new(stub_tgt, mid_y)));
            segments.push((Vec2::new(stub_tgt, mid_y), Vec2::new(stub_tgt, tgt_pos.y)));
            segments.push((Vec2::new(stub_tgt, tgt_pos.y), Vec2::new(tgt_pos.x, tgt_pos.y)));
        }
        Self::chamfer_segments(segments, 10.0 * zoom)
    }

    fn chamfer_segments(segments: Vec<(Vec2, Vec2)>, base_radius: f32) -> Vec<(Vec2, Vec2)> {
        if segments.len() <= 1 {
            return segments;
        }

        let mut out = Vec::new();
        let mut prev_b_new = segments[0].0;

        for i in 0..segments.len() - 1 {
            let (a, b) = segments[i];
            let (c, d) = segments[i + 1];
            
            let len_ab = a.distance(b);
            let len_cd = c.distance(d);
            
            let r = base_radius.min(len_ab / 2.0).min(len_cd / 2.0);
            
            if r <= 0.1 {
                out.push((prev_b_new, b));
                prev_b_new = b;
                continue;
            }

            let dir_ab = (b - a) / len_ab;
            let dir_cd = (d - c) / len_cd;

            let b_new = b - dir_ab * r;
            let c_new = c + dir_cd * r;

            out.push((prev_b_new, b_new));
            out.push((b_new, c_new));
            
            prev_b_new = c_new;
        }
        
        let last = segments.last().unwrap();
        out.push((prev_b_new, last.1));
        
        out
    }

    /// Compute all Manhattan wire segments in world space (without zoom).
    pub fn compute_wire_segments_world(
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        tgt_port: usize,
    ) -> Vec<(Vec2, Vec2)> {
        Self::compute_wire_segments_screen(src_pos, tgt_pos, routing_offset, tgt_port, 1.0)
    }

    /// Find all wire intersections (junctions and crossings) across all connections.
    pub fn find_wire_intersections(&self) -> Vec<WireIntersection> {
        let mut all_segments = Vec::new();
        
        let sw = macroquad::window::screen_width();
        let sh = macroquad::window::screen_height();
        let screen_rect = Rect::new(-20.0, -20.0, sw + 40.0, sh + 40.0);

        for (conn_idx, wire) in self.circuit.connections.iter().enumerate() {
            let src_comp = self.get_component(wire.src_comp_id);
            let tgt_comp = self.get_component(wire.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (src_p, tgt_p) = self.get_connection_ports(wire, src, tgt);
                let src_pos = self.to_screen_space(src_p);
                let tgt_pos = self.to_screen_space(tgt_p);

                let offset = self.get_connection_routing_offset(wire);
                let segments = Self::compute_wire_segments_screen(
                    src_pos,
                    tgt_pos,
                    offset,
                    wire.tgt_port,
                    self.canvas.zoom,
                );
                
                for (a, b) in segments {
                    let min_x = a.x.min(b.x);
                    let max_x = a.x.max(b.x);
                    let min_y = a.y.min(b.y);
                    let max_y = a.y.max(b.y);
                    
                    // Frustum Culling
                    if min_x > screen_rect.right() || max_x < screen_rect.left() || 
                       min_y > screen_rect.bottom() || max_y < screen_rect.top() {
                        continue;
                    }

                    all_segments.push(IdentifiedSegment { a, b, conn_idx });
                }
            }
        }

        // 1D Sweep and Prune (Sort by minimum X coordinate)
        all_segments.sort_by(|s1, s2| {
            let min1 = s1.a.x.min(s1.b.x);
            let min2 = s2.a.x.min(s2.b.x);
            min1.partial_cmp(&min2).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut intersections = Vec::new();
        let mut seen_points: std::collections::HashSet<(i32, i32)> = std::collections::HashSet::new();
        let epsilon = 2.0 * self.canvas.zoom;

        for i in 0..all_segments.len() {
            let s1 = &all_segments[i];
            let max1_x = s1.a.x.max(s1.b.x) + epsilon * 2.0;
            
            for j in (i + 1)..all_segments.len() {
                let s2 = &all_segments[j];
                let min2_x = s2.a.x.min(s2.b.x);
                
                if min2_x > max1_x {
                    // Since elements are sorted by min_x, all subsequent elements will also be > max1_x
                    break;
                }

                if s1.conn_idx == s2.conn_idx {
                    continue;
                }

                if let Some(point) = segment_intersection(
                    s1.a, s1.b,
                    s2.a, s2.b,
                    epsilon,
                ) {
                    let bucket_x = (point.x / epsilon).round() as i32;
                    let bucket_y = (point.y / epsilon).round() as i32;
                    let mut already_seen = false;
                    for dx in -1..=1 {
                        for dy in -1..=1 {
                            if seen_points.contains(&(bucket_x + dx, bucket_y + dy)) {
                                already_seen = true;
                                break;
                            }
                        }
                    }
                    if already_seen {
                        continue;
                    }
                    seen_points.insert((bucket_x, bucket_y));

                    let conn_i = &self.circuit.connections[s1.conn_idx];
                    let conn_j = &self.circuit.connections[s2.conn_idx];

                    let connected = wires_share_endpoint(conn_i, conn_j);
                    if connected {
                        continue;
                    }

                    let seg_i_horizontal = is_horizontal(s1.a, s1.b);
                    let seg_j_horizontal = is_horizontal(s2.a, s2.b);

                    let (lower_conn_idx, upper_conn_idx, upper_is_horizontal) = if seg_i_horizontal {
                        (s2.conn_idx, s1.conn_idx, true)
                    } else if seg_j_horizontal {
                        (s1.conn_idx, s2.conn_idx, true)
                    } else {
                        (s2.conn_idx, s1.conn_idx, seg_i_horizontal)
                    };

                    let lower_conn = &self.circuit.connections[lower_conn_idx];
                    let (lower_color, lower_thickness) = self.get_connection_style(lower_conn);

                    let upper_conn = &self.circuit.connections[upper_conn_idx];
                    let (upper_color, upper_thickness) = self.get_connection_style(upper_conn);

                    intersections.push(WireIntersection {
                        point,
                        junction_type: JunctionType::Crossing,
                        upper_horizontal: upper_is_horizontal,
                        lower_color,
                        lower_thickness,
                        upper_color,
                        upper_thickness,
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
                JunctionType::Connected => {} // Ignored
                JunctionType::Crossing => {
                    // Bridge arc — small semicircle hop
                    let arc_radius = 6.0 * self.canvas.zoom;

                    // 1. Draw a background circle to mask/erase both lines
                    draw_circle(
                        intersection.point.x,
                        intersection.point.y,
                        arc_radius,
                        theme::BG_CANVAS.mq(),
                    );

                    // 2. Draw the lower wire segment straight through the center
                    if intersection.upper_horizontal {
                        // Upper wire is horizontal (arc goes up/down).
                        // Lower wire is vertical, draw straight vertical segment.
                        draw_line(
                            intersection.point.x,
                            intersection.point.y - arc_radius,
                            intersection.point.x,
                            intersection.point.y + arc_radius,
                            intersection.lower_thickness,
                            intersection.lower_color,
                        );
                    } else {
                        // Upper wire is vertical (arc goes left/right).
                        // Lower wire is horizontal, draw straight horizontal segment.
                        draw_line(
                            intersection.point.x - arc_radius,
                            intersection.point.y,
                            intersection.point.x + arc_radius,
                            intersection.point.y,
                            intersection.lower_thickness,
                            intersection.lower_color,
                        );
                    }

                    // 3. Draw the bridge arc for the upper wire using its style
                    draw_bridge_arc(
                        intersection.point,
                        arc_radius,
                        intersection.upper_horizontal,
                        intersection.upper_thickness,
                        intersection.upper_color,
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

    // Parallel or collinear segments do not cross
    if a_horiz == b_horiz {
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
    thickness: f32,
    wire_color: Color,
) {
    // Draw a semicircle arc for the bridge
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

        draw_circle(p0.x, p0.y, thickness / 2.0, wire_color);
        draw_circle(p1.x, p1.y, thickness / 2.0, wire_color);
        draw_line(p0.x, p0.y, p1.x, p1.y, thickness, wire_color);
    }
}
