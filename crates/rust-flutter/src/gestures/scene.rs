use super::*;
use crate::egui::{self, LayerId, Rect, Ui, ViewportId};

struct Root {
    geometry: LayoutBox,
    origin: Pos2,
    clip: Rect,
    layer: LayerId,
    enabled: bool,
}
/// Collect all roots for one viewport/pass, in paint order, then call finish.
/// Input is dispatched after collection; repeated egui passes repaint without
/// replaying input. Dropping an unfinished collector cancels captured input.
#[must_use = "finish the gesture frame after collecting all roots"]
pub struct GestureFrame {
    binding: GestureBinding,
    context: egui::Context,
    viewport: ViewportId,
    frame: u64,
    lease: u64,
    replay: bool,
    switched: bool,
    roots: Vec<Root>,
    finished: bool,
}
fn contains(rect: Rect, point: Pos2) -> bool {
    point.x >= rect.min.x && point.y >= rect.min.y && point.x < rect.max.x && point.y < rect.max.y
}
impl DispatchReport {
    pub(super) fn merge(&mut self, other: Self) {
        self.processed += other.processed;
        self.queued |= other.queued;
        self.failures.extend(other.failures);
    }
}
impl GestureBinding {
    /// One collector per viewport/pass. The same binding must be used by all
    /// detector/button descendants. Other viewports require separate bindings.
    pub fn begin_egui_frame(&self, context: &egui::Context) -> Result<GestureFrame, BindingError> {
        if self.is_disposed() {
            return Err(BindingError::Disposed);
        }
        let viewport = context.viewport_id();
        let frame = context.cumulative_frame_nr();
        let mut slot = self.0.host.borrow_mut();
        let host = slot.get_or_insert_with(|| EguiHost {
            context: context.clone(),
            viewport,
            root: None,
            frame: None,
            collecting: None,
            next_lease: 0,
            previous_targets: Vec::new(),
        });
        if host.context != *context || host.viewport != viewport {
            return Err(BindingError::HostChanged);
        }
        if host.collecting.is_some() {
            return Err(BindingError::FrameAlreadyOpen);
        }
        if host.frame == Some(frame) && host.root.is_some() {
            return Err(BindingError::MultipleRoots);
        }
        let lease = host
            .next_lease
            .checked_add(1)
            .ok_or(BindingError::IdExhausted)?;
        host.next_lease = lease;
        host.collecting = Some((frame, lease));
        Ok(GestureFrame {
            binding: self.clone(),
            context: context.clone(),
            viewport,
            frame,
            lease,
            replay: host.frame == Some(frame),
            switched: host.root.is_some(),
            roots: Vec::new(),
            finished: false,
        })
    }
}
impl GestureFrame {
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }
    fn check(&self) -> Result<(), BindingError> {
        if self.binding.is_disposed() {
            return Err(BindingError::Disposed);
        }
        if self.context.cumulative_frame_nr() != self.frame
            || self.context.viewport_id() != self.viewport
        {
            return Err(BindingError::FrameExpired);
        }
        if !self
            .binding
            .0
            .host
            .borrow()
            .as_ref()
            .is_some_and(|h| h.collecting == Some((self.frame, self.lease)))
        {
            return Err(BindingError::FrameExpired);
        }
        Ok(())
    }
    /// Register current laid-out geometry and paint it separately, or use
    /// ElementTree::show_in_gesture_frame. Later registrations paint above earlier
    /// ones within a layer. Geometry is snapshotted; no tree borrow is retained.
    pub fn add_root(
        &mut self,
        ui: &Ui,
        geometry: &LayoutBox,
        origin: Pos2,
    ) -> Result<(), BindingError> {
        self.check()?;
        if ui.ctx() != &self.context || ui.ctx().viewport_id() != self.viewport {
            return Err(BindingError::HostChanged);
        }
        if !origin.is_finite() {
            return Err(BindingError::InvalidPosition);
        }
        if self.roots.iter().any(|r| r.geometry.id == geometry.id) {
            return Err(BindingError::DuplicateRoot(geometry.id));
        }
        // Give a bare custom Ui layer an explicit backend paint-order entry.
        // Existing Area/Window layers retain their current stacking position.
        if !self
            .context
            .memory(|m| m.layer_ids().any(|l| l == ui.layer_id()))
        {
            self.context.move_to_top(ui.layer_id());
        }
        self.roots.push(Root {
            geometry: geometry.clone(),
            origin,
            clip: ui.clip_rect(),
            layer: ui.layer_id(),
            enabled: ui.is_enabled(),
        });
        Ok(())
    }
    fn local(&self, root: &Root, position: Pos2) -> Pos2 {
        self.context
            .layer_transform_from_global(root.layer)
            .map_or(position, |t| t * position)
    }
    fn path_at(&self, position: Pos2) -> Option<Vec<Id>> {
        let top = self.context.layer_id_at(position);
        let layers = self.context.memory(|m| m.layer_ids().collect::<Vec<_>>());
        let rank = |layer: LayerId| {
            (
                layer.order,
                layers
                    .iter()
                    .position(|l| *l == layer)
                    .unwrap_or(usize::MAX),
            )
        };
        // A backend area not represented by any root at this point occludes the
        // scene. Its ordinary controls remain independent of these arenas.
        if top.is_some_and(|layer| {
            !self.roots.iter().any(|r| {
                rank(r.layer) >= rank(layer)
                    && contains(r.clip, self.local(r, position))
                    && contains(
                        Rect::from_min_size(r.origin, r.geometry.size),
                        self.local(r, position),
                    )
            })
        }) {
            return None;
        }
        let mut roots: Vec<_> = self.roots.iter().enumerate().collect();
        roots.sort_by_key(|(i, r)| {
            (
                r.layer.order,
                layers
                    .iter()
                    .position(|l| *l == r.layer)
                    .unwrap_or(usize::MAX),
                *i,
            )
        });
        let mut path = Vec::new();
        let mut enabled_hit = false;
        for (_, root) in roots.into_iter().rev() {
            if !self.context.memory(|m| m.allows_interaction(root.layer)) {
                continue;
            }
            let point = self.local(root, position);
            if !contains(root.clip, point)
                || !contains(Rect::from_min_size(root.origin, root.geometry.size), point)
            {
                continue;
            }
            let hit = root.geometry.hit_test_result(point - root.origin.to_vec2());
            if root.enabled {
                enabled_hit = true;
                path.extend(hit.path.into_iter().map(|e| e.id));
            }
            if hit.absorbed {
                break;
            }
        }
        enabled_hit.then_some(path)
    }
    fn update_visibility(&self) {
        fn targets(g: &LayoutBox, state: &State, out: &mut Vec<Weak<Target>>) {
            if let Some(t) = state.targets.get(&g.id) {
                out.push(Rc::downgrade(t));
            }
            for child in &g.children {
                targets(&child.layout, state, out);
            }
        }
        let (all, enabled) = {
            let state = self.binding.0.state.borrow();
            let mut all = Vec::new();
            let mut enabled = HashSet::new();
            for root in &self.roots {
                let start = all.len();
                targets(&root.geometry, &state, &mut all);
                if root.enabled && self.context.memory(|m| m.allows_interaction(root.layer)) {
                    enabled.extend(all[start..].iter().map(Weak::as_ptr));
                }
            }
            (all, enabled)
        };
        let previous = {
            let mut host = self.binding.0.host.borrow_mut();
            let host = host.as_mut().unwrap();
            std::mem::replace(&mut host.previous_targets, all.clone())
        };
        let mut canceled = HashSet::new();
        for target in previous.into_iter().chain(all) {
            if !enabled.contains(&target.as_ptr())
                && canceled.insert(target.as_ptr())
                && let Some(target) = target.upgrade()
            {
                let current = self
                    .binding
                    .0
                    .state
                    .borrow()
                    .targets
                    .values()
                    .any(|t| Rc::ptr_eq(t, &target));
                if current {
                    target.cancel();
                }
            }
        }
    }
    /// Dispatch this frame's raw primary-pointer input against all collected
    /// roots. Calls from a repeated egui pass update geometry/visibility only.
    pub fn finish(mut self) -> Result<DispatchReport, BindingError> {
        self.check()?;
        for root in &self.roots {
            if let Some(t) = self.context.layer_transform_to_global(root.layer) {
                let inverse = t.inverse();
                if !t.scaling.is_finite()
                    || t.scaling <= 0.0
                    || !t.translation.is_finite()
                    || !inverse.scaling.is_finite()
                    || !inverse.translation.is_finite()
                {
                    return Err(BindingError::InvalidTransform);
                }
            }
        }
        {
            let mut host = self.binding.0.host.borrow_mut();
            let host = host.as_mut().unwrap();
            host.frame = Some(self.frame);
            host.root = None;
        }
        let mut report = DispatchReport::default();
        if self.switched {
            report.merge(self.binding.cancel_all());
        }
        self.update_visibility();
        if !self.replay && !self.binding.is_disposed() {
            let (focused, time, events) = self
                .context
                .input(|i| (i.focused, i.time, i.events.clone()));
            if !focused {
                report.merge(self.binding.cancel_all());
            } else {
                let time = std::time::Duration::try_from_secs_f64(time).unwrap_or_default();
                let pointer = PointerId(0);
                let sample = |phase, position, buttons| PointerEvent {
                    pointer,
                    phase,
                    position,
                    time,
                    buttons,
                    device: PointerDeviceKind::Unknown,
                };
                for event in events {
                    if self.binding.is_disposed() {
                        break;
                    }
                    let active = self.binding.0.state.borrow().active.contains_key(&pointer);
                    match event {
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            ..
                        } => {
                            if let Some(path) = self.path_at(pos) {
                                if active {
                                    report.merge(self.binding.cancel_all());
                                }
                                report.merge(self.binding.dispatch(
                                    sample(PointerPhase::Down, pos, PointerButtons::PRIMARY),
                                    path,
                                ));
                            }
                        }
                        egui::Event::PointerMoved(pos) if active => {
                            report.merge(self.binding.dispatch(
                                sample(PointerPhase::Move, pos, PointerButtons::PRIMARY),
                                [],
                            ))
                        }
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: false,
                            ..
                        } if active => report.merge(
                            self.binding
                                .dispatch(sample(PointerPhase::Up, pos, PointerButtons::NONE), []),
                        ),
                        egui::Event::PointerGone | egui::Event::WindowFocused(false) => {
                            report.merge(self.binding.cancel_all())
                        }
                        egui::Event::PointerButton { button, .. }
                            if active && button != egui::PointerButton::Primary =>
                        {
                            report.merge(self.binding.cancel_all())
                        }
                        _ => {}
                    }
                }
            }
        }
        self.binding.arena().flush();
        self.release();
        self.finished = true;
        Ok(report)
    }
    fn release(&self) -> bool {
        let mut host = self.binding.0.host.borrow_mut();
        if let Some(host) = host.as_mut()
            && host.collecting == Some((self.frame, self.lease))
        {
            host.collecting = None;
            return true;
        }
        false
    }
}
impl Drop for GestureFrame {
    fn drop(&mut self) {
        if !self.finished && self.release() {
            self.binding.cancel_all();
        }
    }
}
