use rust_flutter::{
    egui::{self, vec2},
    layout::*,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Policy {
    mode: Rc<Cell<u8>>,
    calls: Rc<Cell<usize>>,
    dirty: Rc<RefCell<Option<CustomBox>>>,
}
impl CustomBoxLayout for Policy {
    fn layout(
        &mut self,
        c: BoxConstraints,
        children: &mut BoxChildren<'_>,
    ) -> Result<BoxGeometry, LayoutError> {
        self.calls.set(self.calls.get() + 1);
        if let Some(handle) = self.dirty.borrow_mut().take() {
            handle.invalidate();
        }
        let mut g = BoxGeometry::new(c.constrain(vec2(100.0, 100.0))?);
        g.hit_test_self = true;
        match self.mode.get() {
            1 => {
                children.layout(99, c)?;
            }
            2 => {
                children.place(0, vec2(0.0, 0.0))?;
            }
            3 => {
                children.layout(0, c)?;
                children.place(0, vec2(0.0, 0.0))?;
                children.place(0, vec2(0.0, 0.0))?;
            }
            4 => {
                children.layout(0, c)?;
                children.place(0, vec2(f32::NAN, 0.0))?;
            }
            5 => {}
            6 => {
                g.size = vec2(9999.0, 9999.0);
            }
            7 => {
                children.layout(0, c)?;
                children.place(0, vec2(0.0, 0.0))?;
                children.layout(0, c)?;
            }
            _ => {
                for i in (0..children.len()).rev() {
                    children.layout(i, c.loosen())?;
                    children.place(i, vec2(0.0, 0.0))?;
                }
            }
        }
        Ok(g)
    }
}
fn setup() -> (CustomBox, Rc<Cell<u8>>, Rc<Cell<usize>>) {
    let mode = Rc::new(Cell::new(0));
    let calls = Rc::new(Cell::new(0));
    (
        CustomBox::new(Policy {
            mode: mode.clone(),
            calls: calls.clone(),
            dirty: Rc::default(),
        }),
        mode,
        calls,
    )
}
fn node(layout: CustomBox) -> BoxNode {
    BoxNode::new(
        "custom",
        BoxKind::Custom {
            layout,
            children: vec![
                BoxNode::leaf("A", vec2(20.0, 20.0)),
                BoxNode::leaf("B", vec2(20.0, 20.0)),
            ],
        },
    )
}
fn constraints() -> BoxConstraints {
    BoxConstraints::loose(vec2(200.0, 200.0)).unwrap()
}
#[test]
fn custom_layout_validates_child_protocol_and_recovers_after_each_error() {
    let (handle, mode, _) = setup();
    let mut node = node(handle.clone());
    for failure in 1..=7 {
        mode.set(failure);
        handle.invalidate();
        let error = node.layout(constraints()).unwrap_err();
        if failure == 6 {
            assert_eq!(error, LayoutError::ChildViolatedConstraints(node.id));
        } else {
            assert!(
                matches!(error, LayoutError::InvalidConfiguration(_)),
                "{error:?}"
            );
        }
        mode.set(0);
        handle.invalidate();
        assert!(node.layout(constraints()).is_ok());
    }
}
#[test]
fn cached_geometry_preserves_child_order_self_hits_and_global_key_validation() {
    let (handle, _, calls) = setup();
    let mut root = node(handle.clone());
    let first = root.layout(constraints()).unwrap();
    assert_eq!(
        first.hit_test(egui::pos2(10.0, 10.0)),
        Some(egui::Id::new("A"))
    );
    assert_eq!(first.hit_test(egui::pos2(50.0, 50.0)), Some(root.id));
    assert_eq!(first.hit_test(egui::pos2(100.0, 50.0)), None);
    assert_eq!(first.children[0].layout.id, egui::Id::new("B"));
    root.layout(constraints()).unwrap();
    assert_eq!(calls.get(), 1);
    handle.invalidate();
    root.layout(constraints()).unwrap();
    assert_eq!(calls.get(), 2);
    if let BoxKind::Custom { children, .. } = &mut root.kind {
        children[1].id = children[0].id;
    }
    assert!(matches!(
        root.layout(constraints()),
        Err(LayoutError::DuplicateKey(_))
    ));
}
#[test]
fn local_invalidations_during_layout_are_not_lost_or_reentrantly_borrowed() {
    let mode = Rc::new(Cell::new(0));
    let calls = Rc::new(Cell::new(0));
    let dirty = Rc::new(RefCell::new(None));
    let handle = CustomBox::new(Policy {
        mode,
        calls: calls.clone(),
        dirty: dirty.clone(),
    });
    *dirty.borrow_mut() = Some(handle.clone());
    let mut root = node(handle);
    root.layout(constraints()).unwrap();
    root.layout(constraints()).unwrap();
    root.layout(constraints()).unwrap();
    assert_eq!(calls.get(), 2);
}

#[test]
fn nested_reuse_of_one_layout_object_returns_an_error_instead_of_panicking() {
    let (handle, _, _) = setup();
    let inner = BoxNode::new(
        "inner",
        BoxKind::Custom {
            layout: handle.clone(),
            children: vec![],
        },
    );
    let mut outer = BoxNode::new(
        "outer",
        BoxKind::Custom {
            layout: handle,
            children: vec![inner],
        },
    );
    assert!(matches!(
        outer.layout(constraints()),
        Err(LayoutError::InvalidConfiguration(
            "custom layout object is already active"
        ))
    ));
}
