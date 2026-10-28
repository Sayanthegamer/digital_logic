use crate::editor::math::Vec2;
use crate::editor::types::{VisualComponent, VisualConnection};
use std::collections::{HashMap, VecDeque};

struct LayoutNode {
    comp_idx: Option<usize>, // None if dummy node
    layer: usize,
    width: f32,
    height: f32,
    /// `(parent_node_idx, src_port, tgt_port)`
    parents: Vec<(usize, usize, usize)>,
    /// `(child_node_idx, src_port, tgt_port)`
    children: Vec<(usize, usize, usize)>,
}

#[allow(clippy::needless_range_loop)]
pub fn auto_arrange(components: &mut [VisualComponent], connections: &[VisualConnection]) {
    if components.is_empty() {
        return;
    }

    let mut id_to_idx = HashMap::new();
    for (i, comp) in components.iter().enumerate() {
        id_to_idx.insert(comp.id, i);
    }

    // 1. Build Adjacency List preserving port indices (src_port, tgt_port)
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![vec![]; components.len()];
    for conn in connections {
        if let (Some(&src_idx), Some(&tgt_idx)) = (
            id_to_idx.get(&conn.src_comp_id),
            id_to_idx.get(&conn.tgt_comp_id),
        ) && src_idx != tgt_idx
            && !adj[src_idx]
                .iter()
                .any(|&(t, sp, tp)| t == tgt_idx && sp == conn.src_port && tp == conn.tgt_port)
        {
            adj[src_idx].push((tgt_idx, conn.src_port, conn.tgt_port));
        }
    }

    // 2. Cycle Removal (DFS) -> DAG
    let mut visited = vec![false; components.len()];
    let mut rec_stack = vec![false; components.len()];
    let mut dag_adj: Vec<Vec<(usize, usize, usize)>> = vec![vec![]; components.len()];

    for i in 0..components.len() {
        if !visited[i] {
            dfs_iterative(i, &adj, &mut visited, &mut rec_stack, &mut dag_adj);
        }
    }

    fn dfs_iterative(
        start: usize,
        adj: &[Vec<(usize, usize, usize)>],
        visited: &mut [bool],
        rec_stack: &mut [bool],
        dag_adj: &mut [Vec<(usize, usize, usize)>],
    ) {
        let mut stack = vec![(start, 0)];
        visited[start] = true;
        rec_stack[start] = true;

        while let Some((u, i)) = stack.pop() {
            if i < adj[u].len() {
                let (v, sp, tp) = adj[u][i];
                stack.push((u, i + 1));

                if !visited[v] {
                    visited[v] = true;
                    rec_stack[v] = true;
                    dag_adj[u].push((v, sp, tp));
                    stack.push((v, 0));
                } else if !rec_stack[v] {
                    dag_adj[u].push((v, sp, tp));
                }
            } else {
                rec_stack[u] = false;
            }
        }
    }

    // 3. Kahn's Algorithm for Longest-Path Layer Assignment on DAG
    let mut in_degree = vec![0; components.len()];
    for u in 0..components.len() {
        for &(v, _, _) in &dag_adj[u] {
            in_degree[v] += 1;
        }
    }

    let mut layer = vec![0; components.len()];
    let mut queue = VecDeque::new();

    for (i, &deg) in in_degree.iter().enumerate() {
        if deg == 0 {
            queue.push_back(i);
        }
    }

    while let Some(u) = queue.pop_front() {
        for &(v, _, _) in &dag_adj[u] {
            layer[v] = layer[v].max(layer[u] + 1);
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    // 4. Augmented Graph with Dummy Nodes for multi-layer edges
    let mut nodes = Vec::new();

    for i in 0..components.len() {
        nodes.push(LayoutNode {
            comp_idx: Some(i),
            layer: layer[i],
            width: components[i].width,
            height: components[i].height,
            parents: Vec::new(),
            children: Vec::new(),
        });
    }

    for u in 0..components.len() {
        for &(v, sp, tp) in &dag_adj[u] {
            let lu = layer[u];
            let lv = layer[v];
            if lv > lu + 1 {
                let mut prev = u;
                let mut prev_sp = sp;
                for l in (lu + 1)..lv {
                    let dummy_idx = nodes.len();
                    nodes.push(LayoutNode {
                        comp_idx: None,
                        layer: l,
                        width: 10.0,
                        height: 24.0,
                        parents: vec![(prev, prev_sp, 0)],
                        children: Vec::new(),
                    });
                    nodes[prev].children.push((dummy_idx, prev_sp, 0));
                    prev = dummy_idx;
                    prev_sp = 0;
                }
                nodes[prev].children.push((v, prev_sp, tp));
                nodes[v].parents.push((prev, prev_sp, tp));
            } else {
                nodes[u].children.push((v, sp, tp));
                nodes[v].parents.push((u, sp, tp));
            }
        }
    }

    let max_layer = *layer.iter().max().unwrap_or(&0);
    let mut layers = vec![vec![]; max_layer + 1];
    for (i, node) in nodes.iter().enumerate() {
        layers[node.layer].push(i);
    }

    // Preserve initial stable order by existing Y coordinate or component ID
    for l in 0..=max_layer {
        layers[l].sort_by(|&a, &b| {
            let ya = nodes[a]
                .comp_idx
                .map(|idx| components[idx].pos.y)
                .unwrap_or(0.0);
            let yb = nodes[b]
                .comp_idx
                .map(|idx| components[idx].pos.y)
                .unwrap_or(0.0);
            ya.partial_cmp(&yb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.cmp(&b))
        });
    }

    // 5. Port-Aware Multi-Sweep Crossing Minimization
    let mut node_pos = vec![0; nodes.len()];
    for l in 0..=max_layer {
        for (i, &node) in layers[l].iter().enumerate() {
            node_pos[node] = i;
        }
    }

    for _sweep in 0..4 {
        // Forward sweep (left-to-right): include parent rank + source/target port tie-breaking
        for l in 1..=max_layer {
            let mut barycenters = vec![0.0; nodes.len()];
            for &node in &layers[l] {
                if nodes[node].parents.is_empty() {
                    barycenters[node] = node_pos[node] as f32;
                } else {
                    let mut sum = 0.0;
                    for &(parent, sp, tp) in &nodes[node].parents {
                        sum += node_pos[parent] as f32 + (sp as f32 * 0.04) + (tp as f32 * 0.01);
                    }
                    barycenters[node] = sum / (nodes[node].parents.len() as f32);
                }
            }
            layers[l].sort_by(|&a, &b| {
                barycenters[a]
                    .partial_cmp(&barycenters[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.cmp(&b))
            });
            for (i, &node) in layers[l].iter().enumerate() {
                node_pos[node] = i;
            }
        }

        // Backward sweep (right-to-left): order inputs by child rank + child's target port index!
        for l in (0..max_layer).rev() {
            let mut barycenters = vec![0.0; nodes.len()];
            for &node in &layers[l] {
                if nodes[node].children.is_empty() {
                    barycenters[node] = node_pos[node] as f32;
                } else {
                    let mut sum = 0.0;
                    for &(child, sp, tp) in &nodes[node].children {
                        sum += node_pos[child] as f32 + (tp as f32 * 0.05) + (sp as f32 * 0.01);
                    }
                    barycenters[node] = sum / (nodes[node].children.len() as f32);
                }
            }
            layers[l].sort_by(|&a, &b| {
                barycenters[a]
                    .partial_cmp(&barycenters[b])
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.cmp(&b))
            });
            for (i, &node) in layers[l].iter().enumerate() {
                node_pos[node] = i;
            }
        }
    }

    // 6. Center-Aligned Y-Coordinate Assignment
    // Step 6a: Stack each layer compactly starting from y = 0.0 and record vertical centers
    let mut center_y = vec![0.0_f32; nodes.len()];

    let pair_gap = |n_above: &LayoutNode, n_below: &LayoutNode| -> f32 {
        if n_above.height <= 60.0 && n_below.height <= 60.0 {
            24.0
        } else {
            44.0
        }
    };

    for l in 0..=max_layer {
        let mut cursor_y = 0.0_f32;
        for (idx_in_layer, &node_idx) in layers[l].iter().enumerate() {
            if idx_in_layer > 0 {
                let prev_idx = layers[l][idx_in_layer - 1];
                cursor_y += pair_gap(&nodes[prev_idx], &nodes[node_idx]);
            }
            center_y[node_idx] = cursor_y + nodes[node_idx].height * 0.5;
            cursor_y += nodes[node_idx].height;
        }
    }

    // Helper: relax a layer's vertical positions toward desired target centers while preserving non-overlap
    let relax_layer = |layer_nodes: &[usize],
                       desired_centers: &[Option<f32>],
                       center_y: &mut [f32],
                       nodes: &[LayoutNode]| {
        if layer_nodes.is_empty() {
            return;
        }
        // Place each node at its desired center (if any), enforcing downward minimum separation
        for (i, &node_idx) in layer_nodes.iter().enumerate() {
            if let Some(target_c) = desired_centers[i] {
                center_y[node_idx] = target_c;
            }
            if i > 0 {
                let prev_idx = layer_nodes[i - 1];
                let min_c = center_y[prev_idx]
                    + (nodes[prev_idx].height + nodes[node_idx].height) * 0.5
                    + pair_gap(&nodes[prev_idx], &nodes[node_idx]);
                if center_y[node_idx] < min_c {
                    center_y[node_idx] = min_c;
                }
            }
        }
        // Recenter the layer as a whole around the average desired center so downward push doesn't bias downward
        let mut diff_sum = 0.0_f32;
        let mut diff_count = 0usize;
        for (i, &node_idx) in layer_nodes.iter().enumerate() {
            if let Some(target_c) = desired_centers[i] {
                diff_sum += target_c - center_y[node_idx];
                diff_count += 1;
            }
        }
        if diff_count > 0 {
            let shift = diff_sum / (diff_count as f32);
            for &node_idx in layer_nodes {
                center_y[node_idx] += shift;
            }
        }
    };

    // Step 6b: Forward + Backward vertical center alignment sweeps
    for _iter in 0..3 {
        // Forward pass: align children's vertical centers to the average vertical center of their parents
        for l in 1..=max_layer {
            let desired: Vec<Option<f32>> = layers[l]
                .iter()
                .map(|&node_idx| {
                    if nodes[node_idx].parents.is_empty() {
                        None
                    } else {
                        let sum: f32 = nodes[node_idx]
                            .parents
                            .iter()
                            .map(|&(p, _, _)| center_y[p])
                            .sum();
                        Some(sum / (nodes[node_idx].parents.len() as f32))
                    }
                })
                .collect();
            relax_layer(&layers[l], &desired, &mut center_y, &nodes);
        }

        // Backward pass: align parents' vertical centers to the average vertical center of their children
        for l in (0..max_layer).rev() {
            let desired: Vec<Option<f32>> = layers[l]
                .iter()
                .map(|&node_idx| {
                    if nodes[node_idx].children.is_empty() {
                        None
                    } else {
                        let sum: f32 = nodes[node_idx]
                            .children
                            .iter()
                            .map(|&(c, _, _)| center_y[c])
                            .sum();
                        Some(sum / (nodes[node_idx].children.len() as f32))
                    }
                })
                .collect();
            relax_layer(&layers[l], &desired, &mut center_y, &nodes);
        }
    }

    // Step 6c: Shift entire layout so global top edge starts cleanly at y = 100.0
    let min_top_y = nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.comp_idx.is_some())
        .map(|(idx, n)| center_y[idx] - n.height * 0.5)
        .fold(f32::INFINITY, f32::min);
    let y_offset = if min_top_y.is_finite() {
        100.0 - min_top_y
    } else {
        100.0
    };

    // Step 6d: Assign X and Y coordinates snapped to 20px grid while preventing any post-snap overlap
    let x_gap: f32 = 180.0;
    let mut current_x: f32 = 100.0;

    for l in 0..=max_layer {
        let mut layer_max_width: f32 = 0.0;
        let mut prev_bottom: Option<f32> = None;

        for &idx in &layers[l] {
            layer_max_width = layer_max_width.max(nodes[idx].width);

            if let Some(comp_idx) = nodes[idx].comp_idx {
                let raw_x = current_x;
                let raw_y = center_y[idx] - nodes[idx].height * 0.5 + y_offset;

                let final_x = (raw_x / 20.0_f32).round() * 20.0_f32;
                let mut final_y = (raw_y / 20.0_f32).round() * 20.0_f32;

                if let Some(pb) = prev_bottom {
                    let min_y = ((pb + 20.0) / 20.0_f32).ceil() * 20.0_f32;
                    if final_y < min_y {
                        final_y = min_y;
                    }
                }

                components[comp_idx].pos = Vec2::new(final_x, final_y);
                prev_bottom = Some(final_y + components[comp_idx].height);
            }
        }

        current_x += layer_max_width + x_gap;
    }
}
