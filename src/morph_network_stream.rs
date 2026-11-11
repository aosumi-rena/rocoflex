use crate::{Result, prototype_synthesis_networks::NcwnNetwork};

pub(crate) struct NcwnStream {
    network: NcwnNetwork,
    input: Vec<Vec<f32>>,
    condition: Vec<Vec<f32>>,
    left: usize,
    right: usize,
    base: usize,
    accepted: usize,
    emitted: usize,
    evaluated: usize,
    batch_rows: usize,
    ended: bool,
    failure: Option<String>,
}

impl NcwnStream {
    pub(crate) fn new(network: NcwnNetwork, batch_rows: usize) -> Result<Self> {
        if batch_rows==0 { return Err("Network processing batch must be positive".into()); }
        let (left, right) = network.streaming_context()?;
        Ok(Self { network, input: Vec::new(), condition: Vec::new(), left,
            right, base: 0, accepted: 0, emitted: 0, evaluated: 0, batch_rows,
            ended: false, failure: None })
    }

    pub(crate) fn lookahead_rows(&self) -> usize { self.right }
    pub(crate) fn retained_rows(&self) -> usize { self.input.len() }
    pub(crate) fn emitted_rows(&self) -> usize { self.emitted }

    pub(crate) fn push(&mut self, input: &[Vec<f32>], condition: &[Vec<f32>],
        final_input: bool, output: &mut Vec<Vec<f32>>) -> Result<()>
    {
        if let Some(error) = &self.failure { return Err(error.clone()); }
        if self.ended { return Err("Conditional network stream has ended".into()); }
        let result = self.advance(input, condition, final_input, output);
        if let Err(error) = &result { self.failure = Some(error.clone()); }
        result
    }

    fn advance(&mut self, input: &[Vec<f32>], condition: &[Vec<f32>],
        final_input: bool, output: &mut Vec<Vec<f32>>) -> Result<()>
    {
        if input.len() != condition.len()
            || input.iter().any(|row| row.len() != self.network.input_width
                || row.iter().any(|x| !x.is_finite()))
            || condition.iter().any(|row| row.len() != self.network.condition_width
                || row.iter().any(|x| !x.is_finite()))
        {
            return Err("Conditional network stream requires matching finite feature and condition rows".into());
        }
        if input.is_empty() && !final_input { return Ok(()); }
        let accepted = self.accepted.checked_add(input.len())
            .ok_or("Conditional network stream row count overflow")?;
        self.input.try_reserve(input.len()).map_err(|e| e.to_string())?;
        self.condition.try_reserve(condition.len()).map_err(|e| e.to_string())?;
        self.input.extend_from_slice(input);
        self.condition.extend_from_slice(condition);
        self.accepted = accepted;

        if !final_input && accepted-self.evaluated<self.batch_rows { return Ok(()); }

        let stable = if final_input { accepted } else { accepted.saturating_sub(self.right) };
        if stable > self.emitted {
            let mut rows = self.network.forward(&self.input, &self.condition)?;
            let available = self.base.checked_add(rows.len()).ok_or("Network output row count overflow")?;
            let end = stable.min(available);
            if end > self.emitted {
                let begin = self.emitted.checked_sub(self.base).ok_or("Invalid retained network context")?;
                let count = end - self.emitted;
                if rows[begin..begin + count].iter().flatten().any(|x| !x.is_finite()) {
                    return Err("Conditional network produced nonfinite live features".into());
                }
                output.try_reserve(count).map_err(|e| e.to_string())?;
                output.extend(rows.drain(begin..begin + count));
                self.emitted = end;
            }
        }

        let next_base = self.emitted.saturating_sub(self.left);
        let discard = next_base.checked_sub(self.base).ok_or("Invalid live network base")?;
        if discard > self.input.len() { return Err("Live network context exceeds retained input".into()); }
        self.input.drain(..discard);
        self.condition.drain(..discard);
        self.base = next_base;
        self.evaluated = accepted;
        self.ended = final_input;
        Ok(())
    }
}
