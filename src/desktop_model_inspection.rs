use eframe::egui;
use std::{any::Any, path::Path, thread::JoinHandle, time::Duration};

pub(super) struct Summary { version: u32, nodes: usize }
impl Summary {
    pub fn from_bytes(bytes: &[u8]) -> crate::Result<Self> {
        let tree = crate::model::ModelTree::parse(bytes)?;
        let mut stack = vec![&tree.root];
        let mut nodes = 0usize;
        while let Some(node) = stack.pop() {
            nodes = nodes.checked_add(1).ok_or("Model node count overflow")?;
            stack.extend(node.children.iter());
        }
        Ok(Self { version: tree.version, nodes })
    }
    pub fn message(&self, heading: &str) -> String {
        format!("{heading}\nFormat {} · {} nodes", self.version, self.nodes)
    }
}

struct Request {
    epoch: u64,
    path: String,
}

struct Job {
    request: Request,
    handle: JoinHandle<Result<String, String>>,
}

#[allow(dead_code)]
enum Outcome {
    Inspected { request: Request, result: Result<String, String> },
    Spawn { request: Request, error: std::io::Error },
    Panic { request: Request, payload: Box<dyn Any + Send + 'static> },
}

#[derive(Default)]
pub(super) struct Controller {
    epoch: u64,
    path: Option<String>,
    job: Option<Job>,
    last: Option<Outcome>,
}

impl Controller {
    pub fn busy(&self) -> bool {
        self.job.is_some()
    }

    pub fn invalidate(&mut self, path: &str) {
        self.epoch = self.epoch.wrapping_add(1);
        self.path = Some(path.to_owned());
    }

    fn observe_path(&mut self, path: &str) {
        if self.path.as_deref() != Some(path) {
            self.invalidate(path);
        }
    }

    pub fn start(&mut self, ctx: &egui::Context, path: &str) -> Result<(), String> {
        self.observe_path(path);
        if self.busy() {
            return Err("A model inspection is already in progress.".into());
        }
        self.epoch = self.epoch.wrapping_add(1);
        let request = Request { epoch: self.epoch, path: path.to_owned() };
        let worker_path = request.path.clone();
        let repaint = ctx.clone();
        let handle = std::thread::Builder::new().name("model-inspection".into()).spawn(move || {
            let result = (|| {
                let path = Path::new(&worker_path);
                if super::library::protected_model_path(path)
                    || path.canonicalize().is_ok_and(|resolved| super::library::protected_model_path(&resolved))
                {
                    return Err("Session and authentication files are not model inputs.".into());
                }
                let bytes = crate::model::read_bytes(path, super::generation::MAX_MODEL_BYTES)?;
                let validated = crate::voice_generation::VoiceGenerator::from_dnni(&bytes,
                    crate::pitch_selected_head::IndicatorAccumulation::FinalI16Clamp)?;
                drop(validated);
                drop(crate::tone::ToneGenerator::from_dnni(&bytes)?);
                #[cfg(windows)]
                crate::morph_processor::inspect_analysis_model(&bytes)?;
                Ok(Summary::from_bytes(&bytes)?.message("Valid analysis model"))
            })().map_err(|error: String| format!("Invalid analysis model: {error}"));
            repaint.request_repaint();
            result
        });
        match handle {
            Ok(handle) => {
                self.job = Some(Job { request, handle });
                ctx.request_repaint_after(Duration::from_millis(16));
                Ok(())
            }
            Err(error) => {
                let message = format!("Could not start model inspection: {error}");
                self.last = Some(Outcome::Spawn { request, error });
                Err(message)
            }
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context, path: &str) -> Option<String> {
        self.observe_path(path);
        if !self.job.as_ref()?.handle.is_finished() {

            ctx.request_repaint_after(Duration::from_millis(16));
            return None;
        }
        let Job { request, handle } = self.job.take()?;
        let current = request.epoch == self.epoch && request.path == path;
        match handle.join() {
            Ok(result) => {
                let publication = current.then(|| match &result {
                    Ok(info) | Err(info) => info.clone(),
                });
                self.last = Some(Outcome::Inspected { request, result });
                publication
            }
            Err(payload) => {
                self.last = Some(Outcome::Panic { request, payload });
                current.then(|| "Model inspection stopped unexpectedly.".into())
            }
        }
    }
}

