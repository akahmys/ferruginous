use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct USTNode {
    pub id: usize,
    pub tag: String,
    pub title: String,
    pub alt_text: Option<String>,
    pub rect: Option<[f32; 4]>, // [x1, y1, x2, y2] in PDF User Space
    pub handle_id: Option<u32>, // raw object handle index in PdfArena
    pub children: Vec<USTNode>,
}

#[derive(Serialize, Deserialize)]
pub struct USTRegistry {
    pub root: Option<USTNode>,
    pub selected_node_id: Option<usize>,
    pub next_node_id: usize,
    pub audit_findings: Vec<(String, String, String, Option<u32>)>, // (checkpoint, severity, message, handle_id)
    pub pending_center_node_id: Option<usize>,
}

impl Default for USTRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DragRelation {
    Above,
    Below,
    AsChild,
}

impl USTRegistry {
    pub fn new() -> Self {
        Self {
            root: None,
            selected_node_id: None,
            next_node_id: 1,
            audit_findings: Vec::new(),
            pending_center_node_id: None,
        }
    }

    pub fn clear(&mut self) {
        self.root = None;
        self.selected_node_id = None;
        self.next_node_id = 1;
        self.audit_findings.clear();
        self.pending_center_node_id = None;
    }

    pub fn find_node_id_by_handle_id(&self, handle_id: u32) -> Option<usize> {
        self.root.as_ref().and_then(|r| Self::find_node_id_by_handle_recursive(r, handle_id))
    }

