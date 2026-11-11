use crate::conditional_gate::Batch;

pub(crate) struct Publication { retired: Vec<Batch>, pending: Option<Batch> }
impl Publication {
    pub(crate) fn new() -> Self { Self { retired: Vec::new(), pending: None } }
    pub(crate) fn retired(&self) -> &[Batch] { &self.retired }
    pub(crate) fn pending(&self) -> Option<&Batch> { self.pending.as_ref() }

    pub(crate) fn publish(&mut self, destination: &mut Batch, output: Batch)
        -> Result<(), std::collections::TryReserveError> {
        assert!(self.pending.is_none(), "unreplayed output retirement");
        self.pending = Some(std::mem::replace(destination, output));
        self.retired.try_reserve(1)?;
        self.retired.push(self.pending.take().expect("retained displaced output"));
        Ok(())
    }
    pub(crate) fn into_parts(self) -> (Vec<Batch>, Option<Batch>) { (self.retired, self.pending) }
}
