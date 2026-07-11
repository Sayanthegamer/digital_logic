use crate::editor::types::{VisualComponent, VisualConnection};
use macroquad::prelude::Vec2;
use std::collections::{HashMap, VecDeque};

struct LayoutNode {
    comp_idx: Option<usize>, // None if dummy node
    layer: usize,
    width: f32,
    height: f32,
    parents: Vec<usize>,
    children: Vec<usize>,
}

pub fn auto_arrange(components: &mut [VisualComponent], connections: &[VisualConnection]) {
    if components.is_empty() {
        return;
    }

    let mut id_to_idx = HashMap::new();
    for (i, comp) in components.iter().enumerate() {
        id_to_idx.insert(comp.id, i);
    }

    // 1. Build Adjacency List
    let mut adj = vec![vec![]; components.len()];
    for conn in connections {
        if let (Some(&src_idx), Some(&tgt_idx)) = (
            id_to_idx.get(&conn.src_comp_id),
            id_to_idx.get(&conn.tgt_comp_id),
        ) {
            if !adj[src_idx].contains(&tgt_idx) {
                adj[src_idx].push(tgt_idx);
            }
        }
    }

    // 2. Cycle Removal (DFS) -> DAG
    let mut visited = vec![false; components.len()];
    let mut rec_stack = vec![false; components.len()];
    let mut dag_adj = vec![vec![]; components.len()];

    fn dfs(
        u: usize,
        adj: &[Vec<usize>],
        visited: &mut Vec<bool>,
        rec_stack: &mut Vec<bool>,
        dag_adj: &mut Vec<Vec<usize>>,
    ) {
        visited[u] = true;
        rec_stack[u] = true;

        for &v in &adj[u] {
            if !visited[v] {
                dag_adj[u].push(v);
                dfs(v, adj, visited, rec_stack, dag_adj);
            } else if !rec_stack[v] {
                dag_adj[u].push(v);
            }
        }
        rec_stack[u] = false;
    }

    for i in 0..components.len() {
        if !visited[i] {
            dfs(i, &adj, &mut visited, &mut rec_stack, &mut dag_adj);
        }
    }

    // 3. Layer Assignment (Longest Path)
    let mut in_degree = vec![0; components.len()];
    for u in 0..components.len() {
        for &v in &dag_adj[u] {
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
        for &v in &dag_adj[u] {
            layer[v] = layer[v].max(layer[u] + 1);
            in_degree[v] -= 1;
            if in_degree[v] == 0 {
                queue.push_back(v);
            }
        }
    }

    // 4. Augmented Graph with Dummy Nodes
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
        for &v in &dag_adj[u] {
            let lu = layer[u];
            let lv = layer[v];
            if lv > lu + 1 {
                let mut prev = u;
                for l in (lu + 1)..lv {
                    let dummy_idx = nodes.len();
                    nodes.push(LayoutNode {
                        comp_idx: None,
                        layer: l,
                        width: 10.0,
                        height: 40.0,
                        parents: vec![prev],
                        children: Vec::new(),
                    });
                    nodes[prev].children.push(dummy_idx);
                    prev = dummy_idx;
                }
                nodes[prev].children.push(v);
                nodes[v].parents.push(prev);
            } else {
                nodes[u].children.push(v);
                nodes[v].parents.push(u);
            }
        }
    }

    let max_layer = *layer.iter().max().unwrap_or(&0);
    let mut layers = vec![vec![]; max_layer + 1];
    for (i, node) in nodes.iter().enumerate() {
        layers[node.layer].push(i);
    }

    // 5. Multi-sweep Crossing Minimization
    for _sweep in 0..4 {
        // Forward sweep (left-to-right)
        for l in 1..=max_layer {
            let prev_layer = layers[l - 1].clone();
            layers[l].sort_by(|&a, &b| {
                let barycenter_a = compute_barycenter(a, &nodes, &prev_layer, true);
                let barycenter_b = compute_barycenter(b, &nodes, &prev_layer, true);
                barycenter_a.partial_cmp(&barycenter_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        // Backward sweep (right-to-left)
        for l in (0..max_layer).rev() {
            let next_layer = layers[l + 1].clone();
            layers[l].sort_by(|&a, &b| {
                let barycenter_a = compute_barycenter(a, &nodes, &next_layer, false);
                let barycenter_b = compute_barycenter(b, &nodes, &next_layer, false);
                barycenter_a.partial_cmp(&barycenter_b).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }

    // Final forward sweep to ensure strict left-to-right alignment stability
    for l in 1..=max_layer {
        let prev_layer = layers[l - 1].clone();
        layers[l].sort_by(|&a, &b| {
            let barycenter_a = compute_barycenter(a, &nodes, &prev_layer, true);
            let barycenter_b = compute_barycenter(b, &nodes, &prev_layer, true);
            barycenter_a.partial_cmp(&barycenter_b).unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // 6. Smart Y-Coordinate Assignment
    let x_gap = 200.0;
    let y_gap = 60.0;
    
    let mut y_positions = vec![0.0; nodes.len()];
    let mut current_x = 100.0;

    for l in 0..=max_layer {
        let mut layer_max_width: f32 = 0.0;
        let mut current_y = 100.0; // Minimum Y for this layer
        
        for &idx in &layers[l] {
            layer_max_width = layer_max_width.max(nodes[idx].width);
            
            // Calculate ideal Y based on parents' actual Y positions
            let mut sum_y = 0.0;
            let mut count = 0;
            for &parent in &nodes[idx].parents {
                sum_y += y_positions[parent];
                count += 1;
            }
            
            let mut ideal_y = if count > 0 {
                sum_y / (count as f32)
            } else {
                current_y // No parents, just use the current running Y
            };
            
            // We cannot place it higher than current_y to prevent overlapping with the node above it
            ideal_y = ideal_y.max(current_y);
            
            y_positions[idx] = ideal_y;
            current_y = ideal_y + nodes[idx].height + y_gap;
        }

        // Apply coordinates
        for &idx in &layers[l] {
            if let Some(comp_idx) = nodes[idx].comp_idx {
                let mut final_x = current_x;
                let mut final_y = y_positions[idx];
                
                // Snap to 20px grid for clean editor alignment
                final_x = (final_x / 20.0).round() * 20.0;
                final_y = (final_y / 20.0).round() * 20.0;
                
                components[comp_idx].pos = Vec2::new(final_x, final_y);
            }
        }
        
        current_x += layer_max_width + x_gap;
    }
}

// compute_barycenter uses the positions of neighbors in the adjacent layer
fn compute_barycenter(node: usize, nodes: &[LayoutNode], adj_layer: &[usize], use_parents: bool) -> f32 {
    let mut sum = 0.0;
    let mut count = 0;
    
    let neighbors = if use_parents {
        &nodes[node].parents
    } else {
        &nodes[node].children
    };

    for (i, &adj_node) in adj_layer.iter().enumerate() {
        if neighbors.contains(&adj_node) {
            sum += i as f32;
            count += 1;
        }
    }
    
    if count == 0 {
        0.0 
    } else {
        sum / (count as f32)
    }
}
