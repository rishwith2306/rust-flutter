use rust_flutter::{
    egui,
    element::*,
    forms::*,
    layout::BoxConstraints,
    notifier::{Listenable, ValueListenable},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[test]
fn passive_validity_explicit_validation_interaction_clear_and_reset() {
    let field = FormFieldController::new(String::new());
    field.set_validator(Some(Rc::new(|s| s.is_empty().then(|| "Required".into()))));
    assert!(!field.is_valid().unwrap());
    assert_eq!(field.error_text(), None);
    assert!(!field.validate().unwrap());
    assert_eq!(field.error_text().as_deref(), Some("Required"));
    assert!(!field.has_interacted_by_user());
    field.did_change("Ada".into()).unwrap();
    assert!(field.has_interacted_by_user());
    assert!(field.validate().unwrap());
    field.clear_error();
    assert!(!field.has_interacted_by_user());
    field.reset().unwrap();
    assert_eq!(field.current_value(), "");
    assert_eq!(field.error_text(), None);
}
#[test]
fn mixed_types_granular_validation_visits_every_field_and_guards_own_membership() {
    let form = FormController::new();
    let name = FormFieldController::new(String::new());
    let age = FormFieldController::new(10);
    name.set_validator(Some(Rc::new(|s| s.is_empty().then(|| "Required".into()))));
    age.set_validator(Some(Rc::new(|n| (*n < 18).then(|| "Adult only".into()))));
    let a = form.register(&name);
    let b = form.register(&age);
    let duplicate = form.register(&name);
    assert_eq!(a.id(), duplicate.id());
    assert_eq!(form.field_count(), 2);
    assert_eq!(form.validate_granularly().unwrap(), vec![a.id(), b.id()]);
    drop(duplicate);
    assert_eq!(form.field_count(), 2);
    name.did_change("Ada".into()).unwrap();
    age.did_change(20).unwrap();
    assert!(form.validate().unwrap());
    form.reset().unwrap();
    assert_eq!(age.current_value(), 10);
    assert_eq!(name.current_value(), "");
    form.clear_errors().unwrap();
    drop(a);
    assert_eq!(form.field_count(), 1);
    drop(b);
    assert_eq!(form.field_count(), 0);
}
#[test]
fn registrations_do_not_retain_fields_or_their_form() {
    let form = FormController::new();
    let field = FormFieldController::new(1);
    let weak = field.downgrade();
    let registration = form.register(&field);
    drop(field);
    assert!(weak.upgrade().is_none());
    assert_eq!(form.field_count(), 0);
    assert!(form.validate().unwrap());
    drop(form);
    drop(registration);
}
#[test]
fn recursive_and_superseded_validation_never_publishes_stale_errors() {
    let field = FormFieldController::new(1);
    let weak = field.downgrade();
    field.set_validator(Some(Rc::new(move |_| {
        assert_eq!(
            weak.upgrade().unwrap().validate(),
            Err(FormError::Reentrant)
        );
        None
    })));
    assert!(field.validate().unwrap());
    let weak = field.downgrade();
    field.set_validator(Some(Rc::new(move |_| {
        weak.upgrade().unwrap().did_change(2).unwrap();
        Some("stale".into())
    })));
    assert_eq!(field.validate(), Err(FormError::Superseded));
    assert_eq!(field.current_value(), 2);
    assert_eq!(field.error_text(), None);
}
#[test]
fn callback_panics_are_typed_visible_and_do_not_poison_validation_guard() {
    let field = FormFieldController::new(1);
    field.set_validator(Some(Rc::new(|_| panic!("validator failure"))));
    assert_eq!(
        field.validate(),
        Err(FormError::CallbackPanicked("validator"))
    );
    assert_eq!(
        field.snapshot().validation_failure,
        Some(FormError::CallbackPanicked("validator"))
    );
    field.set_validator(None);
    assert!(field.validate().unwrap());
    assert_eq!(field.snapshot().validation_failure, None);
}
#[test]
fn later_validator_editing_an_earlier_field_supersedes_form_result() {
    let form = FormController::new();
    let first = FormFieldController::new(1);
    let second = FormFieldController::new(2);
    let _a = form.register(&first);
    let _b = form.register(&second);
    let weak = first.downgrade();
    second.set_validator(Some(Rc::new(move |_| {
        weak.upgrade().unwrap().did_change(3).unwrap();
        None
    })));
    assert_eq!(form.validate(), Err(FormError::Superseded));
}
#[test]
fn removing_a_registration_during_validation_is_safe_and_supersedes_membership() {
    let form = FormController::new();
    let first = FormFieldController::new(1);
    let second = FormFieldController::new(2);
    let _a = form.register(&first);
    let b = Rc::new(RefCell::new(Some(form.register(&second))));
    let captured = b.clone();
    first.set_validator(Some(Rc::new(move |_| {
        captured.borrow_mut().take();
        None
    })));
    assert_eq!(form.validate(), Err(FormError::Superseded));
    assert_eq!(form.field_count(), 1);
}
#[test]
fn synchronous_observers_can_inspect_fields_and_form_recursion_is_rejected() {
    let form = FormController::new();
    let field = FormFieldController::new(1);
    let _member = form.register(&field);
    let calls = Rc::new(Cell::new(0));
    let n = calls.clone();
    let f = form.clone();
    let weak = field.downgrade();
    let _listener = form
        .subscribe(Rc::new(move || {
            let _ = weak.upgrade().unwrap().value();
            n.set(n.get() + 1);
            assert_eq!(f.validate(), Err(FormError::Reentrant));
        }))
        .unwrap();
    assert!(form.validate().unwrap());
    assert_eq!(calls.get(), 1);
}
fn captured_field(
    initial: i32,
    slot: Rc<RefCell<Option<FormFieldController<i32>>>>,
) -> FormField<i32> {
    FormField::new(initial, move |_, field| {
        *slot.borrow_mut() = Some(field);
        Widget::empty()
    })
}
#[test]
fn retained_fields_use_nearest_form_and_rebind_without_losing_values() {
    let ctx = egui::Context::default();
    let outer = FormController::new();
    let inner = FormController::new();
    let next = FormController::new();
    let slot = Rc::new(RefCell::new(None));
    let root = |form: &FormController| {
        Form::new(outer.clone()).into_widget(
            Form::new(form.clone())
                .into_widget(captured_field(1, slot.clone()).into_widget().keyed("field")),
        )
    };
    let mut tree = ElementTree::new(&ctx, root(&inner)).unwrap();
    assert_eq!(outer.field_count(), 0);
    assert_eq!(inner.field_count(), 1);
    let handle = slot.borrow().clone().unwrap();
    handle.did_change(9).unwrap();
    tree.flush().unwrap();
    tree.update(root(&next)).unwrap();
    assert_eq!(inner.field_count(), 0);
    assert_eq!(next.field_count(), 1);
    assert!(slot.borrow().as_ref().unwrap().same_source(&handle));
    assert_eq!(handle.current_value(), 9);
    tree.unmount();
    assert_eq!(next.field_count(), 0);
    handle.did_change(10).unwrap();
}
#[test]
fn property_updates_preserve_value_and_reset_uses_latest_initial_and_callbacks() {
    let ctx = egui::Context::default();
    let form = FormController::new();
    let slot = Rc::new(RefCell::new(None));
    let saved = Rc::new(Cell::new(0));
    let reset = Rc::new(Cell::new(0));
    let root = |initial| {
        let save = saved.clone();
        let reset = reset.clone();
        Form::new(form.clone()).into_widget(
            captured_field(initial, slot.clone())
                .on_saved(move |v| save.set(*v))
                .on_reset(move || reset.set(reset.get() + 1))
                .into_widget(),
        )
    };
    let mut tree = ElementTree::new(&ctx, root(1)).unwrap();
    let handle = slot.borrow().clone().unwrap();
    handle.did_change(7).unwrap();
    form.save().unwrap();
    assert_eq!(saved.get(), 7);
    tree.update(root(2)).unwrap();
    assert_eq!(handle.current_value(), 7);
    form.reset().unwrap();
    assert_eq!(handle.current_value(), 2);
    assert_eq!(reset.get(), 1);
}
#[test]
fn all_field_auto_modes_and_disabled_gate_work_with_explicit_unfocus_adapter() {
    for mode in [
        AutovalidateMode::Disabled,
        AutovalidateMode::Always,
        AutovalidateMode::OnUserInteraction,
        AutovalidateMode::OnUnfocus,
        AutovalidateMode::OnUserInteractionIfError,
    ] {
        let slot = Rc::new(RefCell::new(None));
        let mut widget =
            captured_field(0, slot.clone()).validator(|v| (*v == 0).then(|| "Required".into()));
        widget.autovalidate_mode = mode;
        let mut tree = ElementTree::new(&egui::Context::default(), widget.into_widget()).unwrap();
        let handle = slot.borrow().clone().unwrap();
        assert_eq!(
            handle.error_text().is_some(),
            mode == AutovalidateMode::Always
        );
        handle.did_change(0).unwrap();
        tree.flush().unwrap();
        assert_eq!(
            handle.error_text().is_some(),
            matches!(
                mode,
                AutovalidateMode::Always | AutovalidateMode::OnUserInteraction
            )
        );
        handle.did_unfocus().unwrap();
        assert_eq!(
            handle.error_text().is_some(),
            matches!(
                mode,
                AutovalidateMode::Always
                    | AutovalidateMode::OnUserInteraction
                    | AutovalidateMode::OnUnfocus
            )
        );
        handle.validate().unwrap();
        handle.did_change(1).unwrap();
        if mode == AutovalidateMode::OnUserInteractionIfError {
            assert_eq!(handle.error_text(), None);
        }
    }
    let slot = Rc::new(RefCell::new(None));
    let mut widget = captured_field(0, slot.clone()).validator(|_| Some("bad".into()));
    widget.enabled = false;
    widget.autovalidate_mode = AutovalidateMode::Always;
    let _tree = ElementTree::new(&egui::Context::default(), widget.into_widget()).unwrap();
    let field = slot.borrow().clone().unwrap();
    assert_eq!(field.error_text(), None);
    assert!(!field.validate().unwrap());
}
#[test]
fn forced_error_skips_validator_and_save_reset_panics_are_typed() {
    let slot = Rc::new(RefCell::new(None));
    let mut widget =
        captured_field(0, slot.clone()).validator(|_| panic!("must skip forced validator"));
    widget.force_error_text = Some("server rejected".into());
    let _tree = ElementTree::new(
        &egui::Context::default(),
        widget
            .on_saved(|_| panic!("save"))
            .on_reset(|| panic!("reset"))
            .into_widget(),
    )
    .unwrap();
    let field = slot.borrow().clone().unwrap();
    assert!(!field.validate().unwrap());
    assert_eq!(field.error_text().as_deref(), Some("server rejected"));
    assert_eq!(field.save(), Err(FormError::CallbackPanicked("save")));
    assert_eq!(field.reset(), Err(FormError::CallbackPanicked("reset")));
    assert!(!field.is_valid().unwrap());
}

#[test]
fn always_auto_validation_reappears_after_clear_without_rebuild_loops() {
    let slot = Rc::new(RefCell::new(None));
    let mut widget = captured_field(0, slot.clone()).validator(|_| Some("Required".into()));
    widget.autovalidate_mode = AutovalidateMode::Always;
    let mut tree = ElementTree::new(&egui::Context::default(), widget.into_widget()).unwrap();
    let field = slot.borrow().clone().unwrap();
    field.clear_error();
    tree.flush().unwrap();
    assert_eq!(field.error_text().as_deref(), Some("Required"));
    assert!(
        (0..8).any(|_| !tree.flush().unwrap()),
        "automatic validation did not settle"
    );
}
#[test]
fn real_text_input_updates_retained_form_value_and_validity() {
    let ctx = egui::Context::default();
    let slot = Rc::new(RefCell::new(None));
    let capture = slot.clone();
    let bounds = Rc::new(Cell::new(egui::Rect::NOTHING));
    let rect = bounds.clone();
    let form = FormController::new();
    let widget = FormField::new(String::new(), move |_, field| {
        *capture.borrow_mut() = Some(field.clone());
        let rect = rect.clone();
        Widget::leaf(egui::vec2(240.0, 40.0), move |ui| {
            let mut value = field.current_value();
            let response = ui.add(egui::TextEdit::singleline(&mut value));
            rect.set(response.rect);
            if response.changed() {
                field.did_change(value).unwrap();
            }
        })
    })
    .validator(|value| value.is_empty().then(|| "Required".into()));
    let mut tree = ElementTree::new(
        &ctx,
        Form::new(form.clone()).into_widget(widget.into_widget()),
    )
    .unwrap();
    let mut frame = |events| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 200.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    tree.show(
                        ui,
                        rust_flutter::layout::BoxConstraints::tight(egui::vec2(240.0, 40.0))
                            .unwrap(),
                    )
                    .unwrap();
                });
            },
        );
    };
    frame(vec![]);
    frame(vec![]);
    let pos = bounds.get().center();
    for pressed in [true, false] {
        frame(vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
    }
    frame(vec![egui::Event::Text("Ada".into())]);
    frame(vec![]);
    assert_eq!(slot.borrow().as_ref().unwrap().current_value(), "Ada");
    assert!(form.validate().unwrap());
    form.reset().unwrap();
    assert_eq!(slot.borrow().as_ref().unwrap().current_value(), "");
}

