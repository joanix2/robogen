//! A local 2D camera and hit-testing engine. Dragging changes only presentation.
use super::{drawing, layout};
use egui::{pos2, vec2, Pos2, Rect, Sense, Ui, Vec2};
use robogen_domain::{mechanics::MechanicalModel, JointId, RigidBodyId, SourceSpan};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagramMode {
    Graph,
    Schematic,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Projection {
    Xy,
    Xz,
    Yz,
}
impl Projection {
    pub(super) fn project(self, point: [f64; 3]) -> Pos2 {
        let (x, y) = match self {
            Self::Xy => (point[0], point[1]),
            Self::Xz => (point[0], point[2]),
            Self::Yz => (point[1], point[2]),
        };
        pos2(x as f32, -y as f32)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Selection {
    Body(RigidBodyId),
    Joint(JointId),
}
#[derive(Clone)]
pub(super) struct HitRegion {
    pub selection: Selection,
    pub rect: Rect,
}

pub(crate) struct GraphCanvas {
    pub(crate) selected: Option<Selection>,
    pub(crate) mode: DiagramMode,
    pub(super) projection: Projection,
    pub(super) positions: BTreeMap<RigidBodyId, Pos2>,
    previous_model: Option<MechanicalModel>,
    pub(super) zoom: f32,
    pan: Vec2,
    fit: bool,
    drag: Option<RigidBodyId>,
}
impl Default for GraphCanvas {
    fn default() -> Self {
        Self {
            selected: None,
            mode: DiagramMode::Graph,
            projection: Projection::Xy,
            positions: BTreeMap::new(),
            previous_model: None,
            zoom: 1.0,
            pan: Vec2::ZERO,
            fit: true,
            drag: None,
        }
    }
}
impl GraphCanvas {
    pub(super) fn set_mode(&mut self, mode: DiagramMode) {
        self.mode = mode;
        self.fit = true;
    }
    pub(super) fn set_projection(&mut self, projection: Projection) {
        self.projection = projection;
        self.fit = true;
    }
    pub(super) fn fit_view(&mut self) {
        self.fit = true;
    }
    pub(super) fn reset_layout(&mut self) {
        self.positions.clear();
        self.fit = true;
    }
    pub(super) fn selection_source<'a>(
        &self,
        model: &'a MechanicalModel,
    ) -> Option<(&'a str, SourceSpan)> {
        match self.selected? {
            Selection::Body(id) => model
                .bodies
                .get(&id)
                .map(|body| (body.name.as_str(), body.span)),
            Selection::Joint(id) => model
                .joints
                .get(&id)
                .map(|joint| (joint.name.as_str(), joint.span)),
        }
    }
    pub(super) fn screen(&self, rect: Rect, point: Pos2) -> Pos2 {
        rect.center() + self.pan + point.to_vec2() * self.zoom
    }
    fn zoom_at(&mut self, rect: Rect, pointer: Pos2, factor: f32) {
        let previous = self.zoom;
        self.zoom = (self.zoom * factor).clamp(0.02, 8.0);
        let offset = pointer - rect.center();
        self.pan = offset - (offset - self.pan) * (self.zoom / previous);
    }
    fn prepare(&mut self, model: &MechanicalModel, rect: Rect) {
        if self.previous_model.as_ref() != Some(model) {
            let topology_changed = self.previous_model.as_ref().is_none_or(|old| {
                old.bodies.keys().ne(model.bodies.keys())
                    || old
                        .joints
                        .iter()
                        .map(|(id, j)| (id, j.parent, j.child))
                        .ne(model.joints.iter().map(|(id, j)| (id, j.parent, j.child)))
            });
            if topology_changed {
                self.positions.clear();
            }
            if self.selection_source(model).is_none() {
                self.selected = None;
            }
            self.previous_model = Some(model.clone());
            self.fit = true;
        }
        if self.positions.is_empty() {
            self.positions = layout::graph_positions(model);
        }
        if self.fit {
            let mut bounds = Rect::NOTHING;
            if self.mode == DiagramMode::Graph {
                for position in self.positions.values() {
                    bounds.extend_with(*position);
                }
            } else {
                for joint in model.joints.values() {
                    bounds.extend_with(
                        self.projection
                            .project(joint.origin.map(|v| v.millimetres())),
                    );
                }
            }
            if bounds == Rect::NOTHING {
                bounds = Rect::from_center_size(Pos2::ZERO, vec2(100.0, 100.0));
            }
            let available = (rect.size() - vec2(230.0, 160.0)).max(vec2(40.0, 40.0));
            self.zoom = (available.x / bounds.width().max(80.0))
                .min(available.y / bounds.height().max(80.0))
                .clamp(0.02, 3.0);
            self.pan = -bounds.center().to_vec2() * self.zoom;
            self.fit = false;
        }
    }
    pub(super) fn show(&mut self, ui: &mut Ui, model: &MechanicalModel) -> Option<SourceSpan> {
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(vec2(1.0, 1.0)),
            Sense::click_and_drag(),
        );
        self.prepare(model, rect);
        if response.hovered() {
            ui.input(|input| {
                if let Some(pointer) = input.pointer.hover_pos() {
                    self.zoom_at(
                        rect,
                        pointer,
                        (input.smooth_scroll_delta.y * 0.002).exp() * input.zoom_delta(),
                    );
                }
            });
        }
        let hits = drawing::paint(self, ui, rect, model);
        if response.clicked() {
            self.selected = ui
                .input(|input| input.pointer.interact_pos())
                .and_then(|p| pick(&hits, p));
            ui.ctx().request_repaint();
        }
        if response.drag_started() {
            self.drag = ui
                .input(|input| input.pointer.press_origin())
                .and_then(|p| pick(&hits, p))
                .and_then(|selection| {
                    self.selected = Some(selection);
                    match (self.mode, selection) {
                        (DiagramMode::Graph, Selection::Body(id)) => Some(id),
                        _ => None,
                    }
                });
        }
        if response.dragged() {
            let delta = ui.input(|input| input.pointer.delta());
            if let Some(position) = self.drag.and_then(|id| self.positions.get_mut(&id)) {
                *position += delta / self.zoom;
            } else {
                self.pan += delta;
            }
            ui.ctx().request_repaint();
        }
        if response.drag_stopped() {
            self.drag = None;
        }
        if response.double_clicked() {
            self.selection_source(model).map(|(_, span)| span)
        } else {
            None
        }
    }
}
fn pick(hits: &[HitRegion], point: Pos2) -> Option<Selection> {
    hits.iter()
        .rev()
        .find(|hit| hit.rect.contains(point))
        .map(|hit| hit.selection)
}
#[cfg(test)]
mod tests;
