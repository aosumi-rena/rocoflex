use crate::{
    Result,
    model::ModelTree,
    model_tags::decoded_hashes,
    pitch_score::{
        PitchBase, PitchBaseStages, PitchProfile, PitchProfileStages, PitchScore, PitchScoreStages,
    },
    timbre::{QuantizedLinear, u32_at},
};

#[path = "pitch_selected_head_stream.rs"]
mod incremental;
pub use incremental::{SelectedPitchBatch, SelectedPitchStream, SelectedPitchStreamFailure,
    SelectedPitchWork};
pub(crate) use incremental::SelectedPitchState;
#[path = "pitch_head_selection.rs"]
mod admitted;
pub use admitted::{PitchHeadSelection, AdmittedPitchHead, AdmittedPitchStages,
    AdmittedPitchStream, AdmittedPitchBatch, AdmittedPitchFailure,
    PointwisePitchHead, PointwisePitchStages, PointwiseBaseStages,
    PointwisePitchStream, PointwisePitchBatch, PointwisePitchFailure};
pub(crate) use admitted::{AdmittedPitchState,PitchHeadCount,PitchHeadCountError};

#[derive(Clone, Copy)]
pub enum IndicatorAccumulation {
    FinalI16Clamp,
    PerInputSaturatedI16,
}

#[derive(Clone, Copy)]
pub struct PitchRecord {
    pub value: f32,
    pub valid: bool,
    pub voiced: bool,
}

impl PitchRecord {
    pub fn semantic_bytes(&self) -> [u8; 6] {
        let mut bytes = [0; 6];
        bytes[..4].copy_from_slice(&self.value.to_le_bytes());
        bytes[4] = u8::from(self.valid);
        bytes[5] = u8::from(self.voiced);
        bytes
    }
}

pub struct SelectedPitchHead {
    base: PitchBase,
    outputs: HeadOutputs,
}

struct HeadOutputs {
    profile: PitchProfile,
    score: PitchScore,
    voicing: QuantizedLinear,
    auxiliary: QuantizedLinear,
    indicator_accumulation: IndicatorAccumulation,
}

pub struct SelectedPitchHeadStages {
    pub base: PitchBaseStages,
    pub profile: PitchProfileStages,
    pub score: PitchScoreStages,
    pub voicing: Vec<f32>,
    pub auxiliary: Vec<f32>,
    pub records: Vec<PitchRecord>,
}

pub struct SelectedPitchRows {
    pub profile: PitchProfileStages,
    pub score: PitchScoreStages,
    pub voicing: Vec<f32>,
    pub auxiliary: Vec<f32>,
    pub records: Vec<PitchRecord>,
}

fn validate_owner_layout(data: &[u8], children: usize) -> Result<()> {
    if data.len() != 76 || children != 14 {
        return Err("pitch owner layout".into());
    }
    for (offset, expected) in [(12, 3), (16, 4), (20, 3), (24, 3)] {
        if u32_at(data, offset)? != expected {
            return Err("unsupported pitch owner counts".into());
        }
    }
    Ok(())
}

impl SelectedPitchHead {
    pub(crate) fn prepared_right_rows(&self)->Option<usize> {self.base.prepared_right_rows()}
    pub fn stream_from_explicit_position(&self, previous_position: i32, maximum_rows: usize)
        -> Result<SelectedPitchStream<'_>>
    {
        SelectedPitchStream::new(self, previous_position, maximum_rows)
    }

