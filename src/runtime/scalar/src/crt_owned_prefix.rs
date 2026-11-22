use super::crt_storage_native_return::SourceCrtNativeRetFrame;

#[must_use]
pub struct SourceCrtOwnedPrefix<C, P> {
    pub owner: C,
    progress: P,
}
impl<C, P: Copy> SourceCrtOwnedPrefix<C, P> {
    pub fn progress(&self) -> P { self.progress }
    pub(super) fn admitted(owner: C, progress: P) -> Self { Self { owner, progress } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtOwnedMemoryObservation<R> {
    pub receipt: R,
    pub actual_frame: SourceCrtNativeRetFrame,
}
#[must_use]
pub struct SourceCrtOwnedMemoryStep<C, A, R, P> {
    pub previous: SourceCrtOwnedPrefix<C, P>,
    pub actual: A,
    pub completed: SourceCrtOwnedMemoryObservation<R>,
}
#[must_use]
pub struct SourceCrtOwnedMemoryError<C, A, R, P, E> {
    pub previous: SourceCrtOwnedPrefix<C, P>,
    pub actual: A,
    pub supplied: SourceCrtOwnedMemoryObservation<R>,
    pub observed: Option<SourceCrtOwnedMemoryObservation<R>>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub pending: P,
    pub continuation: Option<E>,
}

#[must_use]
pub struct SourceCrtOwnedControllerCompletion<S, C, P, V> {
    pub controller: S,
    pub prefix: SourceCrtOwnedPrefix<C, P>,
    value: V,
}
impl<S, C, P, V: Copy> SourceCrtOwnedControllerCompletion<S, C, P, V> {
    pub fn value(&self) -> V { self.value }
    pub(super) fn admitted(controller: S, prefix: SourceCrtOwnedPrefix<C, P>, value: V) -> Self {
        Self { controller, prefix, value }
    }
}
#[must_use]
pub struct SourceCrtOwnedControllerCompletionError<S, C, P> {
    pub controller: S,
    pub prefix: SourceCrtOwnedPrefix<C, P>,
    pub pending: P,
}

#[must_use]
pub struct SourceCrtOwnedPendingChild<C, P, S> {
    pub previous: SourceCrtOwnedPrefix<C, P>,
    pub source: S,
}
#[must_use]
pub struct SourceCrtOwnedChildBeginError<C, P, E> {
    pub previous: SourceCrtOwnedPrefix<C, P>,
    pub pending: P,
    pub actual_module_base: u64,
    pub continuation: Option<E>,
}
