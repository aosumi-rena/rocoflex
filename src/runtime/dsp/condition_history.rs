use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MissingConditionRows {
    pub required: usize,
    pub available: usize,
}

#[derive(Debug, Clone)]
pub struct ConditionHistory<Row> {
    projected: VecDeque<Row>,
}

impl<Row> Default for ConditionHistory<Row> {
    fn default() -> Self { Self { projected: VecDeque::new() } }
}

impl<Row> ConditionHistory<Row> {

    pub fn from_explicit_pending(projected:VecDeque<Row>)->Self {Self{projected}}
    pub fn pending(&self)->impl ExactSizeIterator<Item=&Row> {self.projected.iter()}
    pub fn into_pending(self)->VecDeque<Row> {self.projected}

    pub fn pending_rows(&self) -> usize { self.projected.len() }

    pub fn clear(&mut self) { self.projected.clear(); }

    pub fn append_and_take<I>(&mut self, projected: I, gate_rows: usize)
        -> Result<Vec<Row>, MissingConditionRows>
    where I: IntoIterator<Item = Row>
    {
        self.projected.extend(projected);
        if self.projected.len() < gate_rows {
            return Err(MissingConditionRows {
                required: gate_rows, available: self.projected.len(),
            });
        }
        Ok(self.projected.drain(..gate_rows).collect())
    }
}