    pub fn from_dnni(model: &[u8], policy: IndicatorAccumulation) -> Result<Self> {
        let tree = ModelTree::parse(model)?;
        if tree.version != 2 {
            return Err("selected pitch head requires v2".into());
        }
        let root = tree.root;
        let owner = root.children.get(1).ok_or("pitch owner missing")?;
        owner.require(
            0x7fca41ff,
            b"",
            &[560151423416826003 ^ 0xf22ee5adeedbda94 ^ 0xb2f45c29815fb08d],
        )?;
        validate_owner_layout(owner.data, owner.children.len())?;
        let parent = owner
            .children
            .get(10)
            .ok_or("selected pitch head missing")?;
        parent.require(0x7fca41ff, b"", &[0x7fb30f7f39ad76e0])?;
        if parent.children.len() != 5 || parent.data != [2, 0, 0, 0, 16, 0, 0, 0] {
            return Err("selected pitch head configuration".into());
        }
        let load_indicator = |index: usize| -> Result<QuantizedLinear> {
            let n = &parent.children[index];
            n.require(0x7fca41ff, b"modl0", decoded_hashes("modl0"))?;
            if !n.data.is_empty() || n.children.len() != 2 {
                return Err("indicator Linear topology".into());
            }
            n.children[0].require(0x7fca40ff, b"prim4", decoded_hashes("prim4"))?;
            n.children[1].require(0x7fca40ff, b"prim1", decoded_hashes("prim1"))?;
            if n.children.iter().any(|c| !c.children.is_empty()) {
                return Err("indicator parameters must be leaves".into());
            }
            let linear = QuantizedLinear::from_payloads(n.children[0].data, n.children[1].data, 8)?;
            if (linear.in_dim, linear.out_dim) != (256, 1) {
                return Err("indicator dimensions".into());
            }
            Ok(linear)
        };
        Ok(Self {
            base: PitchBase::from_dnni(model)?,
            outputs: HeadOutputs {
                profile: PitchProfile::from_dnni(model)?,
                score: PitchScore::from_dnni(model)?,
                voicing: load_indicator(2)?,
                auxiliary: load_indicator(3)?,
                indicator_accumulation: policy,
            },
        })
    }

    pub fn forward384_final(&self, temporal: &[f32]) -> Result<SelectedPitchHeadStages> {
        let base = self.base.forward384(temporal)?;
        let rows = self.forward_base256_with_state(&base.base256, &mut -1)?;
        Ok(SelectedPitchHeadStages {
            base, profile: rows.profile, score: rows.score, voicing: rows.voicing,
            auxiliary: rows.auxiliary, records: rows.records,
        })
    }

    fn forward_base256_with_state(&self, base256: &[f32], previous: &mut i32)
        -> Result<SelectedPitchRows>
    {
        self.outputs.forward_base256_with_state(base256, previous)
    }

    fn forward_base256_into_work(&self, base256: &[f32], previous: &mut i32,
        work: &mut SelectedPitchWork) -> Result<SelectedPitchRows>
    {
        self.outputs.forward_base256_into_work(base256, previous, work)
    }
}

impl HeadOutputs {
    fn forward_base256_with_state(&self, base256: &[f32], previous: &mut i32)
        -> Result<SelectedPitchRows>
    {
        let mut work = SelectedPitchWork::empty();
        self.forward_base256_into_work(base256, previous, &mut work)
    }

    fn forward_base256_into_work(&self, base256: &[f32], previous: &mut i32,
        work: &mut SelectedPitchWork) -> Result<SelectedPitchRows>
    {
        work.profile = Some(self.profile.forward256_stateful(base256, previous)?);
        let saturated = matches!(self.indicator_accumulation,
            IndicatorAccumulation::PerInputSaturatedI16);
        for row in base256.chunks_exact(256) {
            work.voicing.extend(self.voicing.forward_saturation(row, saturated)?);
            work.auxiliary.extend(self.auxiliary.forward_saturation(row, saturated)?);
        }
        let profile = work.profile.as_ref().expect("completed pitch profile");
        work.score = Some(self.score.forward(&profile.source272)?);
        let score = work.score.as_ref().expect("completed pitch score");
        if score.scores.len() != work.voicing.len()
            || work.auxiliary.len() != work.voicing.len()
            || score.scores.iter().chain(&work.voicing).chain(&work.auxiliary)
                .any(|v| !v.is_finite())
        {
            return Err("invalid selected pitch head outputs".into());
        }
        let records = score.scores.iter().zip(&work.voicing).map(|(&value, &logit)|
            PitchRecord { value, valid: true, voiced: logit > 0.0 }).collect();
        Ok(SelectedPitchRows {
            profile: work.profile.take().expect("completed pitch profile"),
            score: work.score.take().expect("completed pitch score"),
            voicing: std::mem::take(&mut work.voicing),
            auxiliary: std::mem::take(&mut work.auxiliary), records,
        })
    }
}