    fn find_node_id_by_handle_recursive(node: &USTNode, handle_id: u32) -> Option<usize> {
        if node.handle_id == Some(handle_id) {
            return Some(node.id);
        }
        for child in &node.children {
            if let Some(id) = Self::find_node_id_by_handle_recursive(child, handle_id) {
                return Some(id);
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn initialize_mock_tree(&mut self, total_pages: usize) {
        let mut doc_node = USTNode {
            id: 0,
            tag: "Document".to_string(),
            title: "PDF Document Catalog".to_string(),
            alt_text: None,
            rect: None,
            handle_id: None,
            children: Vec::new(),
        };

        let mut next_id = 1;

        for i in 0..total_pages {
            let page_node = USTNode {
                id: next_id,
                tag: "Part".to_string(),
                title: format!("Page {} Section", i + 1),
                alt_text: None,
                rect: None,
                handle_id: None,
                children: vec![
                    USTNode {
                        id: next_id + 1,
                        tag: "H1".to_string(),
                        title: format!("Heading of Page {}", i + 1),
                        alt_text: None,
                        rect: None,
                        handle_id: None,
                        children: Vec::new(),
                    },
                    USTNode {
                        id: next_id + 2,
                        tag: "P".to_string(),
                        title: format!("Paragraph content for page {}", i + 1),
                        alt_text: None,
                        rect: None,
                        handle_id: None,
                        children: Vec::new(),
                    },
                    USTNode {
                        id: next_id + 3,
                        tag: "Figure".to_string(),
                        title: format!("Illustration on page {}", i + 1),
                        alt_text: None,
                        rect: None,
                        handle_id: None,
                        children: Vec::new(),
                    },
                ],
            };
            doc_node.children.push(page_node);
            next_id += 4;
        }

        self.root = Some(doc_node);
        self.next_node_id = next_id;
    }

    pub fn find_rect_by_id(&self, id: usize) -> Option<[f32; 4]> {
        self.root.as_ref().and_then(|r| Self::find_rect_recursive(r, id))
    }

    fn find_rect_recursive(node: &USTNode, id: usize) -> Option<[f32; 4]> {
        if node.id == id {
            return node.rect;
        }
        for child in &node.children {
            if let Some(r) = Self::find_rect_recursive(child, id) {
                return Some(r);
            }
        }
        None
    }

    pub fn remove_node(&mut self, id: usize) -> Option<USTNode> {
        if let Some(ref mut root) = self.root {
            if root.id == id {
                return None;
            }
            return Self::remove_node_recursive(root, id);
        }
        None
    }

    fn remove_node_recursive(node: &mut USTNode, id: usize) -> Option<USTNode> {
        for idx in 0..node.children.len() {
            if node.children[idx].id == id {
                return Some(node.children.remove(idx));
            }
        }
        for child in &mut node.children {
            if let Some(removed) = Self::remove_node_recursive(child, id) {
                return Some(removed);
            }
        }
        None
    }

    pub fn move_node(&mut self, dragged_id: usize, target_id: usize, relation: DragRelation) -> bool {
        if dragged_id == target_id {
            return false;
        }

        if let Some(ref root) = self.root {
            if let Some(dragged_node) = Self::find_node_by_id_recursive(root, dragged_id) {
                if Self::is_descendant(dragged_node, target_id) {
                    return false;
                }
            }
        }

        if let Some(dragged_node) = self.remove_node(dragged_id) {
            if let Some(ref mut root) = self.root {
                if Self::insert_node_recursive(root, target_id, dragged_node, relation).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    fn find_node_by_id_recursive<'a>(current: &'a USTNode, id: usize) -> Option<&'a USTNode> {
        if current.id == id {
            return Some(current);
        }
        for child in &current.children {
            if let Some(found) = Self::find_node_by_id_recursive(child, id) {
                return Some(found);
            }
        }
        None
    }

    pub fn is_descendant(parent: &USTNode, target_id: usize) -> bool {
        if parent.id == target_id {
            return true;
        }
        for child in &parent.children {
            if Self::is_descendant(child, target_id) {
                return true;
            }
        }
        false
    }

    fn insert_node_recursive(
        current: &mut USTNode,
        target_id: usize,
        node_to_insert: USTNode,
        relation: DragRelation,
    ) -> Result<(), USTNode> {
        if relation == DragRelation::AsChild && current.id == target_id {
            current.children.push(node_to_insert);
            return Ok(());
        }

        for idx in 0..current.children.len() {
            if current.children[idx].id == target_id {
                match relation {
                    DragRelation::Above => {
                        current.children.insert(idx, node_to_insert);
                        return Ok(());
                    }
                    DragRelation::Below => {
                        current.children.insert(idx + 1, node_to_insert);
                        return Ok(());
                    }
                    DragRelation::AsChild => {
                        current.children[idx].children.push(node_to_insert);
                        return Ok(());
                    }
                }
            }
        }

        let mut temp = Some(node_to_insert);
        for child in &mut current.children {
            if let Some(n) = temp.take() {
                match Self::insert_node_recursive(child, target_id, n, relation) {
                    Ok(()) => return Ok(()),
                    Err(n) => {
                        temp = Some(n);
                    }
                }
            }
        }

        if let Some(n) = temp {
            Err(n)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone)]
pub struct FigureInfo {
    pub id: usize,
    pub title: String,
    pub alt_text: Option<String>,
    pub handle_id: Option<u32>,
}

fn collect_figures(node: &USTNode, figures: &mut Vec<FigureInfo>) {
    if node.tag == "Figure" {
        figures.push(FigureInfo {
            id: node.id,
            title: node.title.clone(),
            alt_text: node.alt_text.clone(),
            handle_id: node.handle_id,
        });
    }
    for child in &node.children {
        collect_figures(child, figures);
    }
}

fn update_alt_text(node: &mut USTNode, id: usize, new_alt: Option<String>) -> bool {
    if node.id == id {
        node.alt_text = new_alt;
        return true;
    }
    for child in &mut node.children {
        if update_alt_text(child, id, new_alt.clone()) {
            return true;
        }
    }
    false
}

pub struct SidebarPanel {
    pub active_tab: usize, // 0 = Tags, 1 = Matterhorn Audit
    pub alt_text_edit_buffer: String,
}

impl Default for SidebarPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl SidebarPanel {
    pub fn new() -> Self {
        Self {
            active_tab: 0,
            alt_text_edit_buffer: String::new(),
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        ui.vertical(|ui| {
            // Tab Selection Headers
            ui.horizontal(|ui| {
                if ui.selectable_label(self.active_tab == 0, "🏷️ Tags Tree").clicked() {
                    self.active_tab = 0;
                }
                ui.separator();
                let audit_count = registry.audit_findings.len();
                let audit_title = if audit_count > 0 {
                    format!("🚨 Matterhorn ({})", audit_count)
                } else {
                    "✅ Matterhorn".to_string()
                };
                if ui.selectable_label(self.active_tab == 1, audit_title).clicked() {
                    self.active_tab = 1;
                }
            });

            ui.separator();

            // Active Tab Content
            if self.active_tab == 0 {
                self.show_tags_tab(ui, registry, tx_worker);
            } else {
                self.show_audit_tab(ui, registry);
            }
        });
    }

    fn show_tags_tab(
        &mut self,
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        ui.label("Collapse/expand structural tags to explore accessibility relationships:");
        ui.add_space(5.0);

        let mut selected_node_id = registry.selected_node_id;

        egui::ScrollArea::vertical().id_salt("tag_tree_scroll").show(ui, |ui| {
            if let Some(ref mut root) = registry.root {
                Self::render_node_recursive(ui, root, &mut selected_node_id, &mut self.alt_text_edit_buffer, tx_worker);
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label("No structure tree loaded. Please load a valid PDF.");
                });
            }
        });

        // Apply pending moves
        let pending_move: Option<Option<(usize, usize, DragRelation)>> = ui.ctx().data(|d| d.get_temp(egui::Id::new("pending_move")));
        if let Some(Some((drag_id, target_id, relation))) = pending_move {
            registry.move_node(drag_id, target_id, relation);
            ui.ctx().data_mut(|d| {
                d.remove::<Option<(usize, usize, DragRelation)>>(egui::Id::new("pending_move"));
                d.insert_temp::<Option<usize>>(egui::Id::new("dragged_node_id"), None);
            });
        }

        // Clear dragged node ID on release
        if ui.input(|i| i.pointer.any_released()) {
            ui.ctx().data_mut(|d| d.insert_temp::<Option<usize>>(egui::Id::new("dragged_node_id"), None));
        }

        registry.selected_node_id = selected_node_id;

        // Render Alt-Text Studio Gallery carousel at the bottom of the tag explorer
        ui.separator();
        self.show_alt_text_gallery(ui, registry, tx_worker);
    }

    fn render_drag_drop_controls(ui: &mut egui::Ui, node_id: usize, node: &USTNode) {
        // Drag handle
        let handle_resp = ui.add(egui::Label::new("☰").sense(egui::Sense::drag()));
        if handle_resp.drag_started() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("dragged_node_id"), Some(node_id)));
        }

        // Drop zones (render if dragged node is active and valid)
        let dragged_id: Option<Option<usize>> = ui.ctx().data(|d| d.get_temp(egui::Id::new("dragged_node_id")));
        if let Some(Some(drag_id)) = dragged_id {
            if drag_id != node_id && !USTRegistry::is_descendant(node, drag_id) {
                let resp_above = ui.button("⬆️ Above");
                if resp_above.clicked() || (resp_above.hovered() && ui.input(|i| i.pointer.any_released())) {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pending_move"), Some((drag_id, node_id, DragRelation::Above))));
                }
                let resp_child = ui.button("📁 Child");
                if resp_child.clicked() || (resp_child.hovered() && ui.input(|i| i.pointer.any_released())) {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pending_move"), Some((drag_id, node_id, DragRelation::AsChild))));
                }
                let resp_below = ui.button("⬇️ Below");
                if resp_below.clicked() || (resp_below.hovered() && ui.input(|i| i.pointer.any_released())) {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pending_move"), Some((drag_id, node_id, DragRelation::Below))));
                }
            }
        }
    }

    fn render_node_buttons(
        ui: &mut egui::Ui,
        node: &mut USTNode,
        selected_node_id: &mut Option<usize>,
        alt_edit_buf: &mut String,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        // Alt-Text Studio drawer trigger
        if ui.button("✏️ Alt").clicked() {
            *selected_node_id = Some(node.id);
            *alt_edit_buf = node.alt_text.clone().unwrap_or_default();
        }

        // Quick tag cycle
        if ui.button("🔁 Tag").clicked() {
            node.tag = match node.tag.as_str() {
                "H1" => "H2".to_string(),
                "H2" => "P".to_string(),
                "P" => "H1".to_string(),
                _ => "P".to_string(),
            };
            if let Some(h_id) = node.handle_id {
                let _ = tx_worker.send(crate::worker::WorkerRequest::UpdateNode {
                    handle_id: h_id,
                    tag: node.tag.clone(),
                    alt_text: node.alt_text.clone(),
                });
            }
        }
    }

    fn render_node_drawer(
        ui: &mut egui::Ui,
        node: &mut USTNode,
        is_selected: bool,
        selected_node_id: &mut Option<usize>,
        alt_edit_buf: &mut String,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        if is_selected {
            ui.indent(node.id, |ui| {
                ui.group(|ui| {
                    ui.label("📝 Alt-Text Studio (Remediation)");
                    ui.horizontal(|ui| {
                        ui.text_edit_singleline(alt_edit_buf);
                        if ui.button("Save").clicked() {
                            node.alt_text = if alt_edit_buf.trim().is_empty() {
                                None
                            } else {
                                Some(alt_edit_buf.clone())
                            };
                            if let Some(h_id) = node.handle_id {
                                let _ = tx_worker.send(crate::worker::WorkerRequest::UpdateNode {
                                    handle_id: h_id,
                                    tag: node.tag.clone(),
                                    alt_text: node.alt_text.clone(),
                                });
                            }
                            *selected_node_id = None; // close drawer
                        }
                    });
                });
            });
        }
    }

    fn render_node_recursive(
        ui: &mut egui::Ui,
        node: &mut USTNode,
        selected_node_id: &mut Option<usize>,
        alt_edit_buf: &mut String,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        let is_selected = *selected_node_id == Some(node.id);
        let header_label = format!("<{}>  {}", node.tag, node.title);

        ui.horizontal(|ui| {
            Self::render_drag_drop_controls(ui, node.id, node);

            // Collapsing tree header
            let _response = ui.collapsing(&header_label, |ui| {
                let children_len = node.children.len();
                for idx in 0..children_len {
                    let child = &mut node.children[idx];
                    ui.indent(child.id, |ui| {
                        Self::render_node_recursive(ui, child, selected_node_id, alt_edit_buf, tx_worker);
                    });
                }
            });

            Self::render_node_buttons(ui, node, selected_node_id, alt_edit_buf, tx_worker);
        });

        Self::render_node_drawer(ui, node, is_selected, selected_node_id, alt_edit_buf, tx_worker);
    }

    fn show_alt_text_gallery(
        &mut self,
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        ui.group(|ui| {
            ui.heading("🎨 Alt-Text Studio Gallery");
            ui.add_space(5.0);

            let mut figures = Vec::new();
            if let Some(ref root) = registry.root {
                collect_figures(root, &mut figures);
            }

            if figures.is_empty() {
                ui.label("No figures found in the structure tree.");
            } else {
                egui::ScrollArea::horizontal().id_salt("figure_gallery_carousel").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for fig in &figures {
                            ui.vertical(|ui| {
                                ui.set_min_width(180.0);
                                ui.group(|ui| {
                                    ui.colored_label(egui::Color32::LIGHT_BLUE, format!("🖼️ {}", fig.title));
                                    
                                    let mut buf = fig.alt_text.clone().unwrap_or_default();
                                    ui.label("Alt Text:");
                                    let response = ui.add(egui::TextEdit::singleline(&mut buf).hint_text("Add description..."));
                                    
                                    if response.changed() {
                                        let new_alt = if buf.trim().is_empty() { None } else { Some(buf.clone()) };
                                        if let Some(ref mut root) = registry.root {
                                            if update_alt_text(root, fig.id, new_alt.clone()) {
                                                if let Some(h_id) = fig.handle_id {
                                                    let _ = tx_worker.send(crate::worker::WorkerRequest::UpdateNode {
                                                        handle_id: h_id,
                                                        tag: "Figure".to_string(),
                                                        alt_text: new_alt,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                });
                            });
                            ui.add_space(10.0);
                        }
                    });
                });
            }
        });
    }

    fn show_audit_tab(&mut self, ui: &mut egui::Ui, registry: &mut USTRegistry) {
        ui.label("Matterhorn Protocol / UA-2 Accessibility Checklist:");
        ui.add_space(5.0);

        egui::ScrollArea::vertical().id_salt("audit_scroll").show(ui, |ui| {
            if registry.audit_findings.is_empty() {
                ui.centered_and_justified(|ui| {
                    ui.colored_label(egui::Color32::GREEN, "🎉 100% Matterhorn Compliance! No errors found.");
                });
            } else {
                for (checkpoint, severity, message, handle_id) in &registry.audit_findings {
                    let inner_res = ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::RED, "🚨");
                            ui.colored_label(egui::Color32::LIGHT_RED, checkpoint);
                            ui.label(format!("({})", severity));
                        });
                        ui.label(message);
                    });

                    // Make the warning card clickable to focus the violation
                    let id = ui.id().with(checkpoint).with(message);
                    let response = ui.interact(inner_res.response.rect, id, egui::Sense::click());
                    if response.clicked() {
                        if let Some(h_id) = handle_id {
                            if let Some(node_id) = registry.find_node_id_by_handle_id(*h_id) {
                                registry.selected_node_id = Some(node_id);
                                registry.pending_center_node_id = Some(node_id);
                            }
                        }
                    }

                    if response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }

                    ui.add_space(5.0);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_node() {
        let mut registry = USTRegistry::new();
        registry.initialize_mock_tree(1);

        // root is Document (id 0)
        // children: Page 1 Section (id 1)
        //   children: Heading of Page 1 (id 2), Paragraph (id 3), Illustration (id 4)
        
        // Let's move Paragraph (id 3) Above Heading (id 2)
        assert!(registry.move_node(3, 2, DragRelation::Above));
        
        let root = registry.root.as_ref().unwrap();
        let page = &root.children[0];
        assert_eq!(page.children[0].id, 3);
        assert_eq!(page.children[1].id, 2);

        // Let's move Illustration (id 4) As Child of Paragraph (id 3)
        assert!(registry.move_node(4, 3, DragRelation::AsChild));
        
        let root = registry.root.as_ref().unwrap();
        let page = &root.children[0];
        assert_eq!(page.children[0].id, 3);
        assert_eq!(page.children[0].children[0].id, 4);

        // Invalid moves: dragging parent to child should fail
        assert!(!registry.move_node(3, 4, DragRelation::Above));
    }
}
