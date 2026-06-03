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

    fn make_mock_page_node(&self, page_num: usize, next_id: usize) -> USTNode {
        USTNode {
            id: next_id,
            tag: "Part".to_string(),
            title: format!("Page {} Section", page_num),
            alt_text: None,
            rect: None,
            handle_id: None,
            children: vec![
                USTNode {
                    id: next_id + 1,
                    tag: "H1".to_string(),
                    title: format!("Heading of Page {}", page_num),
                    alt_text: None,
                    rect: None,
                    handle_id: None,
                    children: Vec::new(),
                },
                USTNode {
                    id: next_id + 2,
                    tag: "P".to_string(),
                    title: format!("Paragraph content for page {}", page_num),
                    alt_text: None,
                    rect: None,
                    handle_id: None,
                    children: Vec::new(),
                },
                USTNode {
                    id: next_id + 3,
                    tag: "Figure".to_string(),
                    title: format!("Illustration on page {}", page_num),
                    alt_text: None,
                    rect: None,
                    handle_id: None,
                    children: Vec::new(),
                },
            ],
        }
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
            let page_node = self.make_mock_page_node(i + 1, next_id);
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

    pub fn find_node_by_id_recursive<'a>(current: &'a USTNode, id: usize) -> Option<&'a USTNode> {
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
    pub active_tab: usize, // Unused but kept for API compatibility
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
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            
            self.show_structure_tree(ui, registry, tx_worker);

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            Self::show_element_properties(ui, registry, tx_worker);

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            self.show_alt_text_gallery(ui, registry, tx_worker);

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);

            Self::show_accessibility_audit(ui, registry);
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
    }

    fn show_structure_tree(
        &mut self,
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Structure Tree").strong().size(13.0));
            ui.add_space(4.0);

            let mut selected_node_id = registry.selected_node_id;
            egui::ScrollArea::vertical()
                .id_salt("tag_tree_scroll")
                .max_height(160.0)
                .show(ui, |ui| {
                    if let Some(ref mut root) = registry.root {
                        Self::render_node_recursive(ui, root, &mut selected_node_id, &mut self.alt_text_edit_buffer, tx_worker);
                    } else {
                        ui.label(egui::RichText::new("No structure tree loaded.").weak());
                    }
                });
            registry.selected_node_id = selected_node_id;
        });
    }

    fn show_element_properties( // RR-15 Limit: GUI - Render properties grid for selected UST node
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Element Properties").strong().size(13.0));
            ui.add_space(6.0);

            let selected_id = registry.selected_node_id;
            let mut node_found = false;

            if let Some(id) = selected_id {
                if let Some(ref mut root) = registry.root {
                    if let Some(node) = Self::find_node_mut_recursive(root, id) {
                        node_found = true;
                        egui::Grid::new("properties_grid")
                            .num_columns(2)
                            .spacing([10.0, 8.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("Tag:").weak());
                                let old_tag = node.tag.clone();
                                egui::ComboBox::from_id_salt("properties_tag_combobox")
                                    .selected_text(&node.tag)
                                    .show_ui(ui, |ui| {
                                        for t in &["H1", "H2", "P", "Figure", "Table", "List", "Part", "Document"] {
                                            ui.selectable_value(&mut node.tag, t.to_string(), *t);
                                        }
                                    });
                                if node.tag != old_tag {
                                    if let Some(h_id) = node.handle_id {
                                        let _ = tx_worker.send(crate::worker::WorkerRequest::UpdateNode {
                                            handle_id: h_id,
                                            tag: node.tag.clone(),
                                            alt_text: node.alt_text.clone(),
                                        });
                                    }
                                }
                                ui.end_row();

                                ui.label(egui::RichText::new("Title:").weak());
                                ui.label(egui::RichText::new(&node.title).strong());
                                ui.end_row();

                                ui.label(egui::RichText::new("BBox:").weak());
                                if let Some(rect) = node.rect {
                                    ui.monospace(format!("[{:.1}, {:.1}, {:.1}, {:.1}]", rect[0], rect[1], rect[2], rect[3]));
                                } else {
                                    ui.monospace("None");
                                }
                                ui.end_row();

                                ui.label(egui::RichText::new("Lang:").weak());
                                ui.label("en-US");
                                ui.end_row();

                                ui.label(egui::RichText::new("Role Map:").weak());
                                ui.label("Default Mapping");
                                ui.end_row();

                                ui.label(egui::RichText::new("Alt Text:").weak());
                                let mut buf = node.alt_text.clone().unwrap_or_default();
                                let text_resp = ui.text_edit_singleline(&mut buf);
                                if text_resp.changed() {
                                    node.alt_text = if buf.trim().is_empty() { None } else { Some(buf) };
                                    if let Some(h_id) = node.handle_id {
                                        let _ = tx_worker.send(crate::worker::WorkerRequest::UpdateNode {
                                            handle_id: h_id,
                                            tag: node.tag.clone(),
                                            alt_text: node.alt_text.clone(),
                                        });
                                    }
                                }
                                ui.end_row();
                            });
                    }
                }
            }

            if !node_found {
                ui.label(egui::RichText::new("Select a node to inspect properties.").weak());
            }
        });
    }

    fn show_accessibility_audit(ui: &mut egui::Ui, registry: &mut USTRegistry) { // RR-15 Limit: GUI - Render accessibility audit findings panel
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Accessibility Audit").strong().size(13.0));
            ui.add_space(6.0);

            let has_doc = registry.root.is_some();
            let audit_findings_count = registry.audit_findings.len();

            ui.vertical(|ui| {
                if has_doc {
                    let compliant_pct = if audit_findings_count == 0 {
                        100
                    } else {
                        (100 - audit_findings_count * 7).max(10)
                    };
                    ui.label(format!("Matterhorn: {}% Compliant", compliant_pct));
                    ui.label(format!("Findings: {}", audit_findings_count));
                } else {
                    ui.label("Matterhorn: -");
                    ui.label("Findings: -");
                }
            });

            ui.add_space(4.0);

            egui::ScrollArea::vertical()
                .id_salt("audit_scroll")
                .max_height(100.0)
                .show(ui, |ui| {
                    if !has_doc {
                        ui.label(egui::RichText::new("No document loaded.").weak());
                    } else if registry.audit_findings.is_empty() {
                        ui.colored_label(egui::Color32::GREEN, "100% Compliant! No errors.");
                    } else {
                        for (checkpoint, severity, message, handle_id) in &registry.audit_findings {
                            let card_resp = ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(egui::Color32::LIGHT_RED, checkpoint);
                                    ui.label(format!("({})", severity));
                                });
                                ui.label(message);
                            });

                            let id = ui.id().with(checkpoint).with(message);
                            let response = ui.interact(card_resp.response.rect, id, egui::Sense::click());
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
                            ui.add_space(3.0);
                        }
                    }
                });
        });
    }

    fn find_node_mut_recursive(node: &mut USTNode, id: usize) -> Option<&mut USTNode> {
        if node.id == id {
            return Some(node);
        }
        for child in &mut node.children {
            if let Some(found) = Self::find_node_mut_recursive(child, id) {
                return Some(found);
            }
        }
        None
    }

    fn render_drag_drop_controls(ui: &mut egui::Ui, node_id: usize, node: &USTNode) {
        let handle_resp = ui.add(egui::Label::new("Drag").sense(egui::Sense::drag()));
        if handle_resp.drag_started() {
            ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("dragged_node_id"), Some(node_id)));
        }

        let dragged_id: Option<Option<usize>> = ui.ctx().data(|d| d.get_temp(egui::Id::new("dragged_node_id")));
        if let Some(Some(drag_id)) = dragged_id {
            if drag_id != node_id && !USTRegistry::is_descendant(node, drag_id) {
                let resp_above = ui.button("Above");
                if resp_above.clicked() || (resp_above.hovered() && ui.input(|i| i.pointer.any_released())) {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pending_move"), Some((drag_id, node_id, DragRelation::Above))));
                }
                let resp_child = ui.button("Child");
                if resp_child.clicked() || (resp_child.hovered() && ui.input(|i| i.pointer.any_released())) {
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("pending_move"), Some((drag_id, node_id, DragRelation::AsChild))));
                }
                let resp_below = ui.button("Below");
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
        if ui.button("Edit").clicked() {
            *selected_node_id = Some(node.id);
            *alt_edit_buf = node.alt_text.clone().unwrap_or_default();
        }

        if ui.button("Cycle").clicked() {
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

    fn render_node_recursive(
        ui: &mut egui::Ui,
        node: &mut USTNode,
        selected_node_id: &mut Option<usize>,
        alt_edit_buf: &mut String,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        let is_selected = *selected_node_id == Some(node.id);
        let header_label = format!("<{}> {}", node.tag, node.title);

        ui.vertical(|ui| {
            let id = ui.make_persistent_id(node.id);
            let mut collapsing = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);

            let header_response = ui.horizontal(|ui| {
                Self::render_drag_drop_controls(ui, node.id, node);

                let is_open = collapsing.is_open();
                let symbol = if is_open { "⏷" } else { "⏵" };
                if ui.small_button(symbol).clicked() {
                    collapsing.toggle(ui);
                }

                let rich_text = if is_selected {
                    egui::RichText::new(&header_label).color(egui::Color32::from_rgb(240, 165, 0)).strong()
                } else {
                    egui::RichText::new(&header_label)
                };

                if ui.selectable_label(is_selected, rich_text).clicked() {
                    *selected_node_id = Some(node.id);
                    *alt_edit_buf = node.alt_text.clone().unwrap_or_default();
                }

                if is_selected {
                    Self::render_node_buttons(ui, node, selected_node_id, alt_edit_buf, tx_worker);
                }
            }).response;

            collapsing.show_body_indented(&header_response, ui, |ui| {
                let children_len = node.children.len();
                for idx in 0..children_len {
                    let child = &mut node.children[idx];
                    Self::render_node_recursive(ui, child, selected_node_id, alt_edit_buf, tx_worker);
                }
            });
        });
    }

    fn show_alt_text_gallery(
        &mut self,
        ui: &mut egui::Ui,
        registry: &mut USTRegistry,
        tx_worker: &std::sync::mpsc::Sender<crate::worker::WorkerRequest>,
    ) {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("Alt-Text Gallery").strong());
            ui.add_space(2.0);

            let mut figures = Vec::new();
            if let Some(ref root) = registry.root {
                collect_figures(root, &mut figures);
            }

            if figures.is_empty() {
                ui.label("No figures found.");
            } else {
                egui::ScrollArea::horizontal().id_salt("figure_gallery_carousel").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for fig in &figures {
                            ui.vertical(|ui| {
                                ui.set_min_width(120.0);
                                ui.vertical(|ui| {
                                    ui.colored_label(egui::Color32::LIGHT_BLUE, fig.title.clone());
                                    
                                    let mut buf = fig.alt_text.clone().unwrap_or_default();
                                    let response = ui.add(egui::TextEdit::singleline(&mut buf).hint_text("Description..."));
                                    
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
                            ui.add_space(5.0);
                        }
                    });
                });
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
