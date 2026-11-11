use super::*;
use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
};

pub(super) const MAX_MODEL_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, PartialEq)]
struct ToneRequest {
    epoch: u64,
    destination: u64,
    model_selection: u64,
    model_path: String,
    controls: [u32; 3],
}

pub(super) struct PendingToneJob {
    request: ToneRequest,
    receiver: Receiver<Result<Value>>,
    valid: bool,
}

impl DesktopApp {
    pub(super) fn tone_controls(&self) -> [f32; 3] {
        self.native_tone_controls.unwrap_or([
            1.0 - self.tone_position.y as f32,
            self.tone_position.x as f32,
            (self.tone_hue / 360.0) as f32,
        ])
    }

    fn tone_request(&self) -> ToneRequest {
        let (destination, model_selection) = self.generation_context();
        ToneRequest {
            epoch: self.tone_epoch,
            destination,
            model_selection,
            model_path: self.model_paths[0].clone(),
            controls: self.tone_controls().map(f32::to_bits),
        }
    }

    pub(super) fn start_tone_generation(&mut self, ctx: &egui::Context) {
        if self.layout_blocked() || self.generation_busy() {
            return;
        }

        self.commit_tone_hex();
        if self.color_source.is_some() || !self.color_picker {
            return;
        }
        if self.model_paths[0].trim().is_empty() {
            self.error = Some("Choose the analysis model in Resources before generating a point.".into());
            return;
        }
        let request = self.tone_request();
        let path = PathBuf::from(&request.model_path);
        let controls = self.tone_controls();
        let (sender, receiver) = mpsc::channel();
        let ctx = ctx.clone();
        match std::thread::Builder::new()
            .name("tone-point".into())
            .spawn(move || {
                let result = generate_source(&path, controls);
                let _ = sender.send(result);
                ctx.request_repaint();
            }) {
            Ok(_) => {
                self.tone_job = Some(PendingToneJob {
                    request,
                    receiver,
                    valid: true,
                });
            }
            Err(error) => {
                self.error = Some(format!("Could not start point generation: {error}"));
            }
        }
    }

    pub(super) fn tone_generation_status(&self) -> &'static str {
        if self.audio_job.is_some() {
            return "Audio analysis must finish before creating a point.";
        }
        match &self.tone_job {
            Some(job) if !job.valid => "Finishing previous request; apply again when ready.",
            Some(_) if self.layout_blocked() => {
                "Generating point; a successful result will be retained while the canvas is paused."
            }
            Some(_) => "Generating model-backed point...",
            None if self.layout_blocked() => {
                "Point changes are retained while the canvas is paused."
            }
            None if self.model_paths[0].trim().is_empty() => "Select the analysis model in Resources.",
            None => "Creates a model-backed timbre point.",
        }
    }

    pub(super) fn poll_tone_generation(&mut self, ctx: &egui::Context) {
        let Some(mut job) = self.tone_job.take() else {
            return;
        };

        job.valid &= self.color_picker
            && self.color_source.is_none()
            && !self.native_tone_hex.is_editing()
            && self.editor.is_none()
            && (self.layout_blocked() || (self.error.is_none() && self.notice.is_none()))
            && job.request == self.tone_request();
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => {
                self.tone_job = Some(job);
                return;
            }
            Err(TryRecvError::Disconnected) => Err("Point generation stopped unexpectedly.".into()),
        };
        if job.valid {
            match result.and_then(|source| self.append_source(source)) {
                Ok(Publication::Committed | Publication::Inspection) => self.color_picker = false,
                Ok(Publication::Retained) => {}
                Err(error) => self.error = Some(format!("Could not generate a point: {error}")),
            }
        }
        ctx.request_repaint();
    }

    pub(super) fn tone_can_cancel(&self) -> bool {
        self.tone_job.as_ref().is_some_and(|job| job.valid)
    }

    pub(super) fn cancel_tone_generation(&mut self) {
        if let Some(job) = &mut self.tone_job {
            job.valid = false;
        }
        self.tone_epoch = self.tone_epoch.wrapping_add(1);
    }
}

fn generate_source(path: &Path, controls: [f32; 3]) -> Result<Value> {
    let bytes = crate::model::read_bytes(path, MAX_MODEL_BYTES)?;
    let generator = crate::tone::ToneGenerator::from_dnni(&bytes)?;
    let point = generator.generate(controls)?;
    let [red, green, blue] = crate::tone::TonePoint::display_color(controls)?;
    point.source_document(
        format!("#{red:02X}{green:02X}{blue:02X}"),
        format!("ff{red:02x}{green:02x}{blue:02x}"),
    )
}

