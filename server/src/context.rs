//! Context provides a cancellation, similar to Golang's Context.

use std::{
    ops::Deref,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Context {
    inner: Arc<ContextInner>,
}

impl Context {
    /// Create a new Context.
    pub fn new() -> Self {
        Context {
            inner: Arc::new(ContextInner::new()),
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for Context {
    type Target = ContextInner;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

#[derive(Debug)]
pub struct ContextInner {
    cancelled: Mutex<bool>,
    cv: Condvar,
}

impl ContextInner {
    fn new() -> Self {
        ContextInner {
            cancelled: Mutex::new(false),
            cv: Condvar::new(),
        }
    }

    /// Cancel the context.
    pub fn cancel(&self) {
        let mut g = self.cancelled.lock().unwrap();
        *g = true;
        self.cv.notify_all();
    }

    /// Returns true iff the context has been cancelled.
    pub fn is_cancelled(&self) -> bool {
        *self.cancelled.lock().unwrap()
    }

    /// Wait until the context is cancelled.
    #[allow(unused)]
    pub fn wait(&self) {
        let g = self.cancelled.lock().unwrap();
        let v = self.cv.wait_while(g, |g| !*g).unwrap();
        std::mem::drop(v);
    }

    /// Wait until the duration expires, or the context is cancelled.
    /// Returns true if the context has been cancelled.
    pub fn wait_timeout(&self, duration: Duration) -> bool {
        let g = self.cancelled.lock().unwrap();
        let (v, _) = self.cv.wait_timeout_while(g, duration, |g| !*g).unwrap();
        *v
    }
}
