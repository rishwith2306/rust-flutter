use super::inherited::Dependencies;
use super::{ElementId, Widget};
use std::rc::Rc;
use std::{
    any::Any,
    marker::PhantomData,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

type Command = Box<dyn FnOnce(&mut dyn Any) + Send>;
pub(super) struct Queue {
    pub dirty: AtomicBool,
    mounted: AtomicBool,
    commands: Mutex<Vec<Command>>,
    context: Mutex<Option<egui::Context>>,
}
impl Queue {
    pub fn new(ctx: &egui::Context) -> Arc<Self> {
        Arc::new(Self {
            dirty: AtomicBool::new(true),
            mounted: AtomicBool::new(true),
            commands: Mutex::new(Vec::new()),
            context: Mutex::new(Some(ctx.clone())),
        })
    }
    pub fn mounted(&self) -> bool {
        self.mounted.load(Ordering::Acquire)
    }
    pub fn request_rebuild(&self) -> Result<(), Unmounted> {
        self.enqueue(None)
    }
    fn enqueue(&self, command: Option<Command>) -> Result<(), Unmounted> {
        let mut commands = self.commands.lock().unwrap();
        if !self.mounted() {
            return Err(Unmounted);
        }
        if let Some(command) = command {
            commands.push(command);
        }
        self.dirty.store(true, Ordering::Release);
        let context = self.context.lock().unwrap().clone();
        drop(commands);
        if let Some(ctx) = context {
            ctx.request_repaint();
        }
        Ok(())
    }
    pub fn drain(&self, state: &mut dyn Any) {
        let commands = std::mem::take(&mut *self.commands.lock().unwrap());
        for command in commands {
            command(state);
        }
    }
    pub fn close(&self) {
        let mut guard = self.commands.lock().unwrap();
        self.mounted.store(false, Ordering::Release);
        let commands = std::mem::take(&mut *guard);
        let context = self.context.lock().unwrap().take();
        drop(guard);
        // User-owned command captures may run destructors that inspect a handle.
        // Never drop those captures while holding the queue lock.
        drop(commands);
        drop(context);
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unmounted;
impl std::fmt::Display for Unmounted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("component is unmounted")
    }
}
impl std::error::Error for Unmounted {}

/// Sendable update handle. Commands execute against component state on the UI
/// thread during the next build pass; the state itself need not be Send.
pub struct StateHandle<S: 'static> {
    queue: Arc<Queue>,
    marker: PhantomData<fn() -> S>,
}
impl<S: 'static> Clone for StateHandle<S> {
    fn clone(&self) -> Self {
        Self {
            queue: self.queue.clone(),
            marker: PhantomData,
        }
    }
}
impl<S: 'static> StateHandle<S> {
    pub(super) fn new(queue: Arc<Queue>) -> Self {
        Self {
            queue,
            marker: PhantomData,
        }
    }
    pub fn mounted(&self) -> bool {
        self.queue.mounted()
    }
    pub fn update(&self, update: impl FnOnce(&mut S) + Send + 'static) -> Result<(), Unmounted> {
        self.queue.enqueue(Some(Box::new(move |state| {
            update(state.downcast_mut::<S>().expect("typed state handle"))
        })))
    }
    pub fn request_rebuild(&self) -> Result<(), Unmounted> {
        self.queue.enqueue(None)
    }
}
#[derive(Clone)]
pub struct BuildContext {
    pub(super) dependencies: Rc<Dependencies>,
    pub(super) id: ElementId,
    pub(super) queue: Arc<Queue>,
    pub(super) egui: egui::Context,
}
impl BuildContext {
    pub fn element_id(&self) -> ElementId {
        self.id
    }
    pub fn mounted(&self) -> bool {
        self.queue.mounted()
    }
    pub fn egui(&self) -> &egui::Context {
        &self.egui
    }
    pub fn request_rebuild(&self) -> Result<(), Unmounted> {
        self.queue.enqueue(None)
    }
}
pub struct StateContext<S: 'static> {
    pub(super) build: BuildContext,
    pub(super) handle: StateHandle<S>,
}
impl<S: 'static> StateContext<S> {
    pub fn handle(&self) -> StateHandle<S> {
        self.handle.clone()
    }
}
impl<S: 'static> std::ops::Deref for StateContext<S> {
    type Target = BuildContext;
    fn deref(&self) -> &BuildContext {
        &self.build
    }
}

