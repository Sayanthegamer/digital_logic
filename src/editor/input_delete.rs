use super::Editor;

impl Editor {
    pub fn handle_canvas_deletion(&mut self) {
        if self.canvas.selected_annotation_idx.is_some()
            || !self.canvas.selected_comp_ids.is_empty()
            || !self.canvas.selected_connections.is_empty()
            || self.canvas.selected_comp_id.is_some()
        {
            self.push_history_snapshot();
        }
        if let Some(idx) = self.canvas.selected_annotation_idx {
            if idx < self.circuit.annotations.len() {
                self.circuit.annotations.remove(idx);
            }
            self.canvas.selected_annotation_idx = None;
        } else if !self.canvas.selected_comp_ids.is_empty()
            || !self.canvas.selected_connections.is_empty()
        {
            self.circuit.components
                .retain(|c| !self.canvas.selected_comp_ids.contains(&c.id));
            self.circuit.connections.retain(|c| {
                !self.canvas.selected_comp_ids.contains(&c.src_comp_id)
                    && !self.canvas.selected_comp_ids.contains(&c.tgt_comp_id)
                    && !self.canvas.selected_connections.contains(c)
            });
            self.canvas.selected_comp_ids.clear();
            self.canvas.selected_connections.clear();
            self.canvas.selected_comp_id = None;
            self.compile();
        } else if let Some(id) = self.canvas.selected_comp_id {
            self.circuit.components.retain(|c| c.id != id);
            self.circuit.connections
                .retain(|c| c.src_comp_id != id && c.tgt_comp_id != id);
            self.canvas.selected_comp_id = None;
            self.compile();
        }
    }
}
