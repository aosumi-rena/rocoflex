use super::{Affine, MatrixError, baseline_dot};
use crate::synthesis::execution::Cancellation;

impl Affine {
    pub(super) fn execute_frames(&self, input: &[f32], output: &mut [f32], _frames: usize) -> Result<(), MatrixError> {
        self.execute_block(input, output, None)
    }

    fn execute_block(
        &self,
        input: &[f32],
        output: &mut [f32],
        cancellation: Option<&Cancellation>,
    ) -> Result<(), MatrixError> {
        for (input, output) in input
            .chunks_exact(self.input_channels())
            .zip(output.chunks_exact_mut(self.output_channels()))
        {
            if cancellation.is_some_and(Cancellation::is_cancelled) {
                return Err(MatrixError::Cancelled);
            }
            if self.matrix.integer_frame(input,output,false) {
                if let Some(bias)=&self.bias {for (value,bias) in output.iter_mut().zip(bias.iter()) {*value+=bias;}}
                continue;
            }
            for (channel, target) in output.iter_mut().enumerate() {
                let mut value = baseline_dot(self.matrix.row(channel), input);

                if let Some(bias) = &self.bias {
                    value += bias[channel];
                }
                *target = value;
            }
        }
        Ok(())
    }
}
