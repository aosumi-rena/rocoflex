use std::sync::{Arc, RwLock};

use crate::prototype_controls::{Parameters, Snapshot, TargetWeight};

#[derive(Clone, Debug)]
pub enum ConditionTarget {
    Canvas128 { embedding: [f32; 128], targets: Vec<TargetWeight> },

    TonePreview { controls: [f32; 3] },
}

#[cfg(windows)]
pub(crate) const PROCESSING_REQUIRES_CONDITION: &str =
    "Choose an enabled canvas voice or open Tone preview before processing audio";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConditionAbsence {
    #[default]
    Empty,
    Unavailable,
}

#[derive(Clone, Debug)]
pub struct CurrentCondition {
    pub revision: u64,
    pub target: ConditionTarget,
    pub pregain_db: f32,
    pub pitch_semitones: f32,
}

#[derive(Default)]
struct Published {
    revision: u64,
    condition: Option<Arc<CurrentCondition>>,
    absence: ConditionAbsence,

    next_boundary: u128,
    first_empty: u128,
    first_unavailable: u128,
}

#[derive(Clone, Default)]
pub struct ConditionBus {
    published: Arc<RwLock<Published>>,
}

impl ConditionBus {
    pub fn new() -> Self { Self::default() }

    pub fn publish_canvas(&self, snapshot: Snapshot) -> Result<u64, String> {
        self.publish(ConditionTarget::Canvas128 {
            embedding: snapshot.embedding, targets: snapshot.targets,
        }, snapshot.parameters)
    }

    pub fn publish_tone_preview(&self, controls: [f32; 3], parameters: Parameters) -> Result<u64, String> {
        self.publish(ConditionTarget::TonePreview { controls }, parameters)
    }

    fn publish(&self, target: ConditionTarget, parameters: Parameters) -> Result<u64, String> {
        let valid_target = match &target {
            ConditionTarget::Canvas128 { embedding, targets } =>
                embedding.iter().all(|value| value.is_finite())
                    && targets.iter().all(|target| target.weight.is_finite()),
            ConditionTarget::TonePreview { controls } => controls.iter().all(|value| value.is_finite()),
        };
        if !valid_target || !parameters.pregain_db.is_finite()
            || !parameters.pitch_semitones.is_finite() {
            return Err("Invalid live voice condition.".into());
        }
        let mut published = self.published.write().map_err(|_| "Live voice controls are unavailable.".to_owned())?;
        if let Some(prior) = &published.condition {
            if same_condition(prior, &target, parameters) { return Ok(published.revision); }
        }
        published.revision = published.revision.wrapping_add(1);
        let revision = published.revision;
        published.absence = ConditionAbsence::Empty;
        published.condition = Some(Arc::new(CurrentCondition {
            revision,
            target,
            pregain_db: parameters.pregain_db,
            pitch_semitones: parameters.pitch_semitones,
        }));
        Ok(revision)
    }

    pub fn publish_empty(&self) -> Result<u64, String> {
        self.publish_absence(ConditionAbsence::Empty)
    }

    pub fn publish_unavailable(&self) -> Result<u64, String> {
        self.publish_absence(ConditionAbsence::Unavailable)
    }

    fn publish_absence(&self, absence: ConditionAbsence) -> Result<u64, String> {
        let mut published = self.published.write().map_err(|_| "Live voice controls are unavailable.".to_owned())?;
        if published.condition.is_some() || published.absence != absence {
            let boundary = published.next_boundary.checked_add(1)
                .ok_or("Live voice boundary identifiers are exhausted.")?;
            published.revision = published.revision.wrapping_add(1);
            published.next_boundary = boundary;
            match absence {
                ConditionAbsence::Empty if published.first_empty == 0 => published.first_empty = boundary,
                ConditionAbsence::Unavailable if published.first_unavailable == 0 => published.first_unavailable = boundary,
                _ => {}
            }
            published.condition = None;
            published.absence = absence;
        }
        Ok(published.revision)
    }

    pub fn reader(&self) -> ConditionReader {
        ConditionReader { bus: self.clone(), revision: 0, current: None,
            absence: ConditionAbsence::Empty, retain_on_empty: false,
            boundary_cursor: 0, retired: false, unavailable_latched: false }
    }

    pub(crate) fn reader_retaining_target_on_empty(&self) -> Result<ConditionReader, String> {
        let mut published = self.published.write()
            .map_err(|_| "Live voice controls are unavailable.".to_owned())?;
        let current = published.condition.clone()
            .ok_or("No live voice condition is available for audio startup.")?;

        published.first_empty = 0;
        published.first_unavailable = 0;
        Ok(ConditionReader { bus: self.clone(), revision: published.revision,
            current: Some(current), absence: published.absence, retain_on_empty: true,
            boundary_cursor: published.next_boundary, retired: false, unavailable_latched: false })
    }
}

pub struct ConditionReader {
    bus: ConditionBus,
    revision: u64,
    current: Option<Arc<CurrentCondition>>,
    absence: ConditionAbsence,
    retain_on_empty: bool,
    boundary_cursor: u128,
    retired: bool,
    unavailable_latched: bool,
}

impl ConditionReader {
    pub fn poll(&mut self) -> Option<&CurrentCondition> {
        if let Ok(published) = self.bus.published.try_read() {
            let unseen_boundary = self.retain_on_empty && !self.retired && !self.unavailable_latched
                && (published.first_empty > self.boundary_cursor
                    || published.first_unavailable > self.boundary_cursor);
            if self.revision != published.revision || unseen_boundary {
                if self.retain_on_empty {
                    if !self.retired && !self.unavailable_latched {
                        let empty = published.first_empty;
                        let unavailable = published.first_unavailable;
                        let has_empty = empty > self.boundary_cursor;
                        let has_unavailable = unavailable > self.boundary_cursor;
                        if has_unavailable && (!has_empty || unavailable < empty) {
                            self.unavailable_latched = true;
                            self.current = None;
                            self.absence = ConditionAbsence::Unavailable;
                        } else if has_empty {
                            self.retired = true;
                        } else {
                            self.current = published.condition.clone();
                            self.absence = published.absence;
                        }
                    }
                } else {
                    self.current = published.condition.clone();
                    self.absence = published.absence;
                }
                self.revision = published.revision;
            }
        }
        self.current()
    }

    pub fn current(&self) -> Option<&CurrentCondition> { self.current.as_deref() }

    pub fn absence(&self) -> Option<ConditionAbsence> {
        if self.current.is_none() { Some(self.absence) } else { None }
    }

    pub fn poll_shared(&mut self) -> Option<Arc<CurrentCondition>> {
        self.poll();
        self.current.clone()
    }

    pub fn revision(&self) -> u64 { self.revision }
}

fn same_condition(prior: &CurrentCondition, target: &ConditionTarget, parameters: Parameters) -> bool {
    let same_target = match (&prior.target, target) {
        (ConditionTarget::Canvas128 { embedding: a, targets: at },
            ConditionTarget::Canvas128 { embedding: b, targets: bt }) =>
                a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits())
                    && at.len() == bt.len()
                    && at.iter().zip(bt).all(|(a, b)|
                        a.target == b.target && a.weight.to_bits() == b.weight.to_bits()),
        (ConditionTarget::TonePreview { controls: a }, ConditionTarget::TonePreview { controls: b }) =>
            a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits()),
        _ => false,
    };
    same_target && prior.pregain_db.to_bits() == parameters.pregain_db.to_bits()
        && prior.pitch_semitones.to_bits() == parameters.pitch_semitones.to_bits()
}

