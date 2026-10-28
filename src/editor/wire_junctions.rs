use crate::editor::drawing_shapes::draw_circle_filled;
use crate::editor::math::{Color, Rect, Vec2};

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

#[derive(Debug, Clone, PartialEq)]
pub struct VerticalSeg {
    pub ideal_x: f32,
    pub y_min: f32,
    pub y_max: f32,
}

#[derive(Debug, Clone)]
pub struct WireIntersection {
    pub point: Vec2,
    pub junction_type: JunctionType,
    pub upper_conn: Option<VisualConnection>,
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
    pub seg_id: usize,
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
        let color = self
            .circuit
            .color_overrides
            .get_wire_color(conn)
            .unwrap_or(base_color);
        (color, thickness)
    }

    pub fn get_connection_routing_offset(&self, conn: &VisualConnection) -> f32 {
        let base_offset = self.circuit.wire_offsets.get(conn).copied().unwrap_or(0.0);
        let manual_nudge = self.circuit.wire_nudges.get(conn).copied().unwrap_or(0.0);
        base_offset + manual_nudge
    }

    pub fn recompute_wire_offsets(
        &mut self,
        _affected_comps: Option<&std::collections::HashSet<usize>>,
    ) {
        #[derive(Clone)]
        struct RoutedConn {
            conn: VisualConnection,
            ideal_x: f32,
            y_min: f32,
            y_max: f32,
            src_y: f32,
            tgt_y: f32,
            vertical_segs: Vec<VerticalSeg>,
        }

        let mut conn_data = Vec::with_capacity(self.circuit.connections.len());

        for conn in &self.circuit.connections {
            let src_comp = self.get_component(conn.src_comp_id);
            let tgt_comp = self.get_component(conn.tgt_comp_id);

            if let (Some(src), Some(tgt)) = (src_comp, tgt_comp) {
                let (src_pos, tgt_pos) = self.get_connection_ports(conn, src, tgt);

                let mut vertical_segs = Vec::new();
                let ideal_x: f32;
                let y_min: f32;
                let y_max: f32;

                if tgt_pos.x >= src_pos.x + 20.0 {
                    ideal_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0;
                    y_min = src_pos.y.min(tgt_pos.y);
                    y_max = src_pos.y.max(tgt_pos.y);
                    vertical_segs.push(VerticalSeg {
                        ideal_x,
                        y_min,
                        y_max,
                    });
                } else {
                    let stub_src = src_pos.x + 20.0;
                    let target_stagger = conn.tgt_port as f32 * 6.0;
                    let stub_tgt = tgt_pos.x - 20.0 - target_stagger;

                    let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0;
                    if (tgt_pos.y - src_pos.y).abs() < 10.0 {
                        mid_y += 35.0;
                    }

                    ideal_x = stub_src;
                    y_min = src_pos.y.min(tgt_pos.y).min(mid_y);
                    y_max = src_pos.y.max(tgt_pos.y).max(mid_y);

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

                conn_data.push(RoutedConn {
                    conn: *conn,
                    ideal_x,
                    y_min,
                    y_max,
                    src_y: src_pos.y,
                    tgt_y: tgt_pos.y,
                    vertical_segs,
                });
            }
        }

        // 1. Sort all connections by ideal_x to cluster into vertical routing corridors
        conn_data.sort_by(|a, b| {
            a.ideal_x
                .partial_cmp(&b.ideal_x)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(
                    a.src_y
                        .partial_cmp(&b.src_y)
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });

        let mut corridors: Vec<Vec<RoutedConn>> = Vec::new();
        for item in conn_data {
            if let Some(last_corridor) = corridors.last_mut() {
                let avg_x = last_corridor.iter().map(|c| c.ideal_x).sum::<f32>()
                    / last_corridor.len() as f32;
                if (item.ideal_x - avg_x).abs() <= 28.0 {
                    last_corridor.push(item);
                    continue;
                }
            }
            corridors.push(vec![item]);
        }

        let mut wire_offsets =
            std::collections::HashMap::with_capacity(self.circuit.connections.len());
        let mut grid: std::collections::HashMap<
            (i32, i32),
            Vec<(usize, VerticalSeg, VisualConnection)>,
        > = std::collections::HashMap::new();
        let cell_size_y = 100.0;
        let cell_size_x = 20.0;
        let lane_spacing = 8.0_f32;

        // 2. Within each corridor, split into Y-overlapping interval clusters and assign crossing-free ribbon lanes
        for mut corridor in corridors {
            corridor.sort_by(|a, b| {
                a.y_min
                    .partial_cmp(&b.y_min)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(
                        a.y_max
                            .partial_cmp(&b.y_max)
                            .unwrap_or(std::cmp::Ordering::Equal),
                    )
            });

            let mut y_clusters: Vec<Vec<RoutedConn>> = Vec::new();
            let mut current_cluster: Vec<RoutedConn> = Vec::new();
            let mut cluster_max_y = f32::MIN;

            for item in corridor {
                if current_cluster.is_empty() {
                    cluster_max_y = item.y_max;
                    current_cluster.push(item);
                } else if item.y_min <= cluster_max_y + 4.0 {
                    cluster_max_y = cluster_max_y.max(item.y_max);
                    current_cluster.push(item);
                } else {
                    y_clusters.push(current_cluster);
                    cluster_max_y = item.y_max;
                    current_cluster = vec![item];
                }
            }
            if !current_cluster.is_empty() {
                y_clusters.push(current_cluster);
            }

            for cluster in y_clusters {
                // Group by electrical source net (src_comp_id, src_port) so fan-out branches share a single vertical trunk lane
                struct NetGroup {
                    net_key: (usize, usize),
                    conns: Vec<RoutedConn>,
                    avg_src_y: f32,
                    avg_tgt_y: f32,
                }

                let mut net_map: std::collections::HashMap<(usize, usize), Vec<RoutedConn>> =
                    std::collections::HashMap::new();
                for item in cluster {
                    net_map
                        .entry((item.conn.src_comp_id, item.conn.src_port))
                        .or_default()
                        .push(item);
                }

                let mut nets: Vec<NetGroup> = net_map
                    .into_iter()
                    .map(|(net_key, conns)| {
                        let n = conns.len() as f32;
                        let avg_src_y = conns.iter().map(|c| c.src_y).sum::<f32>() / n;
                        let avg_tgt_y = conns.iter().map(|c| c.tgt_y).sum::<f32>() / n;
                        NetGroup {
                            net_key,
                            conns,
                            avg_src_y,
                            avg_tgt_y,
                        }
                    })
                    .collect();

                // Crossing-free monotonic ribbon ordering:
                // - For downward-going wires (avg_tgt_y >= avg_src_y), inner (larger src_y) nets must sit to the left (smaller X offset)
                //   and outer (smaller src_y) nets must sit to the right (larger X offset) so their L-bends nest without crossing.
                // - For upward-going wires (avg_tgt_y < avg_src_y), inner (smaller src_y) nets must sit to the left (smaller X offset)
                //   and outer (larger src_y) nets must sit to the right (larger X offset).
                nets.sort_by(|a, b| {
                    let a_down = a.avg_tgt_y >= a.avg_src_y;
                    let b_down = b.avg_tgt_y >= b.avg_src_y;

                    match (a_down, b_down) {
                        (true, true) => b
                            .avg_src_y
                            .partial_cmp(&a.avg_src_y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(
                                b.avg_tgt_y
                                    .partial_cmp(&a.avg_tgt_y)
                                    .unwrap_or(std::cmp::Ordering::Equal),
                            )
                            .then(a.net_key.cmp(&b.net_key)),
                        (false, false) => a
                            .avg_src_y
                            .partial_cmp(&b.avg_src_y)
                            .unwrap_or(std::cmp::Ordering::Equal)
                            .then(
                                a.avg_tgt_y
                                    .partial_cmp(&b.avg_tgt_y)
                                    .unwrap_or(std::cmp::Ordering::Equal),
                            )
                            .then(a.net_key.cmp(&b.net_key)),
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                    }
                });

                let num_lanes = nets.len();
                let center_lane = (num_lanes.saturating_sub(1)) as f32 / 2.0;

                for (lane_idx, net) in nets.into_iter().enumerate() {
                    let pixel_offset = if num_lanes <= 1 {
                        0.0
                    } else {
                        (lane_idx as f32 - center_lane) * lane_spacing
                    };

                    for item in net.conns {
                        wire_offsets.insert(item.conn, pixel_offset);

                        for s in &item.vertical_segs {
                            let col = (s.ideal_x / cell_size_x).floor() as i32;
                            let start_row = (s.y_min / cell_size_y).floor() as i32;
                            let end_row = (s.y_max / cell_size_y).floor() as i32;
                            for r in start_row..=end_row {
                                grid.entry((col, r)).or_default().push((
                                    lane_idx,
                                    VerticalSeg {
                                        ideal_x: s.ideal_x,
                                        y_min: s.y_min,
                                        y_max: s.y_max,
                                    },
                                    item.conn,
                                ));
                            }
                        }
                    }
                }
            }
        }

        self.circuit.wire_offsets = wire_offsets;
        self.circuit.wire_lane_grid = grid;
    }

    pub fn get_blueprint_connection_routing_offset(
        &self,
        conn: &crate::engine::Connection,
        blueprint: &crate::engine::ChipBlueprint,
    ) -> f32 {
        let mut sharing: Vec<&crate::engine::Connection> = blueprint
            .connections
            .iter()
            .filter(|c| c.source == conn.source)
            .collect();

        let fanout_offset = if sharing.len() <= 1 {
            0.0
        } else {
            sharing.sort_by(|a, b| {
                use crate::engine::TargetPort;
                let (type_a, idx_a, port_a) = match a.target {
                    TargetPort::ChipOutput(i) => (1, i, 0),
                    TargetPort::ComponentInput {
                        component_idx,
                        port_idx,
                    } => (0, component_idx, port_idx),
                };
                let (type_b, idx_b, port_b) = match b.target {
                    TargetPort::ChipOutput(i) => (1, i, 0),
                    TargetPort::ComponentInput {
                        component_idx,
                        port_idx,
                    } => (0, component_idx, port_idx),
                };

                type_a
                    .cmp(&type_b)
                    .then(idx_a.cmp(&idx_b))
                    .then(port_a.cmp(&port_b))
            });

            if let Some(idx) = sharing.iter().position(|c| c.target == conn.target) {
                let n = sharing.len() as f32;
                (idx as f32 - (n - 1.0) / 2.0) * 8.0
            } else {
                0.0
            }
        };

        use crate::engine::TargetPort;
        let (tgt_comp_id, tgt_port) = match conn.target {
            TargetPort::ChipOutput(i) => (8888, i),
            TargetPort::ComponentInput {
                component_idx,
                port_idx,
            } => (component_idx, port_idx),
        };
        let hash = (tgt_comp_id + tgt_port) % 3;
        let hash_offset = (hash as f32 - 1.0) * 4.0;

        fanout_offset + hash_offset
    }

    /// Compute all orthogonal Manhattan wire segments for a given connection, in screen space.
    pub fn compute_wire_segments_screen(
        src_pos: Vec2,
        tgt_pos: Vec2,
        routing_offset: f32,
        tgt_port: usize,
        zoom: f32,
    ) -> Vec<(Vec2, Vec2)> {
        let mut segments = Vec::with_capacity(5);
        if tgt_pos.x >= src_pos.x + 20.0 * zoom {
            let offset = routing_offset * zoom;
            let mut mid_x = src_pos.x + (tgt_pos.x - src_pos.x) / 2.0 + offset;
            let margin = (10.0 * zoom).min((tgt_pos.x - src_pos.x) * 0.45);
            mid_x = mid_x.clamp(src_pos.x + margin, tgt_pos.x - margin);

            segments.push((Vec2::new(src_pos.x, src_pos.y), Vec2::new(mid_x, src_pos.y)));
            if (tgt_pos.y - src_pos.y).abs() > 0.5 {
                segments.push((Vec2::new(mid_x, src_pos.y), Vec2::new(mid_x, tgt_pos.y)));
            }
            segments.push((Vec2::new(mid_x, tgt_pos.y), Vec2::new(tgt_pos.x, tgt_pos.y)));
        } else {
            let offset_x = routing_offset.abs() * 0.7 * zoom;
            let offset_y = routing_offset * zoom;
            let target_stagger = (tgt_port as f32 * 6.0) * zoom;

            let stub_src = src_pos.x + 20.0 * zoom + offset_x;
            let stub_tgt = tgt_pos.x - 20.0 * zoom - offset_x - target_stagger;

            let mut mid_y = src_pos.y + (tgt_pos.y - src_pos.y) / 2.0 + offset_y;
            if (tgt_pos.y - src_pos.y).abs() < 10.0 * zoom {
                mid_y += 35.0 * zoom;
            }

            segments.push((
                Vec2::new(src_pos.x, src_pos.y),
                Vec2::new(stub_src, src_pos.y),
            ));
            segments.push((Vec2::new(stub_src, src_pos.y), Vec2::new(stub_src, mid_y)));
            segments.push((Vec2::new(stub_src, mid_y), Vec2::new(stub_tgt, mid_y)));
            segments.push((Vec2::new(stub_tgt, mid_y), Vec2::new(stub_tgt, tgt_pos.y)));
            segments.push((
                Vec2::new(stub_tgt, tgt_pos.y),
                Vec2::new(tgt_pos.x, tgt_pos.y),
            ));
        }
        segments
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

    /// Find all wire intersections (T-junctions and orthogonal crossings) across visible connections.
    pub fn find_wire_intersections(
        &self,
        visible_wires: &[&VisualConnection],
    ) -> Vec<WireIntersection> {
        let mut all_segments = Vec::new();
        let mut seg_id_counter = 0;

        let sw = self.screen_width();
        let sh = self.screen_height();
        let screen_rect = Rect::new(-20.0, -20.0, sw + 40.0, sh + 40.0);

        for (conn_idx, &wire) in visible_wires.iter().enumerate() {
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
                    if a.distance(b) < 1.0 {
                        continue;
                    }
                    let min_x = a.x.min(b.x);
                    let max_x = a.x.max(b.x);
                    let min_y = a.y.min(b.y);
                    let max_y = a.y.max(b.y);

                    // Frustum Culling
                    if min_x > screen_rect.right()
                        || max_x < screen_rect.left()
                        || min_y > screen_rect.bottom()
                        || max_y < screen_rect.top()
                    {
                        continue;
                    }

                    all_segments.push(IdentifiedSegment {
                        a,
                        b,
                        conn_idx,
                        seg_id: seg_id_counter,
                    });
                    seg_id_counter += 1;
                }
            }
        }

        // Spatial Grid for O(N) intersection search
        let cell_size = (50.0 * self.canvas.zoom).max(4.0);
        let mut grid: std::collections::HashMap<(i32, i32), Vec<IdentifiedSegment>> =
            std::collections::HashMap::new();

        for seg in &all_segments {
            let min_x = seg.a.x.min(seg.b.x);
            let max_x = seg.a.x.max(seg.b.x);
            let min_y = seg.a.y.min(seg.b.y);
            let max_y = seg.a.y.max(seg.b.y);

            let start_col = (min_x / cell_size).floor() as i32;
            let end_col = (max_x / cell_size).floor() as i32;
            let start_row = (min_y / cell_size).floor() as i32;
            let end_row = (max_y / cell_size).floor() as i32;

            for col in start_col..=end_col {
                for row in start_row..=end_row {
                    grid.entry((col, row)).or_default().push(seg.clone());
                }
            }
        }

        let mut intersections = Vec::new();
        let mut seen_crossings: std::collections::HashSet<(i32, i32)> =
            std::collections::HashSet::new();
        let mut seen_junctions: std::collections::HashSet<(i32, i32)> =
            std::collections::HashSet::new();
        let mut checked_pairs: std::collections::HashSet<(usize, usize)> =
            std::collections::HashSet::new();

        let zoom = self.canvas.zoom;
        let bucket_eps = (3.0 * zoom).max(1.0);
        let crossing_margin = (5.0 * zoom).max(2.0);
        let junction_tol = (2.0 * zoom).max(0.75);

        for cell in grid.values() {
            for i in 0..cell.len() {
                let s1 = &cell[i];
                for s2 in &cell[(i + 1)..] {
                    if s1.conn_idx == s2.conn_idx {
                        continue;
                    }

                    let pair = if s1.seg_id < s2.seg_id {
                        (s1.seg_id, s2.seg_id)
                    } else {
                        (s2.seg_id, s1.seg_id)
                    };
                    if !checked_pairs.insert(pair) {
                        continue;
                    }

                    let seg_i_horizontal = is_horizontal(s1.a, s1.b);
                    let seg_j_horizontal = is_horizontal(s2.a, s2.b);

                    // Only perpendicular (one horizontal, one vertical) segments can form a crossing or T-junction
                    if seg_i_horizontal == seg_j_horizontal {
                        continue;
                    }

                    let (h_seg, v_seg) = if seg_i_horizontal { (s1, s2) } else { (s2, s1) };

                    let h_min_x = h_seg.a.x.min(h_seg.b.x);
                    let h_max_x = h_seg.a.x.max(h_seg.b.x);
                    let h_y = (h_seg.a.y + h_seg.b.y) * 0.5;

                    let v_min_y = v_seg.a.y.min(v_seg.b.y);
                    let v_max_y = v_seg.a.y.max(v_seg.b.y);
                    let v_x = (v_seg.a.x + v_seg.b.x) * 0.5;

                    let point = Vec2::new(v_x, h_y);

                    let conn_i = visible_wires[s1.conn_idx];
                    let conn_j = visible_wires[s2.conn_idx];

                    if wires_share_endpoint(conn_i, conn_j) {
                        // T-junction detection: one segment's coordinate lies strictly inside the perpendicular segment
                        // while the other coordinate touches within tolerance.
                        let x_on_h = v_x >= h_min_x - junction_tol && v_x <= h_max_x + junction_tol;
                        let y_on_v = h_y >= v_min_y - junction_tol && h_y <= v_max_y + junction_tol;
                        let x_strictly_inside_h =
                            v_x > h_min_x + crossing_margin && v_x < h_max_x - crossing_margin;
                        let y_strictly_inside_v =
                            h_y > v_min_y + crossing_margin && h_y < v_max_y - crossing_margin;

                        if x_on_h && y_on_v && (x_strictly_inside_h || y_strictly_inside_v) {
                            let bx = (point.x / bucket_eps).round() as i32;
                            let by = (point.y / bucket_eps).round() as i32;
                            if seen_junctions.insert((bx, by)) {
                                let (color, thickness) = self.get_connection_style(conn_i);
                                intersections.push(WireIntersection {
                                    point,
                                    junction_type: JunctionType::Connected,
                                    upper_horizontal: true,
                                    upper_conn: None,
                                    lower_color: color,
                                    lower_thickness: thickness,
                                    upper_color: color,
                                    upper_thickness: thickness,
                                });
                            }
                        }
                        continue;
                    }

                    // Strict interior orthogonal crossing between two unrelated electrical nets
                    if v_x > h_min_x + crossing_margin
                        && v_x < h_max_x - crossing_margin
                        && h_y > v_min_y + crossing_margin
                        && h_y < v_max_y - crossing_margin
                    {
                        let bx = (point.x / bucket_eps).round() as i32;
                        let by = (point.y / bucket_eps).round() as i32;
                        let mut already_seen = false;
                        for dx in -1..=1 {
                            for dy in -1..=1 {
                                if seen_crossings.contains(&(bx + dx, by + dy)) {
                                    already_seen = true;
                                    break;
                                }
                            }
                        }
                        if already_seen {
                            continue;
                        }
                        seen_crossings.insert((bx, by));

                        // The horizontal wire always bridges over the vertical wire
                        let lower_conn = visible_wires[v_seg.conn_idx];
                        let (lower_color, lower_thickness) = self.get_connection_style(lower_conn);

                        let upper_conn = visible_wires[h_seg.conn_idx];
                        let (upper_color, upper_thickness) = self.get_connection_style(upper_conn);

                        intersections.push(WireIntersection {
                            point,
                            junction_type: JunctionType::Crossing,
                            upper_horizontal: true,
                            upper_conn: Some(*upper_conn),
                            lower_color,
                            lower_thickness,
                            upper_color,
                            upper_thickness,
                        });
                    }
                }
            }
        }

        intersections
    }

    pub fn draw_wire_junctions(&self, painter: &egui::Painter, intersections: &[WireIntersection]) {
        for int in intersections {
            if int.junction_type == JunctionType::Connected {
                let radius = (int.upper_thickness * 1.45).max(3.2 * self.canvas.zoom);
                draw_circle_filled(painter, int.point.x, int.point.y, radius, int.upper_color);
            }
        }
    }
}

fn is_horizontal(a: Vec2, b: Vec2) -> bool {
    (a.y - b.y).abs() < (a.x - b.x).abs()
}

fn wires_share_endpoint(a: &VisualConnection, b: &VisualConnection) -> bool {
    (a.src_comp_id == b.src_comp_id && a.src_port == b.src_port)
        || (a.tgt_comp_id == b.tgt_comp_id && a.tgt_port == b.tgt_port)
        || (a.src_comp_id == b.tgt_comp_id && a.src_port == b.tgt_port)
        || (a.tgt_comp_id == b.src_comp_id && a.tgt_port == b.src_port)
}
