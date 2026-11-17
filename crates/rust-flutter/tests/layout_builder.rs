use rust_flutter::{
    egui::{self, vec2},
    element::*,
    layout::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn loose() -> BoxConstraints {
    BoxConstraints::loose(vec2(100.0, 100.0)).unwrap()
}
#[test]
fn first_layout_builds_before_return_and_identical_constraints_reuse_child() {
    let ctx = egui::Context::default();
    let calls = Rc::new(RefCell::new(vec![]));
    let seen = calls.clone();
    let mut t = ElementTree::new(
        &ctx,
        Widget::layout_builder(move |_, c| {
            seen.borrow_mut().push(c);
            Widget::baseline_leaf(
                vec2(40.0, 20.0),
                Baselines {
                    alphabetic: Some(9.0),
                    ideographic: Some(11.0),
                },
                |_| {},
            )
        }),
    )
    .unwrap();
    assert!(calls.borrow().is_empty());
    assert!(matches!(
        t.dry_layout(loose()),
        Err(ElementError::Layout(LayoutError::UnsupportedQuery(_)))
    ));
    assert!(matches!(
        t.intrinsic(IntrinsicDimension::MaxWidth, 100.0),
        Err(ElementError::Layout(LayoutError::UnsupportedQuery(_)))
    ));
    assert!(t.dry_baseline(loose(), TextBaseline::Alphabetic).is_err());
    assert!(calls.borrow().is_empty());
    let g = t.layout(loose()).unwrap();
    assert_eq!(g.size, vec2(40.0, 20.0));
    assert_eq!(g.children[0].layout.size, g.size);
    assert_eq!(
        g.baselines,
        Baselines {
            alphabetic: Some(9.0),
            ideographic: Some(11.0)
        }
    );
    assert_eq!(*calls.borrow(), vec![loose()]);
    t.layout(loose()).unwrap();
    assert_eq!(calls.borrow().len(), 1);
    let c = BoxConstraints::tight(vec2(120.0, 30.0)).unwrap();
    assert_eq!(t.layout(c).unwrap().size, c.min());
    assert_eq!(calls.borrow().len(), 2);
}
#[derive(Default)]
struct Log {
    id: Option<ElementId>,
    init: usize,
    dispose: usize,
}
struct Counter {
    log: Rc<RefCell<Log>>,
    value: i32,
}
struct CounterState {
    log: Rc<RefCell<Log>>,
    value: i32,
}
impl StatefulWidget for Counter {
    type State = CounterState;
    fn create_state(&self) -> CounterState {
        CounterState {
            log: self.log.clone(),
            value: self.value,
        }
    }
}
impl State<Counter> for CounterState {
    fn init_state(&mut self, _: &Counter, ctx: &StateContext<Self>) {
        let mut l = self.log.borrow_mut();
        l.id = Some(ctx.element_id());
        l.init += 1;
    }
    fn build(&mut self, _: &Counter, _: &StateContext<Self>) -> Widget {
        Widget::leaf(vec2(self.value as f32, 10.0), |_| {})
    }
    fn dispose(&mut self) {
        self.log.borrow_mut().dispose += 1;
    }
}
fn counter_builder(log: Rc<RefCell<Log>>, calls: Rc<Cell<usize>>, initial: i32) -> Widget {
    LayoutBuilder::new(move |_, _| {
        calls.set(calls.get() + 1);
        Widget::stateful(Counter {
            log: log.clone(),
            value: initial,
        })
        .keyed("counter")
    })
    .into_widget()
}
#[test]
fn configuration_and_constraint_changes_preserve_keyed_child_state() {
    let ctx = egui::Context::default();
    let log = Rc::new(RefCell::new(Log::default()));
    let calls = Rc::new(Cell::new(0));
    let mut t = ElementTree::new(&ctx, counter_builder(log.clone(), calls.clone(), 10)).unwrap();
    t.layout(loose()).unwrap();
    let id = log.borrow().id.unwrap();
    t.state_handle::<CounterState>(id)
        .unwrap()
        .update(|s| s.value = 27)
        .unwrap();
    t.layout(loose()).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(t.state::<CounterState>(id).unwrap().value, 27);
    t.update(counter_builder(log.clone(), calls.clone(), 99))
        .unwrap();
    t.flush().unwrap();
    assert_eq!(calls.get(), 1, "configuration must wait for layout");
    t.layout(BoxConstraints::loose(vec2(90.0, 90.0)).unwrap())
        .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(t.state::<CounterState>(id).unwrap().value, 27);
    assert_eq!(log.borrow().init, 1);
    assert_eq!(log.borrow().dispose, 0);
    drop(t);
    assert_eq!(log.borrow().dispose, 1);
}
#[derive(PartialEq)]
struct Theme(u32);
#[test]
fn inherited_dependencies_rebuild_with_latest_constraints_and_normal_scope() {
    let ctx = egui::Context::default();
    let seen = Rc::new(RefCell::new(vec![]));
    let observed = seen.clone();
    let mut t = ElementTree::new(
        &ctx,
        Widget::inherited(
            Theme(1),
            Widget::layout_builder(move |cx, c| {
                let value = cx.depend_on::<Theme>().unwrap().unwrap().0;
                observed.borrow_mut().push((value, c.max().x));
                Widget::leaf(vec2(20.0, 20.0), |_| {})
            }),
        ),
    )
    .unwrap();
    t.layout(loose()).unwrap();
    let provider = t.inherited_handle::<Theme>(t.root_id().unwrap()).unwrap();
    provider.update(|_| Theme(2)).unwrap();
    t.flush().unwrap();
    assert_eq!(seen.borrow().len(), 1);
    t.layout(BoxConstraints::loose(vec2(70.0, 70.0)).unwrap())
        .unwrap();
    assert_eq!(*seen.borrow(), vec![(1, 100.0), (2, 70.0)]);
}
fn nested(depth: usize, calls: Rc<Cell<usize>>) -> Widget {
    Widget::layout_builder(move |_, _| {
        calls.set(calls.get() + 1);
        if depth == 0 {
            Widget::leaf(vec2(22.0, 12.0), |_| {})
        } else {
            nested(depth - 1, calls.clone())
        }
    })
}
#[test]
fn nested_builders_stabilize_within_one_layout_call() {
    let ctx = egui::Context::default();
    let calls = Rc::new(Cell::new(0));
    let mut t = ElementTree::new(&ctx, nested(20, calls.clone())).unwrap();
    assert_eq!(t.layout(loose()).unwrap().size, vec2(22.0, 12.0));
    assert_eq!(calls.get(), 21);
    t.layout(loose()).unwrap();
    assert_eq!(calls.get(), 21);
}
#[test]
fn more_than_256_siblings_build_in_one_batch() {
    let ctx = egui::Context::default();
    let calls = Rc::new(Cell::new(0));
    let mut style = FlexStyle::new(Axis::Vertical);
    style.main_size = MainAxisSize::Min;
    let children = (0..600)
        .map(|i| {
            let calls = calls.clone();
            FlexItem::fixed(
                Widget::layout_builder(move |_, _| {
                    calls.set(calls.get() + 1);
                    Widget::leaf(vec2(1.0, 1.0), |_| {})
                })
                .keyed(i as u64),
            )
        })
        .collect();
    let mut t = ElementTree::new(&ctx, Widget::flex(style, children)).unwrap();
    t.layout(loose()).unwrap();
    assert_eq!(calls.get(), 600);
}
#[derive(Clone)]
struct Twice {
    first: Rc<Cell<f32>>,
    last: Rc<Cell<f32>>,
}
struct TwiceObject(Twice);
impl RenderObjectWidget for Twice {
    type Object = TwiceObject;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        TwiceObject(self.clone())
    }
    fn update_render_object(&self, o: &mut Self::Object, _: &RenderContext) {
        o.0 = self.clone();
    }
}
impl RenderBox for TwiceObject {
    fn layout_revision(&self) -> u64 {
        u64::from(self.0.first.get().to_bits()) | (u64::from(self.0.last.get().to_bits()) << 32)
    }
    fn layout(
        &mut self,
        _: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        children.layout(0, BoxConstraints::tight(vec2(self.0.first.get(), 10.0))?)?;
        let size = children.layout(0, BoxConstraints::tight(vec2(self.0.last.get(), 10.0))?)?;
        children.place(0, egui::Vec2::ZERO)?;
        Ok(BoxGeometry::new(size))
    }
}
#[test]
fn repeated_child_measurements_coalesce_and_matching_final_constraints_cancel_request() {
    let ctx = egui::Context::default();
    let seen = Rc::new(RefCell::new(vec![]));
    let calls = seen.clone();
    let first = Rc::new(Cell::new(20.0));
    let last = Rc::new(Cell::new(40.0));
    let child = Widget::layout_builder(move |_, c| {
        calls.borrow_mut().push(c.max().x);
        Widget::leaf(c.min(), |_| {})
    });
    let mut t = ElementTree::new(
        &ctx,
        Widget::render_object(
            Twice {
                first: first.clone(),
                last: last.clone(),
            },
            vec![child],
        ),
    )
    .unwrap();
    assert_eq!(t.layout(loose()).unwrap().size.x, 40.0);
    assert_eq!(*seen.borrow(), vec![40.0]);
    first.set(30.0);
    assert_eq!(t.layout(loose()).unwrap().size.x, 40.0);
    assert_eq!(*seen.borrow(), vec![40.0]);
    last.set(60.0);
    assert_eq!(t.layout(loose()).unwrap().size.x, 60.0);
    assert_eq!(*seen.borrow(), vec![40.0, 60.0]);
}
#[test]
fn first_paint_contains_built_child_without_an_extra_display_pass() {
    let ctx = egui::Context::default();
    let paints = Rc::new(RefCell::new(vec![]));
    let log = paints.clone();
    let builds = Rc::new(Cell::new(0));
    let built = builds.clone();
    let mut t = ElementTree::new(
        &ctx,
        Widget::layout_builder(move |_, _| {
            built.set(built.get() + 1);
            let log = log.clone();
            Widget::leaf(vec2(40.0, 20.0), move |ui| {
                log.borrow_mut().push(ui.max_rect().size());
                ui.painter()
                    .rect_filled(ui.max_rect(), 0.0, egui::Color32::RED);
            })
        }),
    )
    .unwrap();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                vec2(200.0, 200.0),
            )),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                t.show(ui, loose()).unwrap();
            });
        },
    );
    assert_eq!(builds.get(), 1);
    assert_eq!(*paints.borrow(), vec![vec2(40.0, 20.0)]);
}

