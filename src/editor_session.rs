use std::sync::{Arc, Mutex};

pub(crate) struct Store<T: Send> { inner: Mutex<State<T>> }
struct State<T> {
    generation: u64,
    next_lease: u64,
    active: Option<u64>,
    cached: Option<T>,
}

impl<T: Send> Default for Store<T> {
    fn default() -> Self {
        Self { inner: Mutex::new(State {
            generation: 1, next_lease: 1, active: None, cached: None,
        }) }
    }
}

impl<T: Send> Store<T> {
    pub(crate) fn claim(self: &Arc<Self>) -> Result<Lease<T>, String> {
        let mut state = self.inner.lock().map_err(|_| "Editor session storage failed.")?;
        if state.active.is_some() {
            return Err("This controller already has an attached editor.".into());
        }
        let id = state.next_lease;
        state.next_lease = id.checked_add(1).ok_or("Editor session identifiers exhausted.")?;
        state.active = Some(id);
        Ok(Lease { store: self.clone(), id, generation: state.generation, released: false })
    }

    pub(crate) fn invalidate(&self) -> Result<(), String> {
        let old = {
            let mut state = self.inner.lock().map_err(|_| "Editor session storage failed.")?;
            let next = state.generation.checked_add(1)
                .ok_or("Editor session generations exhausted.")?;
            state.generation = next;
            state.cached.take()
        };

        drop(old);
        Ok(())
    }
}

pub(crate) struct Lease<T: Send> {
    store: Arc<Store<T>>,
    id: u64,
    generation: u64,
    released: bool,
}

impl<T: Send> Lease<T> {
    pub(crate) fn bound_generation(&self) -> u64 { self.generation }

    pub(crate) fn current_generation(&self) -> Result<u64, String> {
        let state = self.store.inner.lock().map_err(|_| "Editor session storage failed.")?;
        if state.active != Some(self.id) { return Err("Editor session lease was lost.".into()); }
        Ok(state.generation)
    }

    pub(crate) fn accept_generation(&mut self, expected: u64) -> Result<(), String> {
        if self.current_generation()? != expected {
            return Err("The host replaced editor state during restoration.".into());
        }
        self.generation = expected;
        Ok(())
    }

    pub(crate) fn take(&mut self) -> Result<Option<T>, String> {
        let mut state = self.store.inner.lock().map_err(|_| "Editor session storage failed.")?;
        if state.active != Some(self.id) || state.generation != self.generation {
            return Err("The host replaced the retained editor session.".into());
        }
        Ok(state.cached.take())
    }

    pub(crate) fn finish(mut self, value: Option<T>) {
        self.release(value);
    }

    fn release(&mut self, value: Option<T>) {
        let mut rejected = value;
        let mut replaced = None;
        if let Ok(mut state) = self.store.inner.lock() {
            if state.active == Some(self.id) {
                state.active = None;
                if state.generation == self.generation && rejected.is_some() {
                    replaced = std::mem::replace(&mut state.cached, rejected.take());
                }
            }
        }
        self.released = true;
        drop(replaced);
        drop(rejected);
    }
}

impl<T: Send> Drop for Lease<T> {
    fn drop(&mut self) {
        if !self.released { self.release(None); }
    }
}

