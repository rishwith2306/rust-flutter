use rust_flutter::{egui, element::*, forms::*, notifier::Listenable};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn required() -> FormFieldController<String> {
    let field = FormFieldController::new(String::new());
    field.set_validator(Some(Rc::new(|v| v.is_empty().then(|| "Required".into()))));
    field
}
#[test]
fn an_edit_revalidates_related_fields_after_interaction() {
    let form = FormController::new();
    form.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
    let password = required();
    let confirmation = FormFieldController::new(String::new());
    let weak = password.downgrade();
    confirmation.set_validator(Some(Rc::new(move |v| {
        (v != &weak.upgrade().unwrap().current_value()).then(|| "Does not match".into())
    })));
    let _a = form.register(&password);
    let _b = form.register(&confirmation);
    assert_eq!(password.error_text(), None);
    assert_eq!(confirmation.error_text(), None);
    password.did_change("first".into()).unwrap();
    assert!(form.has_interacted_by_user());
    assert_eq!(confirmation.error_text().as_deref(), Some("Does not match"));
    confirmation.did_change("first".into()).unwrap();
    assert_eq!(confirmation.error_text(), None);
    password.did_change("second".into()).unwrap();
    assert_eq!(confirmation.error_text().as_deref(), Some("Does not match"));
}
#[test]
fn always_validates_on_registration_and_error_notifications_do_not_revalidate() {
    let form = FormController::new();
    form.set_autovalidate_mode(AutovalidateMode::Always);
    let field = required();
    let calls = Rc::new(Cell::new(0));
    let n = calls.clone();
    field.set_validator(Some(Rc::new(move |_| {
        n.set(n.get() + 1);
        Some("Required".into())
    })));
    let _a = form.register(&field);
    let _duplicate = form.register(&field);
    assert_eq!(calls.get(), 1);
    assert_eq!(field.error_text().as_deref(), Some("Required"));
    field.did_change("edit".into()).unwrap();
    assert_eq!(calls.get(), 2);
    field.validate().unwrap();
    assert_eq!(calls.get(), 3);
    form.flush_validation().unwrap();
    assert_eq!(calls.get(), 3);
}
#[test]
fn form_reset_waits_for_all_initial_values_before_automatic_validation() {
    let form = FormController::new();
    let first = FormFieldController::new(1);
    let second = FormFieldController::new(1);
    let weak = first.downgrade();
    let pairs = Rc::new(RefCell::new(vec![]));
    let record = pairs.clone();
    second.set_validator(Some(Rc::new(move |v| {
        let a = weak.upgrade().unwrap().current_value();
        record.borrow_mut().push((a, *v));
        (a != *v).then(|| "Mismatch".into())
    })));
    let _a = form.register(&first);
    let _b = form.register(&second);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    first.did_change(2).unwrap();
    second.did_change(2).unwrap();
    pairs.borrow_mut().clear();
    form.reset().unwrap();
    assert_eq!(&*pairs.borrow(), &[(1, 1)]);
    assert_eq!(second.error_text(), None);
    assert!(!form.has_interacted_by_user());
}
#[test]
fn interaction_mode_reset_and_clear_leave_pristine_fields_until_another_edit() {
    let form = FormController::new();
    let field = required();
    let _member = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
    field.did_change(String::new()).unwrap();
    assert!(field.error_text().is_some());
    form.reset().unwrap();
    assert_eq!(field.error_text(), None);
    assert!(!form.has_interacted_by_user());
    field.did_change(String::new()).unwrap();
    form.clear_errors().unwrap();
    assert_eq!(field.error_text(), None);
    assert!(!form.has_interacted_by_user());
    field.did_change(String::new()).unwrap();
    assert!(field.error_text().is_some());
}
#[test]
fn always_republishes_errors_after_clear_and_switching_to_disabled_stops_auto() {
    let form = FormController::new();
    let field = required();
    let _member = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    form.clear_errors().unwrap();
    assert!(field.error_text().is_some());
    form.set_autovalidate_mode(AutovalidateMode::Disabled);
    form.clear_errors().unwrap();
    field.did_change(String::new()).unwrap();
    assert_eq!(field.error_text(), None);
}
#[test]
fn error_only_mode_starts_with_published_errors_and_stops_when_all_clear() {
    let form = FormController::new();
    let field = required();
    let _member = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::OnUserInteractionIfError);
    field.did_change(String::new()).unwrap();
    assert_eq!(field.error_text(), None);
    assert!(!form.validate().unwrap());
    field.did_change("Ada".into()).unwrap();
    assert_eq!(field.error_text(), None);
    field.did_change(String::new()).unwrap();
    assert_eq!(field.error_text(), None);
    assert!(!form.validate().unwrap());
}
#[test]
fn unfocus_validates_only_the_departing_field_even_without_an_edit() {
    let form = FormController::new();
    let first = required();
    let second = required();
    let _a = form.register(&first);
    let _b = form.register(&second);
    form.set_autovalidate_mode(AutovalidateMode::OnUnfocus);
    first.did_change(String::new()).unwrap();
    assert_eq!(first.error_text(), None);
    first.did_unfocus().unwrap();
    assert!(first.error_text().is_some());
    assert_eq!(second.error_text(), None);
    second.did_unfocus().unwrap();
    assert!(second.error_text().is_some());
}
#[test]
fn a_manual_validation_latches_interaction_for_new_members() {
    let form = FormController::new();
    let first = required();
    let _a = form.register(&first);
    assert!(!form.validate().unwrap());
    form.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
    let second = required();
    let _b = form.register(&second);
    assert!(form.has_interacted_by_user());
    assert!(second.error_text().is_some());
}
#[test]
fn nested_scope_policies_are_independent_and_disabled_fields_skip_automatic_passes() {
    let outer = FormController::new();
    let inner = FormController::new();
    outer.set_autovalidate_mode(AutovalidateMode::Always);
    inner.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
    let slot = Rc::new(RefCell::new(None));
    let capture = slot.clone();
    let mut field = FormField::new(String::new(), move |_, field| {
        *capture.borrow_mut() = Some(field);
        Widget::empty()
    })
    .validator(|_| Some("Required".into()));
    field.enabled = false;
    let mut wrapper = Form::new(inner.clone());
    wrapper.autovalidate_mode = Some(AutovalidateMode::Always);
    let _tree = ElementTree::new(
        &egui::Context::default(),
        Form::new(outer.clone()).into_widget(wrapper.into_widget(field.into_widget())),
    )
    .unwrap();
    assert_eq!(outer.field_count(), 0);
    assert_eq!(inner.field_count(), 1);
    let field: FormFieldController<String> = slot.borrow().clone().unwrap();
    assert_eq!(field.error_text(), None);
    field.did_unfocus().unwrap();
    assert_eq!(field.error_text(), None);
    assert!(!inner.validate().unwrap());
}
#[test]
fn automatic_callback_failure_is_observable_and_configuration_can_recover() {
    let form = FormController::new();
    let field = required();
    let _member = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    field.set_validator(Some(Rc::new(|_| panic!("automatic group callback"))));
    assert_eq!(
        form.last_auto_failure(),
        Some(FormError::CallbackPanicked("validator"))
    );
    field.set_validator(None);
    assert_eq!(form.last_auto_failure(), None);
    assert_eq!(field.error_text(), None);
}
#[test]
fn reentrant_edits_are_deferred_without_a_synchronous_retry_loop() {
    let form = FormController::new();
    let first = FormFieldController::new(1);
    let second = FormFieldController::new(1);
    let _a = form.register(&first);
    let _b = form.register(&second);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    let weak = first.downgrade();
    let calls = Rc::new(Cell::new(0));
    let n = calls.clone();
    second.set_validator(Some(Rc::new(move |_| {
        let called = n.get();
        n.set(called + 1);
        if called == 0 {
            weak.upgrade().unwrap().did_change(2).unwrap();
        }
        None
    })));
    assert_eq!(calls.get(), 1);
    assert_eq!(form.last_auto_failure(), Some(FormError::Superseded));
    assert_eq!(first.current_value(), 2);
    form.flush_validation().unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(form.last_auto_failure(), None);
}
#[test]
fn automatic_validation_detects_membership_changes_and_recovers_on_explicit_flush() {
    let form = FormController::new();
    let first = required();
    let second = required();
    let _a = form.register(&first);
    let b = Rc::new(RefCell::new(Some(form.register(&second))));
    form.set_autovalidate_mode(AutovalidateMode::Always);
    let remove = b.clone();
    first.set_validator(Some(Rc::new(move |_| {
        remove.borrow_mut().take();
        None
    })));
    assert_eq!(form.field_count(), 1);
    assert_eq!(form.last_auto_failure(), Some(FormError::Superseded));
    form.flush_validation().unwrap();
    assert_eq!(form.last_auto_failure(), None);
}
#[test]
fn recursive_explicit_commands_from_an_automatic_validator_are_rejected() {
    let form = FormController::new();
    let field = required();
    let _a = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    let copy = form.clone();
    field.set_validator(Some(Rc::new(move |_| {
        assert_eq!(copy.validate(), Err(FormError::Reentrant));
        assert_eq!(copy.reset(), Err(FormError::Reentrant));
        assert_eq!(copy.flush_validation(), Err(FormError::Reentrant));
        None
    })));
    assert_eq!(form.last_auto_failure(), None);
    assert_eq!(field.error_text(), None);
    // Release the callback's explicit form capture before testing teardown.
    field.set_validator(None);
}
#[test]
fn observers_receive_results_without_becoming_automatic_edit_events() {
    let form = FormController::new();
    let field = required();
    let _member = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    let events = Rc::new(Cell::new(0));
    let n = events.clone();
    let _observer = form.subscribe(Rc::new(move || n.set(n.get() + 1))).unwrap();
    field.did_change("Ada".into()).unwrap();
    assert!(events.get() > 0);
    let before = events.get();
    form.flush_validation().unwrap();
    assert_eq!(events.get(), before);
}

