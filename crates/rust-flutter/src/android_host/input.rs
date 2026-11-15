use android_activity::{
    AndroidApp, InputStatus,
    input::{InputEvent, KeyAction, KeyMapChar, Keycode, MetaState, MotionAction},
};

#[derive(Default)]
pub(super) struct Input {
    pub events: Vec<egui::Event>,
    pub modifiers: egui::Modifiers,
    primary_touch: Option<i32>,
    accent: Option<char>,
}
impl Input {
    pub fn cancel(&mut self) {
        if self.primary_touch.take().is_some() {
            self.events.push(egui::Event::PointerGone);
            self.events.push(egui::Event::PointerButton {
                pos: egui::pos2(-1.0, -1.0),
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: self.modifiers,
            });
        }
        self.modifiers = egui::Modifiers::NONE;
        self.accent = None;
    }
    pub fn read(&mut self, app: &AndroidApp, event: &InputEvent<'_>, scale: f32) -> InputStatus {
        match event {
            InputEvent::MotionEvent(motion) => {
                self.modifiers = modifiers(motion.meta_state());
                let phase = match motion.action() {
                    MotionAction::Down | MotionAction::PointerDown => egui::TouchPhase::Start,
                    MotionAction::Up | MotionAction::PointerUp => egui::TouchPhase::End,
                    MotionAction::Move => egui::TouchPhase::Move,
                    MotionAction::Cancel => egui::TouchPhase::Cancel,
                    MotionAction::HoverMove | MotionAction::HoverEnter => {
                        let p = motion.pointer_at_index(0);
                        self.events.push(egui::Event::PointerMoved(egui::pos2(
                            p.x() / scale,
                            p.y() / scale,
                        )));
                        return InputStatus::Handled;
                    }
                    MotionAction::HoverExit => {
                        self.events.push(egui::Event::PointerGone);
                        return InputStatus::Handled;
                    }
                    MotionAction::Scroll => {
                        use android_activity::input::Axis;
                        self.events.push(egui::Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Line,
                            delta: egui::vec2(
                                motion.pointer_at_index(0).axis_value(Axis::Hscroll),
                                motion.pointer_at_index(0).axis_value(Axis::Vscroll),
                            ),
                            modifiers: self.modifiers,
                        });
                        return InputStatus::Handled;
                    }
                    _ => return InputStatus::Unhandled,
                };
                let changed = motion.pointer_index();
                for pointer in motion.pointers() {
                    if matches!(phase, egui::TouchPhase::Start | egui::TouchPhase::End)
                        && pointer.pointer_index() != changed
                    {
                        continue;
                    }
                    let id = pointer.pointer_id();
                    let pos = egui::pos2(pointer.x() / scale, pointer.y() / scale);
                    self.events.push(egui::Event::Touch {
                        device_id: egui::TouchDeviceId(motion.device_id() as u64),
                        id: egui::TouchId(id as u64),
                        phase,
                        pos,
                        force: Some(pointer.pressure()),
                    });
                    if phase == egui::TouchPhase::Start && self.primary_touch.is_none() {
                        self.primary_touch = Some(id);
                        self.events.push(egui::Event::PointerMoved(pos));
                        self.events.push(egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: self.modifiers,
                        });
                    } else if self.primary_touch == Some(id) {
                        match phase {
                            egui::TouchPhase::Move => {
                                self.events.push(egui::Event::PointerMoved(pos))
                            }
                            egui::TouchPhase::End => {
                                self.events.push(egui::Event::PointerButton {
                                    pos,
                                    button: egui::PointerButton::Primary,
                                    pressed: false,
                                    modifiers: self.modifiers,
                                });
                                self.events.push(egui::Event::PointerGone);
                                self.primary_touch = None;
                            }
                            egui::TouchPhase::Cancel => self.cancel(),
                            _ => {}
                        }
                    }
                }
                InputStatus::Handled
            }
            InputEvent::KeyEvent(key) => {
                if matches!(
                    key.key_code(),
                    Keycode::VolumeUp | Keycode::VolumeDown | Keycode::VolumeMute
                ) {
                    return InputStatus::Unhandled;
                }
                self.modifiers = modifiers(key.meta_state());
                let pressed = key.action() == KeyAction::Down;
                let number: u32 = key.key_code().into();
                if let Some(mapped) = keyboard_key(number) {
                    self.events.push(egui::Event::Key {
                        key: mapped,
                        physical_key: Some(mapped),
                        pressed,
                        repeat: key.repeat_count() > 0,
                        modifiers: self.modifiers,
                    });
                }
                if pressed
                    && !self.modifiers.ctrl
                    && !self.modifiers.mac_cmd
                    && key.device_id() != 0
                    && let Ok(map) = app.device_key_character_map(key.device_id())
                    && let Ok(character) = map.get(key.key_code(), key.meta_state())
                {
                    match character {
                        KeyMapChar::CombiningAccent(c) => self.accent = Some(c),
                        KeyMapChar::Unicode(c) if !c.is_control() => {
                            if let Some(accent) = self.accent.take() {
                                if let Ok(Some(combined)) = map.get_dead_char(accent, c) {
                                    self.events.push(egui::Event::Text(combined.to_string()));
                                } else {
                                    self.events.push(egui::Event::Text(format!("{accent}{c}")));
                                }
                            } else {
                                self.events.push(egui::Event::Text(c.to_string()));
                            }
                        }
                        _ => {}
                    }
                }
                InputStatus::Handled
            }
            // NativeActivity does not implement composition snapshots. GameActivity /
            // a Java text-input connection is needed for that separate integration.
            _ => InputStatus::Unhandled,
        }
    }
}
fn modifiers(meta: MetaState) -> egui::Modifiers {
    egui::Modifiers {
        alt: meta.alt_on(),
        ctrl: meta.ctrl_on(),
        shift: meta.shift_on(),
        mac_cmd: false,
        command: meta.ctrl_on(),
    }
}
fn keyboard_key(code: u32) -> Option<egui::Key> {
    use egui::Key;
    Some(match code {
        4 => Key::BrowserBack,
        19 => Key::ArrowUp,
        20 => Key::ArrowDown,
        21 => Key::ArrowLeft,
        22 => Key::ArrowRight,
        61 => Key::Tab,
        62 => Key::Space,
        66 | 160 => Key::Enter,
        67 => Key::Backspace,
        92 => Key::PageUp,
        93 => Key::PageDown,
        111 => Key::Escape,
        112 => Key::Delete,
        122 => Key::Home,
        123 => Key::End,
        124 => Key::Insert,
        29..=54 => Key::from_name(&((b'A' + (code - 29) as u8) as char).to_string())?,
        7..=16 => Key::from_name(&(code - 7).to_string())?,
        131..=142 => Key::from_name(&format!("F{}", code - 130))?,
        _ => return None,
    })
}
