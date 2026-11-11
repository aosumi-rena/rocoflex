use super::*;
use std::sync::Arc;
use layout::background_geometry::CompletedGeometry;

pub(super) struct RootRecord { pub argb: u32, pub xy: [f32; 2] }

pub(super) struct Completed {
    pub geometry: Arc<CompletedGeometry>,
    pub roots: Vec<RootRecord>,
    display: Arc<empty_background::literal::Geometry>,
}

enum Cause {
    Shape,
    SourceIdentity,
    NonfinitePosition,
    Colour { index: usize, text: String },
    Saturation { index: usize, error: probe_ring::ServiceError },
}

struct Refused {
    geometry: Arc<CompletedGeometry>,
    cause: Cause,
    completed_roots: Vec<RootRecord>,
}

enum State { Complete(Completed), Refused(Refused) }

#[derive(Default)]
pub(super) struct Published {
    state: Option<State>,
    colours: Vec<String>,
    colour_values: Vec<u32>,
    lineage: empty_background::Lineage,
}

impl Published {

    pub fn begin_document(&mut self) {
        self.lineage.retire();
        self.lineage = empty_background::Lineage::default();
        self.state = None;
        self.colours.clear();
        self.colour_values.clear();
    }

    pub fn lineage(&self) -> empty_background::Lineage { self.lineage.clone() }

    pub fn observe(&mut self, controller: &canvas_state::CanvasState, preset: &Preset) {
        let Some(geometry) = controller.completed_probe_geometry() else { return; };
        let same_colours = self.colours.len() == preset.sources.len()
            && self.colours.iter().zip(&preset.sources).all(|(old, source)| old == &source.color);
        let geometry = geometry.clone();
        let mut roots = Vec::new();
        let result = (|| -> std::result::Result<bool, Cause> {
            let count = geometry.sources.len();
            if count != geometry.getters.len() || count != geometry.root_positions.len()
                || count != preset.sources.len() {
                return Err(Cause::Shape);
            }
            if !controller.sources().is_some_and(|sources| layout::same_sources(&geometry.sources, sources)) {
                return Err(Cause::SourceIdentity);
            }
            if geometry.root_positions.iter().flatten()
                .chain(geometry.getters.iter().flatten().flatten())
                .chain(geometry.segments.iter().flat_map(|edge| edge.xy.iter()))
                .any(|value| !value.is_finite()) {
                return Err(Cause::NonfinitePosition);
            }

            let cached_colours = same_colours && self.colour_values.len() == count;
            if cached_colours {
                if let Some(State::Complete(previous)) = self.state.as_mut() {
                    if Arc::ptr_eq(&previous.geometry, &geometry)
                        || same_display_values(&previous.geometry, &geometry) {

                        previous.geometry = geometry.clone();
                        return Ok(true);
                    }
                }
            }
            for (index, (source, xy)) in preset.sources.iter().zip(&geometry.root_positions).enumerate() {
                let argb = if cached_colours {
                    self.colour_values[index]
                } else {
                    let Some(argb) = source_argb(&source.color) else {
                        return Err(Cause::Colour { index, text: source.color.clone() });
                    };
                    probe_ring::multiplied_source_saturation(argb, 1.5)
                        .map_err(|error| Cause::Saturation { index, error })?
                };
                roots.push(RootRecord { argb, xy: *xy });
            }
            Ok(false)
        })();
        self.state = Some(match result {
            Ok(true) => return,
            Ok(false) => {

                if !same_colours || self.colour_values.len() != roots.len() {
                    self.colours = preset.sources.iter().map(|source| source.color.clone()).collect();
                    self.colour_values = roots.iter().map(|root| root.argb).collect();
                }
                let display = Arc::new(empty_background::literal::Geometry {
                    roots: roots.iter().map(|root| empty_background::literal::RootRecord {
                        argb: root.argb, xy: root.xy.map(f32::to_bits),
                    }).collect(),
                    getters: geometry.getters.iter().flatten().map(|xy| xy.map(f32::to_bits)).collect(),
                    segments: geometry.segments.iter().map(|edge| edge.xy.map(f32::to_bits)).collect(),
                });
                State::Complete(Completed { geometry, roots, display })
            }
            Err(cause) => State::Refused(Refused { geometry, cause, completed_roots: roots }),
        });
    }

    pub fn completed(&self) -> Option<&Completed> {
        match self.state.as_ref()? { State::Complete(value) => Some(value), State::Refused(_) => None }
    }

    pub fn display_geometry(&self) -> Option<Arc<empty_background::literal::Geometry>> {
        Some(self.completed()?.display.clone())
    }
}

fn same_display_values(a: &CompletedGeometry, b: &CompletedGeometry) -> bool {
    fn points(a: &[[f32; 2]], b: &[[f32; 2]]) -> bool {
        a.len() == b.len() && a.iter().flatten().zip(b.iter().flatten())
            .all(|(a, b)| a.to_bits() == b.to_bits())
    }
    points(&a.root_positions, &b.root_positions)
        && a.getters.len() == b.getters.len()
        && a.getters.iter().zip(&b.getters).all(|(a, b)| points(a, b))
        && a.segments.len() == b.segments.len()
        && a.segments.iter().zip(&b.segments)
            .all(|(a, b)| a.xy.iter().zip(&b.xy).all(|(a, b)| a.to_bits() == b.to_bits()))
}

fn source_argb(text: &str) -> Option<u32> {
    let hex = text.strip_prefix('#').unwrap_or(text);
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) { return None; }
    match hex.len() {
        6 => Some(u32::from_str_radix(hex, 16).ok()? | 0xff000000),
        8 => u32::from_str_radix(hex, 16).ok(),
        _ => None,
    }
}