#[test]
fn changing_policy_during_an_automatic_pass_supersedes_it_without_retrying() {
    let form = FormController::new();
    let field = required();
    let _member = form.register(&field);
    form.set_autovalidate_mode(AutovalidateMode::Always);
    let change = form.clone();
    field.set_validator(Some(Rc::new(move |_| {
        change.set_autovalidate_mode(AutovalidateMode::Disabled);
        None
    })));
    assert_eq!(form.autovalidate_mode(), AutovalidateMode::Disabled);
    assert_eq!(form.last_auto_failure(), Some(FormError::Superseded));
    field.set_validator(None);
}
#[test]
fn unfocus_pass_detects_changes_to_a_related_field_outside_its_targets() {
    let form = FormController::new();
    let first = FormFieldController::new(1);
    let other = FormFieldController::new(1);
    let _a = form.register(&first);
    let _b = form.register(&other);
    form.set_autovalidate_mode(AutovalidateMode::OnUnfocus);
    let weak = other.downgrade();
    first.set_validator(Some(Rc::new(move |_| {
        weak.upgrade().unwrap().did_change(2).unwrap();
        None
    })));
    first.did_unfocus().unwrap();
    assert_eq!(other.current_value(), 2);
    assert_eq!(form.last_auto_failure(), Some(FormError::Superseded));
}

