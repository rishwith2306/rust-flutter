//! Typed route stacks. The root route cannot be popped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Navigator<R> {
    stack: Vec<R>,
}
impl<R> Navigator<R> {
    pub fn new(root: R) -> Self {
        Self { stack: vec![root] }
    }
    pub fn current(&self) -> &R {
        self.stack.last().expect("root route always exists")
    }
    pub fn push(&mut self, route: R) {
        self.stack.push(route);
    }
    pub fn pop(&mut self) -> Option<R> {
        (self.stack.len() > 1).then(|| self.stack.pop().unwrap())
    }
    pub fn replace(&mut self, route: R) -> R {
        std::mem::replace(self.stack.last_mut().unwrap(), route)
    }
    pub fn reset(&mut self, root: R) {
        self.stack.clear();
        self.stack.push(root);
    }
    pub fn can_pop(&self) -> bool {
        self.stack.len() > 1
    }
    pub fn routes(&self) -> &[R] {
        &self.stack
    }
}
