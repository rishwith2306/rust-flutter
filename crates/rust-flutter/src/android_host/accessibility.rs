//! AccessKit bridge for the NativeActivity host.
use accesskit_android::InjectingAdapter;
use accesskit_android::jni::{JavaVM, objects::JObject};
use android_activity::{AndroidApp, AndroidAppWaker};
use egui::{self, Event, accesskit};
use std::{
    mem::ManuallyDrop,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
struct CachedTree {
    nodes: Vec<(accesskit::NodeId, accesskit::Node)>,
    tree: Option<accesskit::Tree>,
    focus: accesskit::NodeId,
}

impl CachedTree {
    fn capture(update: &accesskit::TreeUpdate) -> Self {
        Self {
            nodes: update.nodes.clone(),
            tree: update.tree.clone(),
            focus: update.focus,
        }
    }

    fn update(&self) -> accesskit::TreeUpdate {
        accesskit::TreeUpdate {
            nodes: self.nodes.clone(),
            tree: self.tree.clone(),
            focus: self.focus,
        }
    }
}

struct TreeActivation(Arc<Mutex<Option<CachedTree>>>);

impl accesskit::ActivationHandler for TreeActivation {
    fn request_initial_tree(&mut self) -> Option<accesskit::TreeUpdate> {
        self.0.lock().unwrap().as_ref().map(CachedTree::update)
    }
}

struct ActionQueue {
    events: Arc<Mutex<Vec<Event>>>,
    wake: AndroidAppWaker,
}

impl accesskit::ActionHandler for ActionQueue {
    fn do_action(&mut self, request: accesskit::ActionRequest) {
        self.events
            .lock()
            .unwrap()
            .push(Event::AccessKitActionRequest(request));
        self.wake.wake();
    }
}

/// Owns the Java accessibility delegate and the latest complete egui tree.
pub(super) struct AndroidAccessibility {
    adapter: InjectingAdapter,
    tree: Arc<Mutex<Option<CachedTree>>>,
    events: Arc<Mutex<Vec<Event>>>,
}

impl AndroidAccessibility {
    pub(super) fn new(activity: &AndroidApp, wake: AndroidAppWaker) -> Result<Self, String> {
        let tree = Arc::new(Mutex::new(None));
        let events = Arc::new(Mutex::new(Vec::new()));
        // SAFETY: AndroidApp retains the JavaVM and Activity for this host's lifetime.
        let vm = unsafe { JavaVM::from_raw(activity.vm_as_ptr().cast()) }
            .map_err(|error| format!("could not get Android JavaVM: {error}"))?;
        let mut env = vm
            .attach_current_thread()
            .map_err(|error| format!("could not attach Android accessibility thread: {error}"))?;
        let raw_activity = activity.activity_as_ptr() as accesskit_android::jni::sys::jobject;
        // SAFETY: AndroidApp owns this global Activity reference. ManuallyDrop prevents
        // treating the borrowed global reference as a JNI local reference on drop.
        let activity_global = ManuallyDrop::new(unsafe { JObject::from_raw(raw_activity) });
        let activity = env
            .new_local_ref(&*activity_global)
            .map_err(|error| format!("could not reference Android Activity: {error}"))?;
        let window = env
            .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
            .and_then(|value| value.l())
            .map_err(|error| format!("could not access Android window: {error}"))?;
        let decor_view = env
            .call_method(&window, "getDecorView", "()Landroid/view/View;", &[])
            .and_then(|value| value.l())
            .map_err(|error| format!("could not access Android decor view: {error}"))?;
        let adapter = InjectingAdapter::new(
            &mut *env,
            &decor_view,
            TreeActivation(tree.clone()),
            ActionQueue {
                events: events.clone(),
                wake,
            },
        );
        Ok(Self {
            adapter,
            tree,
            events,
        })
    }

    pub(super) fn update(&mut self, update: accesskit::TreeUpdate) {
        *self.tree.lock().unwrap() = Some(CachedTree::capture(&update));
        self.adapter.update_if_active(|| update);
    }

    pub(super) fn take_events(&self) -> Vec<Event> {
        std::mem::take(&mut *self.events.lock().unwrap())
    }
}