#[derive(Clone)]
struct Guarded(Rc<RefCell<()>>);
struct GuardedObject(Guarded);
impl RenderObjectWidget for Guarded {
    type Object = GuardedObject;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        GuardedObject(self.clone())
    }
    fn update_render_object(&self, o: &mut Self::Object, _: &RenderContext) {
        o.0 = self.clone();
    }
}
impl RenderBox for GuardedObject {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let _guard = self.0.0.borrow_mut();
        let size = children.layout(0, c)?;
        children.place(0, egui::Vec2::ZERO)?;
        Ok(BoxGeometry::new(size))
    }
}
#[test]
fn builder_executes_after_all_ancestor_render_borrows_are_released() {
    let ctx = egui::Context::default();
    let guard = Rc::new(RefCell::new(()));
    let check = guard.clone();
    let child = Widget::layout_builder(move |_, _| {
        let _safe = check.borrow_mut();
        Widget::leaf(vec2(20.0, 10.0), |_| {})
    });
    let mut t = ElementTree::new(&ctx, Widget::render_object(Guarded(guard), vec![child])).unwrap();
    assert_eq!(t.layout(loose()).unwrap().size, vec2(20.0, 10.0));
}
#[derive(Clone)]
struct Oscillator(Rc<Cell<bool>>, Rc<Cell<bool>>);
struct OscillatorObject(Oscillator);
impl RenderObjectWidget for Oscillator {
    type Object = OscillatorObject;
    fn create_render_object(&self, _: &RenderContext) -> Self::Object {
        OscillatorObject(self.clone())
    }
    fn update_render_object(&self, o: &mut Self::Object, _: &RenderContext) {
        o.0 = self.clone();
    }
}
impl RenderBox for OscillatorObject {
    fn layout_revision(&self) -> u64 {
        u64::from(self.0.0.get()) | (u64::from(self.0.1.get()) << 1)
    }
    fn layout(
        &mut self,
        _: BoxConstraints,
        children: &mut BoxChildren<'_>,
        _: &RenderContext,
    ) -> Result<BoxGeometry, LayoutError> {
        let width = if !self.0.1.get() {
            40.0
        } else if self.0.0.get() {
            20.0
        } else {
            30.0
        };
        let size = children.layout(0, BoxConstraints::tight(vec2(width, 10.0))?)?;
        children.place(0, egui::Vec2::ZERO)?;
        Ok(BoxGeometry::new(size))
    }
}
#[test]
fn nonconverging_constraints_error_keeps_tree_mounted_and_can_retry() {
    let ctx = egui::Context::default();
    let flip = Rc::new(Cell::new(false));
    let enabled = Rc::new(Cell::new(true));
    let toggle = flip.clone();
    let active = enabled.clone();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let child = Widget::layout_builder(move |_, c| {
        count.set(count.get() + 1);
        if active.get() {
            toggle.set(!toggle.get());
        }
        Widget::leaf(c.min(), |_| {})
    });
    let mut t = ElementTree::new(
        &ctx,
        Widget::render_object(Oscillator(flip, enabled.clone()), vec![child]),
    )
    .unwrap();
    let id = t.root_id();
    assert!(matches!(
        t.layout(loose()),
        Err(ElementError::Layout(LayoutError::InvalidConfiguration(
            "layout builders did not stabilize after 256 transactions"
        )))
    ));
    assert_eq!(t.root_id(), id);
    let before = calls.get();
    assert!(t.dry_layout(loose()).is_err());
    assert_eq!(
        calls.get(),
        before,
        "failed transactions must not leave builds queued for dry queries"
    );
    enabled.set(false);
    assert_eq!(t.layout(loose()).unwrap().size.x, 40.0);
}
#[test]
fn bad_built_description_uses_normal_validation_and_unmounts_cleanly() {
    let ctx = egui::Context::default();
    let mut style = FlexStyle::new(Axis::Vertical);
    style.main_size = MainAxisSize::Min;
    let mut t = ElementTree::new(
        &ctx,
        Widget::layout_builder(move |_, _| {
            Widget::flex(
                style,
                vec![
                    FlexItem::fixed(Widget::empty().keyed("duplicate")),
                    FlexItem::fixed(Widget::empty().keyed("duplicate")),
                ],
            )
        }),
    )
    .unwrap();
    assert!(matches!(
        t.layout(loose()),
        Err(ElementError::DuplicateKey(_))
    ));
    assert!(t.root_id().is_none());
    t.update(Widget::layout_builder(|_, _| {
        Widget::leaf(vec2(30.0, 10.0), |_| {})
    }))
    .unwrap();
    assert_eq!(t.layout(loose()).unwrap().size.x, 30.0);
}
#[test]
fn context_rebuild_waits_for_layout_and_is_invalid_after_unmount() {
    let ctx = egui::Context::default();
    let saved = Rc::new(RefCell::new(None));
    let capture = saved.clone();
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut t = ElementTree::new(
        &ctx,
        Widget::layout_builder(move |cx, _| {
            *capture.borrow_mut() = Some(cx.clone());
            count.set(count.get() + 1);
            Widget::empty()
        }),
    )
    .unwrap();
    t.layout(loose()).unwrap();
    let handle = saved.borrow().clone().unwrap();
    handle.request_rebuild().unwrap();
    t.flush().unwrap();
    assert_eq!(calls.get(), 1);
    t.layout(loose()).unwrap();
    assert_eq!(calls.get(), 2);
    t.unmount();
    assert!(!handle.mounted());
    assert_eq!(handle.request_rebuild(), Err(Unmounted));
}
