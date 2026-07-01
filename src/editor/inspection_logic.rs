use crate::engine::{
    ChipBlueprint, Component, ComponentType, OutputSource, SourcePort, TargetPort, TraceNode,
};

use super::Editor;

impl Editor {
    pub fn get_inspected_blueprint_and_components(
        &self,
    ) -> Option<(&ChipBlueprint, Vec<Component>)> {
        if self.inspection_path.is_empty() {
            return None;
        }

        let bp_idx = self.get_blueprint_idx_for_path(&self.inspection_path)?;
        let blueprint = &self.library[bp_idx];
        Some((blueprint, blueprint.components.clone()))
    }

    pub fn get_blueprint_idx_for_path(&self, path: &[usize]) -> Option<usize> {
        if path.is_empty() {
            return None;
        }
        let first_comp_id = path[0];
        let curr_comp = self.components.iter().find(|c| c.id == first_comp_id)?;
        let mut curr_bp_idx = match curr_comp.comp_type {
            ComponentType::SubChip(idx) => idx,
            _ => return None,
        };

        for &comp_idx in path.iter().skip(1) {
            let blueprint = &self.library[curr_bp_idx];
            if comp_idx < blueprint.components.len() {
                let next_comp = &blueprint.components[comp_idx];
                curr_bp_idx = match next_comp.component_type {
                    ComponentType::SubChip(idx) => idx,
                    _ => return None,
                };
            } else {
                return None;
            }
        }
        Some(curr_bp_idx)
    }

    pub fn get_node_state_at_path(&self, node: &TraceNode, path: &[usize]) -> bool {
        if path.is_empty() {
            match node {
                TraceNode::ChipInput(idx) => {
                    let inputs: Vec<&super::types::VisualComponent> = self
                        .components
                        .iter()
                        .filter(|c| c.comp_type == ComponentType::Input)
                        .collect();
                    if let Some(comp) = inputs.get(*idx)
                        && let Some(&g_idx) = self.visual_to_sim_map.get(&comp.id)
                    {
                        return self.simulator.get_state(g_idx);
                    }
                }
                TraceNode::CompOutput {
                    component_idx,
                    port_idx,
                } => {
                    if let Some(&g_idx) =
                        self.port_to_sim_gate_map.get(&(*component_idx, *port_idx))
                    {
                        return self.simulator.get_state(g_idx);
                    }
                }
                TraceNode::CompInput {
                    component_idx,
                    port_idx,
                } => {
                    if let Some(conn) = self
                        .connections
                        .iter()
                        .find(|c| c.tgt_comp_id == *component_idx && c.tgt_port == *port_idx)
                    {
                        let src_node = TraceNode::CompOutput {
                            component_idx: conn.src_comp_id,
                            port_idx: conn.src_port,
                        };
                        return self.get_node_state_at_path(&src_node, &[]);
                    }
                }
                _ => {}
            }
            return false;
        }

        let parent_path = &path[..path.len() - 1];
        let comp_id_in_parent = path[path.len() - 1];

        if let Some(bp_idx) = self.get_blueprint_idx_for_path(path) {
            let blueprint = &self.library[bp_idx];
            let driver = self.trace_local_driver(node, blueprint, path);

            match driver {
                OutputSource::DrivenByGate(g_idx) => self.simulator.get_state(g_idx),
                OutputSource::PassedThrough(in_idx) => {
                    let parent_node = TraceNode::CompInput {
                        component_idx: comp_id_in_parent,
                        port_idx: in_idx,
                    };
                    self.get_node_state_at_path(&parent_node, parent_path)
                }
                OutputSource::Floating => false,
            }
        } else {
            false
        }
    }

    pub(crate) fn trace_local_driver(
        &self,
        node: &TraceNode,
        blueprint: &ChipBlueprint,
        path: &[usize],
    ) -> OutputSource {
        match node {
            TraceNode::CompOutput {
                component_idx,
                port_idx,
            } => {
                let component = &blueprint.components[*component_idx];
                match component.component_type {
                    ComponentType::Nand | ComponentType::Clock => {
                        if let Some(&g_idx) = self
                            .instance_to_sim_map
                            .get(&(path.to_vec(), *component_idx))
                        {
                            OutputSource::DrivenByGate(g_idx)
                        } else {
                            OutputSource::Floating
                        }
                    }
                    ComponentType::SubChip(_) => {
                        if let Some(outputs) =
                            self.instance_outputs.get(&(path.to_vec(), *component_idx))
                        {
                            if *port_idx < outputs.len() {
                                outputs[*port_idx]
                            } else {
                                OutputSource::Floating
                            }
                        } else {
                            OutputSource::Floating
                        }
                    }
                    ComponentType::Input | ComponentType::Output | ComponentType::SevenSegment => OutputSource::Floating,
                }
            }
            TraceNode::CompInput {
                component_idx,
                port_idx,
            } => {
                let conn = blueprint.connections.iter().find(|c| {
                    c.target
                        == TargetPort::ComponentInput {
                            component_idx: *component_idx,
                            port_idx: *port_idx,
                        }
                });

                if let Some(c) = conn {
                    match c.source {
                        SourcePort::ChipInput(i) => OutputSource::PassedThrough(i),
                        SourcePort::ComponentOutput {
                            component_idx: src_c,
                            port_idx: src_p,
                        } => self.trace_local_driver(
                            &TraceNode::CompOutput {
                                component_idx: src_c,
                                port_idx: src_p,
                            },
                            blueprint,
                            path,
                        ),
                    }
                } else {
                    OutputSource::Floating
                }
            }
            TraceNode::ChipOutput(out_idx) => {
                let conn = blueprint
                    .connections
                    .iter()
                    .find(|c| c.target == TargetPort::ChipOutput(*out_idx));

                if let Some(c) = conn {
                    match c.source {
                        SourcePort::ChipInput(i) => OutputSource::PassedThrough(i),
                        SourcePort::ComponentOutput {
                            component_idx: src_c,
                            port_idx: src_p,
                        } => self.trace_local_driver(
                            &TraceNode::CompOutput {
                                component_idx: src_c,
                                port_idx: src_p,
                            },
                            blueprint,
                            path,
                        ),
                    }
                } else {
                    OutputSource::Floating
                }
            }
            TraceNode::ChipInput(idx) => OutputSource::PassedThrough(*idx),
        }
    }
}