#[test]
fn automatic_validator_failure_is_visible_and_settles() {
    let slot = Rc::new(RefCell::new(None));
    let mut widget = captured_field(0, slot.clone()).validator(|_| panic!("automatic validator"));
    widget.autovalidate_mode = AutovalidateMode::Always;
    let mut tree = ElementTree::new(&egui::Context::default(), widget.into_widget()).unwrap();
    assert_eq!(
        slot.borrow()
            .as_ref()
            .unwrap()
            .snapshot()
            .validation_failure,
        Some(FormError::CallbackPanicked("validator"))
    );
    assert!(
        (0..8).any(|_| !tree.flush().unwrap()),
        "automatic callback failure did not settle"
    );
}

#[test]
fn text_editing_values_validate_utf16_ranges_atomically_and_notify_once() {
    let controller = TextEditingController::from_value(TextEditingValue {
        text: "A😀é".into(),
        selection: Some(TextSelection {
            base_offset: 1,
            extent_offset: 3,
            affinity: TextAffinity::Downstream,
        }),
        composing: Some(TextRange { start: 3, end: 4 }),
    })
    .unwrap();
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    let _subscription = controller
        .subscribe(Rc::new(move || seen.set(seen.get() + 1)))
        .unwrap();
    assert_eq!(controller.value().selection.unwrap().extent_offset, 3);
    assert_eq!(
        controller.set_selection(Some(TextSelection::collapsed(2))),
        Err(TextEditingError::InvalidOffset)
    );
    assert_eq!(controller.text(), "A😀é");
    assert_eq!(calls.get(), 0);
    controller
        .set_selection(Some(TextSelection::collapsed(4)))
        .unwrap();
    assert_eq!(calls.get(), 1);
    assert!(controller.set_text("new").unwrap());
    assert_eq!(controller.value().selection, None);
    assert_eq!(controller.value().composing, None);
    controller.dispose().unwrap();
    assert_eq!(controller.set_text("late"), Err(TextEditingError::Disposed));
}

#[test]
fn text_form_field_keeps_controller_and_form_value_in_sync_across_reset() {
    let context = egui::Context::default();
    let controller = TextEditingController::new("");
    let form = FormController::new();
    let saved = Rc::new(RefCell::new(String::new()));
    let result = saved.clone();
    let widget = TextFormField::with_controller(controller.clone())
        .validator(|s| s.trim().is_empty().then(|| "Required".into()))
        .on_saved(move |s| *result.borrow_mut() = s.to_owned())
        .into_widget()
        .unwrap();
    let mut tree = ElementTree::new(&context, Form::new(form.clone()).into_widget(widget)).unwrap();
    let frame = |tree: &mut ElementTree, ctx: &egui::Context| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(320.0, 160.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    tree.show(ui, BoxConstraints::tight(egui::vec2(320.0, 160.0)).unwrap())
                        .unwrap();
                });
            },
        );
    };
    frame(&mut tree, &context);
    controller.set_text("Ada").unwrap();
    assert!(form.validate().unwrap());
    form.save().unwrap();
    assert_eq!(saved.borrow().as_str(), "Ada");
    form.reset().unwrap();
    assert_eq!(controller.text(), "");
    controller.set_text("Grace").unwrap();
    assert!(form.validate().unwrap());
}