pub trait StatefulWidget: Sized + 'static {
    type State: State<Self>;
    fn create_state(&self) -> Self::State;
}
pub trait State<W: StatefulWidget>: Sized + 'static {
    fn init_state(&mut self, _widget: &W, _ctx: &StateContext<Self>) {}
    fn did_change_dependencies(&mut self, _widget: &W, _ctx: &StateContext<Self>) {}
    fn did_update_widget(&mut self, _old: &W, _new: &W, _ctx: &StateContext<Self>) {}
    fn build(&mut self, widget: &W, ctx: &StateContext<Self>) -> Widget;
    fn dispose(&mut self) {}
}
pub trait StatelessWidget: 'static {
    fn build(&self, ctx: &BuildContext) -> Widget;
}

pub(super) trait Description {
    fn token(&self) -> std::any::TypeId;
    fn name(&self) -> &'static str;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
    fn mount(self: Box<Self>, context: BuildContext) -> Box<dyn Component>;
}
pub(super) trait Component {
    fn dependencies_changed(&mut self);
    fn update(&mut self, description: Box<dyn Any>);
    fn build(&mut self) -> Widget;
    fn apply_pending(&mut self);
    fn state(&self) -> Option<&dyn Any>;
}
pub(super) struct Stateful<W: StatefulWidget>(pub W);
struct StatefulInstance<W: StatefulWidget> {
    widget: W,
    state: W::State,
    context: StateContext<W::State>,
}
impl<W: StatefulWidget> Description for Stateful<W> {
    fn token(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn name(&self) -> &'static str {
        std::any::type_name::<W>()
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn mount(self: Box<Self>, context: BuildContext) -> Box<dyn Component> {
        let state = self.0.create_state();
        let handle = StateHandle::new(context.queue.clone());
        let mut instance = StatefulInstance {
            widget: self.0,
            state,
            context: StateContext {
                build: context,
                handle,
            },
        };
        let _phase = instance
            .context
            .dependencies
            .enter(super::inherited::Phase::Init);
        instance
            .state
            .init_state(&instance.widget, &instance.context);
        Box::new(instance)
    }
}
impl<W: StatefulWidget> Component for StatefulInstance<W> {
    fn dependencies_changed(&mut self) {
        self.state
            .did_change_dependencies(&self.widget, &self.context);
    }
    fn update(&mut self, description: Box<dyn Any>) {
        self.apply_pending();
        let next = *description
            .downcast::<W>()
            .expect("matching component type");
        let old = std::mem::replace(&mut self.widget, next);
        self.state
            .did_update_widget(&old, &self.widget, &self.context);
    }
    fn build(&mut self) -> Widget {
        self.state.build(&self.widget, &self.context)
    }
    fn apply_pending(&mut self) {
        self.context.queue.drain(&mut self.state);
    }
    fn state(&self) -> Option<&dyn Any> {
        Some(&self.state)
    }
}
impl<W: StatefulWidget> Drop for StatefulInstance<W> {
    fn drop(&mut self) {
        self.context.queue.close();
        self.state.dispose();
    }
}
pub(super) struct Stateless<W: StatelessWidget>(pub W);
struct StatelessInstance<W: StatelessWidget> {
    widget: W,
    context: BuildContext,
}
impl<W: StatelessWidget> Description for Stateless<W> {
    fn token(&self) -> std::any::TypeId {
        std::any::TypeId::of::<Self>()
    }
    fn name(&self) -> &'static str {
        std::any::type_name::<W>()
    }
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(self.0)
    }
    fn mount(self: Box<Self>, context: BuildContext) -> Box<dyn Component> {
        Box::new(StatelessInstance {
            widget: self.0,
            context,
        })
    }
}
impl<W: StatelessWidget> Component for StatelessInstance<W> {
    fn dependencies_changed(&mut self) {}
    fn update(&mut self, description: Box<dyn Any>) {
        self.widget = *description
            .downcast::<W>()
            .expect("matching stateless type");
    }
    fn build(&mut self) -> Widget {
        self.widget.build(&self.context)
    }
    fn apply_pending(&mut self) {}
    fn state(&self) -> Option<&dyn Any> {
        None
    }
}