#[test]
fn reset_batches_field_level_always_checks_even_without_a_group_always_policy() {
    let form = FormController::new();
    form.set_autovalidate_mode(AutovalidateMode::OnUserInteraction);
    let first_slot: Rc<RefCell<Option<FormFieldController<i32>>>> = Rc::new(RefCell::new(None));
    let second_slot: Rc<RefCell<Option<FormFieldController<i32>>>> = Rc::new(RefCell::new(None));
    let capture = first_slot.clone();
    let peer = second_slot.clone();
    let values = Rc::new(RefCell::new(vec![]));
    let recorded = values.clone();
    let mut first = FormField::new(1, move |_, field| {
        *capture.borrow_mut() = Some(field);
        Widget::empty()
    })
    .validator(move |a| {
        peer.borrow().as_ref().and_then(|b| {
            let b = b.current_value();
            recorded.borrow_mut().push((*a, b));
            (*a != b).then(|| "Mismatch".into())
        })
    });
    first.autovalidate_mode = AutovalidateMode::Always;
    let capture = second_slot.clone();
    let second = FormField::new(1, move |_, field| {
        *capture.borrow_mut() = Some(field);
        Widget::empty()
    });
    let mut tree = ElementTree::new(
        &egui::Context::default(),
        Form::new(form.clone()).into_widget(Widget::flex(
            FlexStyle::new(rust_flutter::layout::Axis::Vertical),
            vec![
                FlexItem::fixed(first.into_widget()),
                FlexItem::fixed(second.into_widget()),
            ],
        )),
    )
    .unwrap();
    let a = first_slot.borrow().clone().unwrap();
    let b = second_slot.borrow().clone().unwrap();
    a.did_change(2).unwrap();
    b.did_change(2).unwrap();
    tree.flush().unwrap();
    values.borrow_mut().clear();
    form.reset().unwrap();
    assert_eq!(&*values.borrow(), &[(1, 1)]);
    assert_eq!(a.error_text(), None);
    assert_eq!(b.current_value(), 1);
    assert!(!form.has_interacted_by_user());
}
