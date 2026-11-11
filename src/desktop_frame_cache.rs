use std::sync::Arc;
use super::paint::{Frame, SourceInput};

#[derive(PartialEq, Eq)]
struct PointKey { index: usize, position: [u32; 2], fields: [u32; 4] }
struct SourceKey {
    index: usize, argb: u32, trajectory: bool, selected: bool, points: Vec<PointKey>,
}
impl SourceKey {
    fn matches(&self, source: &SourceInput) -> bool {
        self.index == source.index && self.argb == source.argb
            && self.trajectory == source.trajectory && self.selected == source.selected
            && self.points.len() == source.points.len()
            && self.points.iter().zip(&source.points).all(|(old, point)| {
                old.index == point.index && old.position == point.position.map(f32::to_bits)
                    && old.fields == point.fields.map(f32::to_bits)
            })
    }
    fn capture(source: &SourceInput) -> Self {
        Self { index: source.index, argb: source.argb, trajectory: source.trajectory,
            selected: source.selected, points: source.points.iter().map(|point| PointKey {
                index: point.index, position: point.position.map(f32::to_bits),
                fields: point.fields.map(f32::to_bits),
            }).collect() }
    }
}
struct Entry {
    sources: Vec<SourceKey>, probe: [u32; 2], zoom: u64, frame: Arc<Frame>,
}
#[derive(Default)]
pub(super) struct Cache { entry: Option<Entry> }

impl Cache {
    pub fn get_or_build(&mut self, sources: Vec<SourceInput>, probe: [f32; 2], zoom: f64) -> Arc<Frame> {
        if !normal_controls() {
            self.entry = None;
            return Arc::new(Frame::new(sources, probe, zoom));
        }
        self.lookup(sources, probe, zoom)
    }
    fn lookup(&mut self, sources: Vec<SourceInput>, probe: [f32; 2], zoom: f64) -> Arc<Frame> {
        let probe_bits = probe.map(f32::to_bits);
        let zoom_bits = zoom.to_bits();
        if let Some(entry) = &self.entry {
            if entry.probe == probe_bits && entry.zoom == zoom_bits
                && entry.sources.len() == sources.len()
                && entry.sources.iter().zip(&sources).all(|(old, source)| old.matches(source))
            {
                return entry.frame.clone();
            }
        }
        let key = sources.iter().map(SourceKey::capture).collect();
        let frame = Arc::new(Frame::new(sources, probe, zoom));
        self.entry = Some(Entry { sources: key, probe: probe_bits, zoom: zoom_bits, frame: frame.clone() });
        frame
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
        controls & 0xffc0 == 0x1f80
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    { false }
}

