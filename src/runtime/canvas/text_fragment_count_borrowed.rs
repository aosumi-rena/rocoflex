use super::{CharacterMemory, FragmentCounter, FragmentPoll};

pub struct Prepared<'a, M: CharacterMemory + ?Sized> {
    memory: &'a mut M,
    counter: FragmentCounter<M::Address>,
}
pub struct Counted<'a, M: CharacterMemory + ?Sized> {
    memory: &'a mut M,
    count: super::FragmentCounted<M::Address>,
}
pub enum Poll<'a, M: CharacterMemory + ?Sized> {
    Pending(Prepared<'a, M>), Ready(Counted<'a, M>),
    Failed { pending: Prepared<'a, M>, error: M::Error },
    Stopped(Prepared<'a, M>),
}
impl<'a, M: CharacterMemory + ?Sized> Prepared<'a, M> {

    pub fn from_actual_fragment(memory: &'a mut M, initial: M::Address) -> Self {
        Self { memory, counter: FragmentCounter::from_actual_fragment(initial) }
    }
    pub fn memory(&self) -> &M { &*self.memory }
    pub fn counter(&self) -> &FragmentCounter<M::Address> { &self.counter }

    pub fn poll(self, budget: usize) -> Poll<'a, M> {
        let Self { memory, counter } = self;
        match counter.poll(&mut *memory, budget) {
            FragmentPoll::Pending(counter) => Poll::Pending(Self { memory, counter }),
            FragmentPoll::Ready(count) => Poll::Ready(Counted { memory, count }),
            FragmentPoll::Failed(failure) => Poll::Failed {
                pending: Self { memory, counter: failure.pending }, error: failure.error },
            FragmentPoll::Stopped(counter) => Poll::Stopped(Self { memory, counter }),
        }
    }
}
impl<M: CharacterMemory + ?Sized> Counted<'_, M> {
    pub fn memory(&self) -> &M { &*self.memory }
    pub fn count(&self) -> u64 { self.count.count() }
    pub fn cursor(&self) -> M::Address { self.count.cursor() }
}
