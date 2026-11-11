use std::sync::Arc;
use crate::prototype_controls::{ControllerMix, Parameters, Snapshot};

struct Entry {
    revision: u64,
    mix: Arc<ControllerMix>,
    parameters: [u32; 3],
    snapshot: Snapshot,
}

#[derive(Default)]
pub(super) struct Cache { entry: Option<Entry> }

impl Cache {
    pub fn clear(&mut self) { self.entry = None; }

    pub fn get_or_build(&mut self, revision: u64, mix: &Arc<ControllerMix>, parameters: Parameters,
        build: impl FnOnce() -> Result<Snapshot, String>) -> Result<Snapshot, String>
    {
        if !normal_controls() {
            self.clear();
            return build();
        }
        self.lookup(revision, mix, parameters, build)
    }

    fn lookup(&mut self, revision: u64, mix: &Arc<ControllerMix>, parameters: Parameters,
        build: impl FnOnce() -> Result<Snapshot, String>) -> Result<Snapshot, String>
    {
        let parameters = [parameters.pregain_db, parameters.pitch_semitones, parameters.tone_db]
            .map(f32::to_bits);
        if let Some(entry) = &self.entry {
            if entry.revision == revision && Arc::ptr_eq(&entry.mix, mix)
                && entry.parameters == parameters
            {
                return Ok(entry.snapshot.clone());
            }
        }
        self.clear();
        let snapshot = build()?;
        self.entry = Some(Entry { revision, mix: mix.clone(), parameters, snapshot: snapshot.clone() });
        Ok(snapshot)
    }
}

fn normal_controls() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        let mut controls = 0u32;
        unsafe {
            std::arch::asm!("stmxcsr [{p}]", p = in(reg) &mut controls,
                options(nostack, preserves_flags));
        }

        reusable_controls(controls)
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    { false }
}

fn reusable_controls(controls: u32) -> bool { controls & 0xffc0 == 0x1f80 }

