use super::*;
use crate::{ltv_output_values::ValueOutputs, ltv_stream::Kernel, wave_output};

pub enum Error { KernelStopped, OutputStopped, Output(crate::ltv_output_values::Error), Wave(wave_output::Error) }

impl<M, F, O> Completed<M, F, Kernel<ValueOutputs>, O> {

    pub fn take_output_values(&mut self) -> Result<Batch, Error> {
        let outputs = self.run.state.kernel.outputs_mut_if_idle().ok_or(Error::KernelStopped)?;
        outputs.take_output().map_err(Error::Output)
    }

    #[cfg(windows)]
    pub fn play_output_wasapi(&self, parts: &crate::wasapi_render::Parts,
        wait: crate::wasapi_playback::WaitPolicy, cancel: &std::sync::atomic::AtomicBool)
        -> Result<crate::wasapi_playback::Outcome, Error>
    {
        let outputs = self.run.state.kernel.outputs_if_idle().ok_or(Error::KernelStopped)?;
        if outputs.trace().failure.is_some() { return Err(Error::OutputStopped); }
        Ok(crate::wasapi_playback::play_mono_rows(&outputs.parts().output,
            self.run.state.config.hop(), parts, wait, cancel))
    }

    pub fn write_output_wav<W: std::io::Write + ?Sized>(&self, writer: &mut W, sample_rate: u32)
        -> Result<wave_output::Receipt, Error>
    {
        let outputs = self.run.state.kernel.outputs_if_idle().ok_or(Error::KernelStopped)?;
        if outputs.trace().failure.is_some() { return Err(Error::OutputStopped); }
        wave_output::write_mono_rows(writer, sample_rate, self.run.state.config.hop(),
            &outputs.parts().output).map_err(Error::Wave)
    }
}
