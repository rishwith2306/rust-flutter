use super::*;
/// Reserve root geometry and paint participating boxes inside stable child UIs.
/// Custom boxes paint before their children; ordinary leaves paint as before.
/// The callback should fit widgets/paint to `ui.max_rect()`; clip inheritance follows
/// the geometry tree. Leaf UIs have independent layout cursors and stable IDs.
pub fn show_layout(
    ui: &mut egui::Ui,
    layout: &LayoutBox,
    mut render: impl FnMut(Id, &mut egui::Ui),
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(layout.size, egui::Sense::hover());
    let scope = ui.id().with(layout.id);
    visit(ui, scope, layout, rect.min, ui.clip_rect(), &mut render);
    response
}
fn visit(
    ui: &mut egui::Ui,
    scope: Id,
    layout: &LayoutBox,
    origin: Pos2,
    inherited_clip: Rect,
    render: &mut impl FnMut(Id, &mut egui::Ui),
) {
    let rect = Rect::from_min_size(origin, layout.size);
    let clip = if layout.clip == Clip::HardEdge {
        inherited_clip.intersect(rect)
    } else {
        inherited_clip
    };
    if layout.paint_self {
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id(scope.with(layout.id))
                .max_rect(rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(clip);
        render(layout.id, &mut child);
    }
    for child in &layout.children {
        visit(
            ui,
            scope,
            &child.layout,
            origin + child.offset,
            clip,
            render,
        );
    }
}
