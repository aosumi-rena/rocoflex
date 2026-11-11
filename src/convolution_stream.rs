use super::Conv;
use crate::Result;
use std::collections::VecDeque;

pub(crate) struct ConvolutionCursor {
    input: VecDeque<f32>,
    base_row: usize,
    seen_rows: usize,
    next_row: usize,
    row_limit: usize,
    ended: bool,
    halted: bool,
}

impl ConvolutionCursor {
    pub(crate) fn new(conv: &Conv, row_limit: usize) -> Result<Self> {
        if row_limit == 0 || conv.in_dim == 0 || conv.out_dim == 0
            || conv.stride == 0 || conv.taps.is_empty()
            || row_limit.checked_mul(conv.in_dim).is_none()
            || row_limit.checked_mul(conv.out_dim).is_none()
            || row_limit.checked_add(2 * conv.padding).is_none()
        {
            return Err("Convolution stream dimensions or row limit are invalid".into());
        }
        Ok(Self {
            input: VecDeque::new(), base_row: 0, seen_rows: 0, next_row: 0,
            row_limit, ended: false, halted: false,
        })
    }

    pub(crate) fn emitted_rows(&self) -> usize { self.next_row }

    pub(crate) fn count_cursor_state(&self)->(bool,bool) {
        (!self.input.is_empty() || self.base_row!=0 || self.seen_rows!=0
            || self.next_row!=0 || self.ended,self.halted)
    }

    pub(crate) fn append(
        &mut self, conv: &Conv, input: &[f32], final_input: bool, output: &mut Vec<f32>,
    ) -> Result<()> {
        if self.ended || self.halted {
            return Err("Convolution stream is already ended or halted".into());
        }
        if input.len() % conv.in_dim != 0 || input.iter().any(|v| !v.is_finite()) {
            return Err("Convolution stream requires finite complete input rows".into());
        }
        let rows = input.len() / conv.in_dim;
        let seen = self.seen_rows.checked_add(rows)
            .filter(|&v| v <= self.row_limit)
            .ok_or("Convolution stream row limit exceeded")?;
        let effective = conv.dilation.checked_mul(conv.taps.len() - 1)
            .and_then(|v| v.checked_add(1)).ok_or("Convolution extent overflow")?;
        let padded = seen.checked_add(2 * conv.padding)
            .ok_or("Convolution padded extent overflow")?;
        if final_input && padded < effective {
            return Err("Nonpositive final convolution output length".into());
        }
        self.input.extend(input.iter().copied());
        self.seen_rows = seen;
        self.ended = final_input;
        let final_rows = if padded >= effective {
            (padded - effective) / conv.stride + 1
        } else { 0 };

        loop {
            let start = self.next_row.checked_mul(conv.stride)
                .ok_or("Convolution output cursor overflow")?;
            let last = start.checked_add(effective - 1)
                .ok_or("Convolution read cursor overflow")?;
            if final_input {
                if self.next_row >= final_rows { break; }
            } else if last >= conv.padding && last - conv.padding >= seen {
                break;
            }
            let mut value = vec![0.0_f32; conv.out_dim];
            let mut row = vec![0.0_f32; conv.in_dim];
            for (tap, matrix) in conv.taps.iter().enumerate() {
                let position = start + tap * conv.dilation;
                if let Some(source_row)=position.checked_sub(conv.padding).filter(|&src|src<seen) {
                    let Some(relative) = source_row.checked_sub(self.base_row) else {
                        self.halted = true;
                        return Err("Convolution history precedes its retained range".into());
                    };
                    let begin = relative * conv.in_dim;
                    if begin + conv.in_dim > self.input.len() {
                        self.halted = true;
                        return Err("Convolution history is missing an admitted row".into());
                    }
                    for (target,source) in row.iter_mut().zip(self.input.iter().skip(begin)) {
                        *target=*source;
                    }
                } else {
                    row.fill(0.);
                }
                let result=if tap==0 {
                    matrix.forward_unbiased_saturation(&row,true)
                        .map(|values|value.copy_from_slice(&values))
                } else {
                    matrix.forward_add_saturating_f32(&row,&mut value)
                };
                if let Err(reason) = result {
                    self.halted = true;
                    return Err(reason);
                }
            }
            for (v, &bias) in value.iter_mut().zip(&conv.bias) { *v += bias; }
            output.extend(value);
            self.next_row += 1;
            let keep_from = (self.next_row * conv.stride)
                .saturating_sub(conv.padding).min(seen);
            let discard_rows = keep_from.saturating_sub(self.base_row);
            let discard = discard_rows * conv.in_dim;
            if discard > self.input.len() {
                self.halted = true;
                return Err("Convolution retained-range invariant failed".into());
            }
            drop(self.input.drain(..discard));
            self.base_row = keep_from;
        }
        Ok(())
    }
}
