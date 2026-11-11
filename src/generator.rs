use crate::{
    Result,
    analysis_frontend::{AnalysisRows, SourceMathJoinAnalysis},
    bank::{BankProducer, BankProducerStages},
    bank_reducer::{SourcebankReducer, SourcebankStages, current_windows},
    generation_curve::{GenerationCurve4, GenerationArcStages, SourceGenerationArcParts,
        SourceGenerationArcContext, SourceGenerationArcFailure, SourceInterpolationRemapMath},
    generation_schedule::{AcceptedWindow, GenerationSchedule, SourceGenerationWindowMath,
        SourceGenerationWindowsParts, SourceGenerationWindowsFailure, SourceGenerationWindow,
        consume_source_windows, InterruptedSourceWindows, SourceWindowConsumptionCause,
        ConsumerStop, ConsumerPhase, SourceWindowVisit},
    ltv_coefficient_math::ScalarUpper,
    pitch_selected_head::PitchRecord,
    reduction::TimbreReduction as Target4,
    source_pitch_join::{JoinedRows, SourceJoinMath},
    timbre::{TimbreHead as Head8, TimbreStages},
};
use std::{collections::TryReserveError, convert::Infallible};

#[path = "generation_decimation_math.rs"]
mod generation_decimation_math;
pub use generation_decimation_math::*;

pub struct DecimatedRows {
    pub rows128: Vec<f32>,
    pub rows4: Vec<f32>,
    pub removed_indices: Vec<usize>,
}

fn norm4(a: &[f32], b: &[f32]) -> f32 {
    let mut sum = 0.0_f32;
    for j in 0..4 {
        let d = a[j] - b[j];
        sum += d * d;
    }
    sum.sqrt()
}

pub fn greedy_decimate(
    rows128: &[f32],
    rows4: &[f32],
    target_count: usize,
) -> Result<DecimatedRows> {
    match greedy_core(rows128, rows4, target_count, HostGreedy) {
        Ok(completed) => Ok(completed.output),
        Err(failure) => match failure.cause {
            GreedyCoreCause::Input => Err(GREEDY_INPUT_ERROR.into()),
            GreedyCoreCause::Choice(never) => match never {},
        },
    }
}

const GREEDY_INPUT_ERROR: &str = "invalid greedy-decimation input";

#[derive(Clone, Copy, Debug)]
pub enum SourceDecimationStage {
    Outer128Storage, Copy128Row, Outer4Storage, Copy4Row, Ready,
    RoundStorage, Candidate, RequiredControl, Merge, Erase4, Erase128, IndexCommit, Complete,
}
pub struct SourceDecimationProgress {
    stage: SourceDecimationStage,
    round: usize,
    current_count: usize,
    scan_count: Option<usize>,
    candidate: Option<u32>,
    raw_best_bits: Option<u64>,
    raw_chosen: u32,
    initialization: Option<(SourceGenerationDecimationSpace, usize)>,
    merge: Option<SourceGenerationDecimationMergeTag>,
    pending_word: Option<u32>,
}
impl SourceDecimationProgress {
    pub fn stage(&self) -> SourceDecimationStage { self.stage }
    pub fn round(&self) -> usize { self.round }

    pub fn current_count(&self) -> usize { self.current_count }

    pub fn scan_count(&self) -> Option<usize> { self.scan_count }

    pub fn candidate(&self) -> Option<u32> { self.candidate }
    pub fn raw_best_bits(&self) -> Option<u64> { self.raw_best_bits }

    pub fn raw_chosen(&self) -> u32 { self.raw_chosen }
    pub fn initialization(&self) -> Option<(SourceGenerationDecimationSpace, usize)> { self.initialization }
    pub fn merge(&self) -> Option<SourceGenerationDecimationMergeTag> { self.merge }
    pub fn pending_word(&self) -> Option<u32> { self.pending_word }
}

pub struct SourceDecimationPendingRow {
    space: SourceGenerationDecimationSpace, ordinal: usize, words: Vec<f32>,
}
impl SourceDecimationPendingRow {
    pub fn space(&self) -> SourceGenerationDecimationSpace { self.space }
    pub fn ordinal(&self) -> usize { self.ordinal }
    pub fn words(&self) -> &[f32] { &self.words }
}

pub struct SourceDecimationRequiredControl {
    round: usize, raw_chosen: u32, signed_left_index: i64, current_count: usize, best_bits: u64,
}
impl SourceDecimationRequiredControl {
    pub fn round(&self) -> usize { self.round }
    pub fn raw_chosen(&self) -> u32 { self.raw_chosen }
    pub fn signed_left_index(&self) -> i64 { self.signed_left_index }
    pub fn current_count(&self) -> usize { self.current_count }
    pub fn best_bits(&self) -> u64 { self.best_bits }
}
pub enum SourceDecimationCause {
    Input(String), AlreadyStopped, Storage(TryReserveError),
    Math(SourceGenerationDecimationStopped), RequiredControl(SourceDecimationRequiredControl),
}
pub struct SourceDecimationParts {
    pub original_rows128: Vec<f32>, pub original_rows4: Vec<f32>,
    pub output: DecimatedRows, pub math: SourceGenerationDecimationMath,
    pub progress: SourceDecimationProgress,
}
pub struct SourceDecimationCompleted { parts: SourceDecimationParts }
impl SourceDecimationCompleted {
    pub fn original_rows128(&self) -> &[f32] { &self.parts.original_rows128 }
    pub fn original_rows4(&self) -> &[f32] { &self.parts.original_rows4 }
    pub fn output(&self) -> &DecimatedRows { &self.parts.output }
    pub fn math(&self) -> &SourceGenerationDecimationMath { &self.parts.math }
    pub fn progress(&self) -> &SourceDecimationProgress { &self.parts.progress }
    pub fn into_parts(self) -> SourceDecimationParts { self.parts }
}

struct GreedyRound<B> { best: B, chosen: usize }
struct GreedyWork<B> {
    rows128: Option<Vec<Vec<f32>>>, rows4: Option<Vec<Vec<f32>>>,
    pending_row: Option<SourceDecimationPendingRow>, removed_indices: Option<Vec<usize>>,
    round: Option<GreedyRound<B>>, progress: Option<SourceDecimationProgress>,
}
impl<B> GreedyWork<B> {
    fn new() -> Self {
        Self { rows128: None, rows4: None, pending_row: None,
            removed_indices: None, round: None, progress: None }
    }
}

pub struct SourceDecimationFailure {
    original_rows128: Vec<f32>, original_rows4: Vec<f32>, target_count: usize,
    math: SourceGenerationDecimationMath, work: GreedyWork<u64>, cause: SourceDecimationCause,
}
impl SourceDecimationFailure {
    pub fn original_rows128(&self) -> &[f32] { &self.original_rows128 }
    pub fn original_rows4(&self) -> &[f32] { &self.original_rows4 }
    pub fn target_count(&self) -> usize { self.target_count }
    pub fn math(&self) -> &SourceGenerationDecimationMath { &self.math }
    pub fn working128(&self) -> Option<&[Vec<f32>]> { self.work.rows128.as_deref() }
    pub fn working4(&self) -> Option<&[Vec<f32>]> { self.work.rows4.as_deref() }
    pub fn pending_row(&self) -> Option<&SourceDecimationPendingRow> { self.work.pending_row.as_ref() }

    pub fn removed_indices(&self) -> Option<&[usize]> { self.work.removed_indices.as_deref() }
    pub fn progress(&self) -> Option<&SourceDecimationProgress> { self.work.progress.as_ref() }
    pub fn cause(&self) -> &SourceDecimationCause { &self.cause }
}

pub fn greedy_decimate_with_source_math(original_rows128: Vec<f32>, original_rows4: Vec<f32>,
    target_count: usize, math: SourceGenerationDecimationMath)
    -> std::result::Result<SourceDecimationCompleted, SourceDecimationFailure>
{
    match greedy_core(&original_rows128, &original_rows4, target_count, SourceGreedy { math }) {
        Ok(completed) => Ok(SourceDecimationCompleted { parts: SourceDecimationParts {
            original_rows128, original_rows4, output: completed.output, math: completed.choice.math,
            progress: completed.progress.unwrap() } }),
        Err(failure) => {
            let cause = match failure.cause {
                GreedyCoreCause::Input => SourceDecimationCause::Input(GREEDY_INPUT_ERROR.into()),
                GreedyCoreCause::Choice(cause) => cause,
            };
            Err(SourceDecimationFailure { original_rows128, original_rows4, target_count,
                math: failure.choice.math, work: failure.work, cause })
        }
    }
}

#[derive(Clone, Copy)]
enum GreedyOrder { Host, Source }
#[derive(Clone, Copy)]
struct GreedyMergeStep {
    space: SourceGenerationDecimationSpace, side: SourceGenerationDecimationSide, element: usize,
}
struct GreedyMergeSteps { order: GreedyOrder, pass: usize, element: usize, right: bool }
impl GreedyMergeSteps {
    fn new(order: GreedyOrder) -> Self { Self { order, pass: 0, element: 0, right: false } }
}
impl Iterator for GreedyMergeSteps {
    type Item = GreedyMergeStep;
    fn next(&mut self) -> Option<Self::Item> {
        use SourceGenerationDecimationSpace::{Rows4, Rows128};
        use SourceGenerationDecimationSide::{Left, Right};
        match self.order {
            GreedyOrder::Host => {
                if self.pass == 2 { return None; }
                let (space, width) = if self.pass == 0 { (Rows128, 128) } else { (Rows4, 4) };
                let step = GreedyMergeStep { space, side: if self.right { Right } else { Left }, element: self.element };
                if self.right {
                    self.element += 1;
                    if self.element == width { self.pass += 1; self.element = 0; }
                }
                self.right = !self.right;
                Some(step)
            }
            GreedyOrder::Source => {
                if self.pass == 4 { return None; }
                let (space, width) = if self.pass < 2 { (Rows4, 4) } else { (Rows128, 128) };
                let step = GreedyMergeStep { space, side: if self.pass % 2 == 0 { Left } else { Right }, element: self.element };
                self.element += 1;
                if self.element == width { self.pass += 1; self.element = 0; }
                Some(step)
            }
        }
    }
}
struct GreedyDecision<B> { best: B, replace: bool }
trait GreedyChoice {
    type Best: Copy;
    type Error;
    fn values_admitted(&self, rows128: &[f32], rows4: &[f32], n: usize) -> bool;
    fn ready(&self) -> std::result::Result<(), Self::Error>;
    fn initialize(&mut self, rows128: &[f32], rows4: &[f32], work: &mut GreedyWork<Self::Best>)
        -> std::result::Result<(), Self::Error>;
    fn begin_round(&self, work: &mut GreedyWork<Self::Best>, round: usize, n: usize);
    fn reserve_round(&mut self, work: &mut GreedyWork<Self::Best>) -> std::result::Result<(), Self::Error>;
    fn score(&mut self, round: usize, candidate: usize, center: &[f32], left: &[f32], right: &[f32], best: Self::Best)
        -> std::result::Result<GreedyDecision<Self::Best>, Self::Error>;
    fn commit(&self, work: &mut GreedyWork<Self::Best>, candidate: usize, decision: GreedyDecision<Self::Best>);
    fn control(&self, work: &mut GreedyWork<Self::Best>, n: usize) -> std::result::Result<(), Self::Error>;
    fn order(&self) -> GreedyOrder;
    fn merge(&mut self, round: usize, selected: usize, step: GreedyMergeStep, center: u32, neighbor: u32)
        -> std::result::Result<u32, Self::Error>;
    fn erase_order(&self) -> [SourceGenerationDecimationSpace; 2];
}

struct HostGreedy;
impl GreedyChoice for HostGreedy {
    type Best = f32;
    type Error = Infallible;
    fn values_admitted(&self, rows128: &[f32], rows4: &[f32], _: usize) -> bool {
        !rows128.iter().chain(rows4).any(|x| !x.is_finite())
    }
    fn ready(&self) -> std::result::Result<(), Infallible> { Ok(()) }
    fn initialize(&mut self, rows128: &[f32], rows4: &[f32], work: &mut GreedyWork<f32>)
        -> std::result::Result<(), Infallible>
    {
        work.rows128 = Some(rows128.chunks_exact(128).map(|r| r.to_vec()).collect::<Vec<_>>());
        work.rows4 = Some(rows4.chunks_exact(4).map(|r| r.to_vec()).collect::<Vec<_>>());
        Ok(())
    }
    fn begin_round(&self, work: &mut GreedyWork<f32>, _: usize, _: usize) {
        work.round = Some(GreedyRound { best: f32::INFINITY, chosen: 1 });
    }
    fn reserve_round(&mut self, _: &mut GreedyWork<f32>) -> std::result::Result<(), Infallible> { Ok(()) }
    fn score(&mut self, _: usize, _: usize, center: &[f32], left: &[f32], right: &[f32], best: f32)
        -> std::result::Result<GreedyDecision<f32>, Infallible>
    {
        let score = norm4(center, left) + norm4(center, right);
        Ok(GreedyDecision { best: score, replace: score < best })
    }
    fn commit(&self, work: &mut GreedyWork<f32>, candidate: usize, decision: GreedyDecision<f32>) {
        if decision.replace {
            let round = work.round.as_mut().unwrap();
            round.best = decision.best;
            round.chosen = candidate;
        }
    }
    fn control(&self, _: &mut GreedyWork<f32>, _: usize) -> std::result::Result<(), Infallible> { Ok(()) }
    fn order(&self) -> GreedyOrder { GreedyOrder::Host }
    fn merge(&mut self, _: usize, _: usize, step: GreedyMergeStep, center: u32, neighbor: u32)
        -> std::result::Result<u32, Infallible>
    {
        let center = f32::from_bits(center);
        let neighbor = f32::from_bits(neighbor);
        let bits = match step.side {
            SourceGenerationDecimationSide::Left => 0.5_f32 * (neighbor + center),
            SourceGenerationDecimationSide::Right => 0.5_f32 * (center + neighbor),
        };
        Ok(bits.to_bits())
    }
    fn erase_order(&self) -> [SourceGenerationDecimationSpace; 2] {
        [SourceGenerationDecimationSpace::Rows128, SourceGenerationDecimationSpace::Rows4]
    }
}

fn copy_source_groups(input: &[f32], width: usize, space: SourceGenerationDecimationSpace,
    groups: &mut Option<Vec<Vec<f32>>>, pending: &mut Option<SourceDecimationPendingRow>,
    progress: &mut Option<SourceDecimationProgress>) -> std::result::Result<(), SourceDecimationCause>
{
    let state = progress.as_mut().unwrap();
    state.stage = match space {
        SourceGenerationDecimationSpace::Rows128 => SourceDecimationStage::Outer128Storage,
        SourceGenerationDecimationSpace::Rows4 => SourceDecimationStage::Outer4Storage,
    };
    state.initialization = None;
    *groups = Some(Vec::new());
    groups.as_mut().unwrap().try_reserve_exact(input.len() / width).map_err(SourceDecimationCause::Storage)?;
    for (ordinal, row) in input.chunks_exact(width).enumerate() {
        state.stage = match space {
            SourceGenerationDecimationSpace::Rows128 => SourceDecimationStage::Copy128Row,
            SourceGenerationDecimationSpace::Rows4 => SourceDecimationStage::Copy4Row,
        };
        state.initialization = Some((space, ordinal));
        *pending = Some(SourceDecimationPendingRow { space, ordinal, words: Vec::new() });
        let words = &mut pending.as_mut().unwrap().words;
        words.try_reserve_exact(width).map_err(SourceDecimationCause::Storage)?;
        words.extend_from_slice(row);
        groups.as_mut().unwrap().push(pending.take().unwrap().words);
    }
    Ok(())
}

struct SourceGreedy { math: SourceGenerationDecimationMath }
impl GreedyChoice for SourceGreedy {
    type Best = u64;
    type Error = SourceDecimationCause;
    fn values_admitted(&self, _: &[f32], _: &[f32], n: usize) -> bool { n <= i32::MAX as usize }
    fn ready(&self) -> std::result::Result<(), SourceDecimationCause> {
        if self.math.failure().is_some() || self.math.is_active() { Err(SourceDecimationCause::AlreadyStopped) }
        else { Ok(()) }
    }
    fn initialize(&mut self, rows128: &[f32], rows4: &[f32], work: &mut GreedyWork<u64>)
        -> std::result::Result<(), SourceDecimationCause>
    {
        work.progress = Some(SourceDecimationProgress { stage: SourceDecimationStage::Outer128Storage,
            round: 0, current_count: rows4.len() / 4, scan_count: None, candidate: None,
            raw_best_bits: None, raw_chosen: 0, initialization: None, merge: None, pending_word: None });
        copy_source_groups(rows128, 128, SourceGenerationDecimationSpace::Rows128,
            &mut work.rows128, &mut work.pending_row, &mut work.progress)?;
        copy_source_groups(rows4, 4, SourceGenerationDecimationSpace::Rows4,
            &mut work.rows4, &mut work.pending_row, &mut work.progress)?;
        let progress = work.progress.as_mut().unwrap();
        progress.initialization = None;
        progress.stage = SourceDecimationStage::Ready;
        Ok(())
    }
    fn begin_round(&self, work: &mut GreedyWork<u64>, round: usize, n: usize) {
        let best = self.math.configuration().best_seed_bits;
        work.round = Some(GreedyRound { best, chosen: 0 });
        let progress = work.progress.as_mut().unwrap();
        progress.stage = SourceDecimationStage::RoundStorage;
        progress.round = round;
        progress.current_count = n;
        progress.scan_count = Some(n);
        progress.candidate = None;
        progress.raw_best_bits = Some(best);
        progress.raw_chosen = 0;
        progress.merge = None;
        progress.pending_word = None;
    }
    fn reserve_round(&mut self, work: &mut GreedyWork<u64>) -> std::result::Result<(), SourceDecimationCause> {
        work.removed_indices.as_mut().unwrap().try_reserve(1).map_err(SourceDecimationCause::Storage)
    }
    fn score(&mut self, round: usize, candidate: usize, center: &[f32], left: &[f32], right: &[f32], best: u64)
        -> std::result::Result<GreedyDecision<u64>, SourceDecimationCause>
    {
        let bits = |row: &[f32]| [row[0].to_bits(), row[1].to_bits(), row[2].to_bits(), row[3].to_bits()];
        let score = self.math.score_candidate(SourceGenerationDecimationCandidateTag {
            round, candidate: candidate as u32 }, bits(center), bits(left), bits(right), best)
            .map_err(SourceDecimationCause::Math)?;
        Ok(GreedyDecision { best: score.new_best_bits, replace: score.index_should_replace })
    }
    fn commit(&self, work: &mut GreedyWork<u64>, candidate: usize, decision: GreedyDecision<u64>) {
        let round = work.round.as_mut().unwrap();
        if decision.replace { round.chosen = candidate; }
        round.best = decision.best;
        let progress = work.progress.as_mut().unwrap();
        progress.raw_chosen = round.chosen as u32;
        progress.raw_best_bits = Some(round.best);
    }
    fn control(&self, work: &mut GreedyWork<u64>, n: usize) -> std::result::Result<(), SourceDecimationCause> {
        let round = work.round.as_ref().unwrap();
        if round.chosen == 0 || round.chosen >= n - 1 {
            let progress = work.progress.as_mut().unwrap();
            progress.stage = SourceDecimationStage::RequiredControl;
            return Err(SourceDecimationCause::RequiredControl(SourceDecimationRequiredControl {
                round: progress.round, raw_chosen: round.chosen as u32,
                signed_left_index: i64::from(round.chosen as u32) - 1,
                current_count: n, best_bits: round.best }));
        }
        Ok(())
    }
    fn order(&self) -> GreedyOrder { GreedyOrder::Source }
    fn merge(&mut self, round: usize, selected: usize, step: GreedyMergeStep, center: u32, neighbor: u32)
        -> std::result::Result<u32, SourceDecimationCause>
    {
        self.math.merge_neighbor(SourceGenerationDecimationMergeTag { round, selected: selected as u32,
            space: step.space, side: step.side, element: step.element }, center, neighbor)
            .map_err(SourceDecimationCause::Math)
    }
    fn erase_order(&self) -> [SourceGenerationDecimationSpace; 2] {
        [SourceGenerationDecimationSpace::Rows4, SourceGenerationDecimationSpace::Rows128]
    }
}

enum GreedyCoreCause<E> { Input, Choice(E) }
struct GreedyCoreFailure<C: GreedyChoice> { choice: C, work: GreedyWork<C::Best>, cause: GreedyCoreCause<C::Error> }
struct GreedyCoreCompleted<C: GreedyChoice> {
    choice: C, output: DecimatedRows, progress: Option<SourceDecimationProgress>,
}
fn greedy_core<C: GreedyChoice>(rows128: &[f32], rows4: &[f32], target_count: usize, mut choice: C)
    -> std::result::Result<GreedyCoreCompleted<C>, GreedyCoreFailure<C>>
{
    let mut work = GreedyWork::new();
    let result = (|| {
        let n = rows128.len() / 128;
        if rows128.len() % 128 != 0 || rows4.len() != n * 4 || target_count < 2 || target_count > n
            || !choice.values_admitted(rows128, rows4, n)
        { return Err(GreedyCoreCause::Input); }
        choice.ready().map_err(GreedyCoreCause::Choice)?;
        choice.initialize(rows128, rows4, &mut work).map_err(GreedyCoreCause::Choice)?;
        work.removed_indices = Some(Vec::new());
        while work.rows4.as_ref().unwrap().len() > target_count {
            let n = work.rows4.as_ref().unwrap().len();
            let round = work.removed_indices.as_ref().unwrap().len();
            choice.begin_round(&mut work, round, n);
            choice.reserve_round(&mut work).map_err(GreedyCoreCause::Choice)?;
            for candidate in 1..n - 1 {
                if let Some(progress) = work.progress.as_mut() {
                    progress.stage = SourceDecimationStage::Candidate;
                    progress.candidate = Some(candidate as u32);
                }
                let rows = work.rows4.as_ref().unwrap();
                let best = work.round.as_ref().unwrap().best;
                let decision = choice.score(round, candidate, &rows[candidate], &rows[candidate - 1], &rows[candidate + 1], best)
                    .map_err(GreedyCoreCause::Choice)?;
                choice.commit(&mut work, candidate, decision);
            }
            choice.control(&mut work, n).map_err(GreedyCoreCause::Choice)?;
            let selected = work.round.as_ref().unwrap().chosen;
            for step in GreedyMergeSteps::new(choice.order()) {
                if let Some(progress) = work.progress.as_mut() {
                    progress.stage = SourceDecimationStage::Merge;
                    progress.merge = Some(SourceGenerationDecimationMergeTag { round, selected: selected as u32,
                        space: step.space, side: step.side, element: step.element });
                }
                let rows = match step.space {
                    SourceGenerationDecimationSpace::Rows128 => work.rows128.as_mut().unwrap(),
                    SourceGenerationDecimationSpace::Rows4 => work.rows4.as_mut().unwrap(),
                };
                let neighbor = match step.side {
                    SourceGenerationDecimationSide::Left => selected - 1,
                    SourceGenerationDecimationSide::Right => selected + 1,
                };
                let (center_bits, neighbor_bits) = match step.side {
                    SourceGenerationDecimationSide::Left => {
                        let neighbor_bits = rows[neighbor][step.element].to_bits();
                        (rows[selected][step.element].to_bits(), neighbor_bits)
                    }
                    SourceGenerationDecimationSide::Right => {
                        let center_bits = rows[selected][step.element].to_bits();
                        (center_bits, rows[neighbor][step.element].to_bits())
                    }
                };
                let word = choice.merge(round, selected, step, center_bits, neighbor_bits).map_err(GreedyCoreCause::Choice)?;
                if let Some(progress) = work.progress.as_mut() { progress.pending_word = Some(word); }
                rows[neighbor][step.element] = f32::from_bits(word);
                if let Some(progress) = work.progress.as_mut() { progress.pending_word = None; }
            }
            for space in choice.erase_order() {
                match space {
                    SourceGenerationDecimationSpace::Rows128 => {
                        if let Some(progress) = work.progress.as_mut() { progress.stage = SourceDecimationStage::Erase128; }
                        work.rows128.as_mut().unwrap().remove(selected);
                    }
                    SourceGenerationDecimationSpace::Rows4 => {
                        if let Some(progress) = work.progress.as_mut() { progress.stage = SourceDecimationStage::Erase4; }
                        work.rows4.as_mut().unwrap().remove(selected);
                    }
                }
            }
            if let Some(progress) = work.progress.as_mut() { progress.stage = SourceDecimationStage::IndexCommit; }
            work.removed_indices.as_mut().unwrap().push(selected);
            if let Some(progress) = work.progress.as_mut() { progress.current_count = n - 1; }
        }
        if let Some(progress) = work.progress.as_mut() { progress.stage = SourceDecimationStage::Complete; }
        Ok(DecimatedRows {
            rows128: work.rows128.take().unwrap().into_iter().flatten().collect(),
            rows4: work.rows4.take().unwrap().into_iter().flatten().collect(),
            removed_indices: work.removed_indices.take().unwrap(),
        })
    })();
    match result {
        Ok(output) => Ok(GreedyCoreCompleted { choice, output, progress: work.progress }),
        Err(cause) => Err(GreedyCoreFailure { choice, work, cause }),
    }
}

pub fn interpolate_rows(post: &[f32], width: usize, table: &[f32]) -> Result<Vec<f32>> {
    if width == 0
        || post.len() % width != 0
        || post.len() / width < 2
        || table.is_empty()
        || table.iter().any(|v| !v.is_finite() || *v < 0.0 || *v > 1.0)
    {
        return Err("invalid interpolation control table".into());
    }
    let rows = post.len() / width;
    let mut out = Vec::with_capacity(table.len() * width);
    for &t in table {
        let pos = t * (rows - 1) as f32;
        let lo = (pos.floor() as usize).min(rows - 1);
        let hi = (lo + 1).min(rows - 1);
        let frac = pos - lo as f32;
        for j in 0..width {
            let delta = post[hi * width + j] - post[lo * width + j];
            let scaled = delta * frac;
            out.push(post[lo * width + j] + scaled);
        }
    }
    Ok(out)
}

pub struct ResampledRows {
    pub table: Vec<f32>,
    pub points128: Vec<f32>,
    pub points4: Vec<f32>,
}

pub fn resample_decimated(rows128: &[f32], rows4: &[f32]) -> Result<ResampledRows> {
    match resample_core(rows128, rows4, HostResampleChoice) {
        Ok(completed) => Ok(completed.output),
        Err(ResampleCoreFailure::Before { error, .. }
            | ResampleCoreFailure::Arc { stop: error, .. }
            | ResampleCoreFailure::Interpolation { error, .. }) => Err(error),
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceResampleStage { Input, FitGrid, Curve, Arc, Interpolate128, Interpolate4, Complete }

pub struct SourceResamplePreparation {
    points128: Option<Vec<f32>>,
    table: Option<Vec<f32>>,
    curve: Option<GenerationCurve4>,
    fitgrid: Option<Vec<f32>>,
    fitted_rows: Option<Vec<[f32; 4]>>,
    stage: SourceResampleStage,
}
impl SourceResamplePreparation {
    pub fn stage(&self) -> SourceResampleStage { self.stage }
    pub fn fitted_rows(&self) -> Option<&[[f32; 4]]> { self.fitted_rows.as_deref() }
    pub fn fitgrid(&self) -> Option<&[f32]> { self.fitgrid.as_deref() }
    pub fn curve(&self) -> Option<&GenerationCurve4> { self.curve.as_ref() }
    pub fn table(&self) -> Option<&[f32]> { self.table.as_deref() }
    pub fn points128(&self) -> Option<&[f32]> { self.points128.as_deref() }
}

pub struct SourceResampleArcRemainder {
    pub parameters: Vec<f32>,
    pub evaluated: Vec<[f32; 4]>,
    pub squared: Vec<f32>,
    pub distances: Vec<f32>,
    pub cumulative: Vec<f32>,
    pub context: SourceGenerationArcContext,
}
pub struct SourceResampleContext {
    pub fitted_rows: Vec<[f32; 4]>,
    pub curve: GenerationCurve4,
    pub arc: SourceResampleArcRemainder,
}
pub struct SourceResampleParts {
    pub original_rows128: Vec<f32>,
    pub original_rows4: Vec<f32>,
    pub output: ResampledRows,
    pub context: SourceResampleContext,
}
pub struct SourceResampleCompleted { parts: SourceResampleParts }
impl SourceResampleCompleted {
    pub fn original_rows128(&self) -> &[f32] { &self.parts.original_rows128 }
    pub fn original_rows4(&self) -> &[f32] { &self.parts.original_rows4 }
    pub fn output(&self) -> &ResampledRows { &self.parts.output }
    pub fn context(&self) -> &SourceResampleContext { &self.parts.context }
    pub fn into_parts(self) -> SourceResampleParts { self.parts }
}
type SourceResampleCoreFailure = ResampleCoreFailure<SourceResampleChoice,
    SourceResampleArcRemainder, SourceGenerationArcFailure>;

pub struct SourceResampleFailure {
    original_rows128: Vec<f32>, original_rows4: Vec<f32>, failure: SourceResampleCoreFailure,
}
impl SourceResampleFailure {
    pub fn original_rows128(&self) -> &[f32] { &self.original_rows128 }
    pub fn original_rows4(&self) -> &[f32] { &self.original_rows4 }
    pub fn preparation(&self) -> &SourceResamplePreparation {
        match &self.failure {
            ResampleCoreFailure::Before { preparation, .. }
            | ResampleCoreFailure::Arc { preparation, .. }
            | ResampleCoreFailure::Interpolation { preparation, .. } => preparation,
        }
    }
    pub fn error(&self) -> Option<&str> {
        match &self.failure {
            ResampleCoreFailure::Before { error, .. }
            | ResampleCoreFailure::Interpolation { error, .. } => Some(error), _ => None,
        }
    }
    pub fn arc_failure(&self) -> Option<&SourceGenerationArcFailure> {
        match &self.failure { ResampleCoreFailure::Arc { stop, .. } => Some(stop), _ => None }
    }
    pub fn arc_remainder(&self) -> Option<&SourceResampleArcRemainder> {
        match &self.failure { ResampleCoreFailure::Interpolation { done, .. } => Some(done), _ => None }
    }
    pub fn math(&self) -> &SourceInterpolationRemapMath {
        match &self.failure {
            ResampleCoreFailure::Before { choice, .. } => &choice.math,
            ResampleCoreFailure::Arc { stop, .. } => stop.math(),
            ResampleCoreFailure::Interpolation { done, .. } => &done.context.math,
        }
    }
}

pub fn resample_decimated_with_source_remap_math(rows128: Vec<f32>, rows4: Vec<f32>,
    math: SourceInterpolationRemapMath) -> std::result::Result<SourceResampleCompleted, SourceResampleFailure>
{
    match resample_core(&rows128, &rows4, SourceResampleChoice { math }) {
        Ok(mut completed) => {
            let fitted_rows = completed.preparation.fitted_rows.take().unwrap();
            let curve = completed.preparation.curve.take().unwrap();
            Ok(SourceResampleCompleted { parts: SourceResampleParts {
                original_rows128: rows128, original_rows4: rows4, output: completed.output,
                context: SourceResampleContext { fitted_rows, curve, arc: completed.done } } })
        }
        Err(failure) => Err(SourceResampleFailure { original_rows128: rows128, original_rows4: rows4, failure }),
    }
}

struct PreparedResample<D> { table: Vec<f32>, done: D }
trait ResampleChoice {
    type Done;
    type Stop;
    fn run(self, curve: &GenerationCurve4, fitgrid: &mut Option<Vec<f32>>)
        -> std::result::Result<PreparedResample<Self::Done>, Self::Stop>;
}
struct HostResampleChoice;
impl ResampleChoice for HostResampleChoice {
    type Done = ();
    type Stop = String;
    fn run(self, curve: &GenerationCurve4, fitgrid: &mut Option<Vec<f32>>) -> Result<PreparedResample<()>> {
        let table = crate::generation_curve::arc_stages(curve, fitgrid.as_ref().unwrap())
            .map_err(str::to_owned)?.table;
        Ok(PreparedResample { table, done: () })
    }
}
struct SourceResampleChoice { math: SourceInterpolationRemapMath }
impl ResampleChoice for SourceResampleChoice {
    type Done = SourceResampleArcRemainder;
    type Stop = SourceGenerationArcFailure;
    fn run(self, curve: &GenerationCurve4, fitgrid: &mut Option<Vec<f32>>)
        -> std::result::Result<PreparedResample<Self::Done>, Self::Stop>
    {
        let queries = fitgrid.take().unwrap();
        let SourceGenerationArcParts { stages, context } =
            crate::generation_curve::arc_stages_with_source_remap_math(curve, queries, self.math)?.into_parts();
        let GenerationArcStages { parameters, evaluated, squared, distances, cumulative, table } = stages;
        Ok(PreparedResample { table, done: SourceResampleArcRemainder {
            parameters, evaluated, squared, distances, cumulative, context } })
    }
}
enum ResampleCoreFailure<C, D, E> {
    Before { choice: C, preparation: SourceResamplePreparation, error: String },
    Arc { stop: E, preparation: SourceResamplePreparation },
    Interpolation { done: D, preparation: SourceResamplePreparation, error: String },
}
struct ResampleCoreCompleted<D> { output: ResampledRows, preparation: SourceResamplePreparation, done: D }
fn resample_core<C: ResampleChoice>(rows128: &[f32], rows4: &[f32], choice: C)
    -> std::result::Result<ResampleCoreCompleted<C::Done>, ResampleCoreFailure<C, C::Done, C::Stop>>
{
    use crate::generation_curve::{interpolate_final_rows, unit_grid};
    let mut preparation = SourceResamplePreparation { points128: None, table: None, curve: None,
        fitgrid: None, fitted_rows: None, stage: SourceResampleStage::Input };
    let prepared: Result<()> = (|| {
        if rows4.len() % 4 != 0 || rows128.len() / 128 != rows4.len() / 4 || rows128.len() % 128 != 0 {
            return Err("post-decimation embedding dimensions do not match".into());
        }
        preparation.fitted_rows = Some(rows4.chunks_exact(4)
            .map(|r| r.try_into().expect("four-lane chunk")).collect());
        preparation.stage = SourceResampleStage::FitGrid;
        preparation.fitgrid = Some(unit_grid(preparation.fitted_rows.as_ref().unwrap().len()).map_err(str::to_owned)?);
        preparation.stage = SourceResampleStage::Curve;
        preparation.curve = Some(GenerationCurve4::from_rows(preparation.fitgrid.as_ref().unwrap(),
            preparation.fitted_rows.as_ref().unwrap(), f32::from_bits(0x3c23d70a)).map_err(str::to_owned)?);
        preparation.stage = SourceResampleStage::Arc;
        Ok(())
    })();
    if let Err(error) = prepared { return Err(ResampleCoreFailure::Before { choice, preparation, error }); }
    let PreparedResample { table, done } = match choice.run(preparation.curve.as_ref().unwrap(), &mut preparation.fitgrid) {
        Ok(prepared) => prepared,
        Err(stop) => return Err(ResampleCoreFailure::Arc { stop, preparation }),
    };
    preparation.table = Some(table);
    preparation.stage = SourceResampleStage::Interpolate128;
    let points128 = match interpolate_final_rows(rows128, 128, preparation.table.as_ref().unwrap()).map_err(str::to_owned) {
        Ok(points) => points,
        Err(error) => return Err(ResampleCoreFailure::Interpolation { done, preparation, error }),
    };
    preparation.points128 = Some(points128);
    preparation.stage = SourceResampleStage::Interpolate4;
    let points4 = match interpolate_final_rows(rows4, 4, preparation.table.as_ref().unwrap()).map_err(str::to_owned) {
        Ok(points) => points,
        Err(error) => return Err(ResampleCoreFailure::Interpolation { done, preparation, error }),
    };
    preparation.stage = SourceResampleStage::Complete;
    let output = ResampledRows { table: preparation.table.take().unwrap(),
        points128: preparation.points128.take().unwrap(), points4 };
    Ok(ResampleCoreCompleted { output, preparation, done })
}

pub struct Generator {
    producer: BankProducer,
    head: Head8,
    reducer: SourcebankReducer,
    target4: Target4,
    bank_factor: usize,
}

pub struct ComputedGeneration {
    pub rows: GeneratedRows,
    pub interpolation: Vec<f32>,
    pub schedule: crate::generation_schedule::GenerationSchedule,
    pub windows: Vec<crate::generation_schedule::AcceptedWindow>,
}
pub struct GeneratedRows {
    pub bank8: Vec<f32>,
    pub bank384: Vec<f32>,
    pub trajectory128: Vec<f32>,
    pub initial128: Vec<f32>,
    pub initial4: Vec<f32>,
    pub postdecim128: Vec<f32>,
    pub postdecim4: Vec<f32>,
    pub points128: Vec<f32>,
    pub points4: Vec<f32>,
    pub removed_indices: Vec<usize>,
}

pub struct SourceScheduledGenerationRequest {
    pub rows72: Vec<f32>,
    pub sample_count: usize,
    pub energies: Vec<f32>,
    pub math: SourceGenerationWindowMath,
}
pub struct SourceScheduledWindowContext {
    pub energies: Vec<f32>,
    pub math: SourceGenerationWindowMath,
    pub bank_factor: usize,
}
pub enum SourceScheduledGenerationParts {
    Insufficient { rows72: Vec<f32>, selection: SourceGenerationWindowsParts },
    Generated { rows72: Vec<f32>, generation: ComputedGeneration, context: SourceScheduledWindowContext },
}
pub struct SourceScheduledGenerationCompleted { parts: SourceScheduledGenerationParts }
impl SourceScheduledGenerationCompleted {
    pub fn parts(&self) -> &SourceScheduledGenerationParts { &self.parts }
    pub fn windows(&self) -> &[AcceptedWindow] {
        match &self.parts {
            SourceScheduledGenerationParts::Insufficient { selection, .. } => &selection.windows,
            SourceScheduledGenerationParts::Generated { generation, .. } => &generation.windows,
        }
    }
    pub fn math(&self) -> &SourceGenerationWindowMath {
        match &self.parts {
            SourceScheduledGenerationParts::Insufficient { selection, .. } => &selection.math,
            SourceScheduledGenerationParts::Generated { context, .. } => &context.math,
        }
    }
    pub fn into_parts(self) -> SourceScheduledGenerationParts { self.parts }
}

#[derive(Clone, Copy, Debug)]
pub enum SourceScheduledGenerationStage {
    Banks, Head, GlobalReducer, WindowExtent, WindowReducer, Target4, Decimate, Resample, Complete,
}

pub struct SourceScheduledReductionRemainder { weights: Vec<f32>, accum384: Vec<f32> }
impl SourceScheduledReductionRemainder {
    pub fn weights(&self) -> &[f32] { &self.weights }
    pub fn accum384(&self) -> &[f32] { &self.accum384 }
}
pub struct SourceScheduledWindowReductionRemainder {
    ordinal: usize, remainder: SourceScheduledReductionRemainder,
}
impl SourceScheduledWindowReductionRemainder {

    pub fn ordinal(&self) -> usize { self.ordinal }
    pub fn remainder(&self) -> &SourceScheduledReductionRemainder { &self.remainder }
}

trait ScheduledRetention {
    type Head;
    fn head(packet: TimbreStages) -> Self::Head;
    fn head_output(packet: &Self::Head) -> &Vec<f32>;
    fn global(&mut self, packet: SourcebankStages) -> Vec<f32>;
    fn window(&mut self, ordinal: usize, packet: SourcebankStages) -> Vec<f32>;
}
struct HostScheduledRetention;
impl ScheduledRetention for HostScheduledRetention {
    type Head = Vec<f32>;
    fn head(packet: TimbreStages) -> Vec<f32> { packet.out8 }
    fn head_output(packet: &Vec<f32>) -> &Vec<f32> { packet }
    fn global(&mut self, packet: SourcebankStages) -> Vec<f32> { packet.tail120 }
    fn window(&mut self, _: usize, packet: SourcebankStages) -> Vec<f32> { packet.tail120 }
}
struct SourceScheduledRetention {
    global: Option<SourceScheduledReductionRemainder>,
    window: Option<SourceScheduledWindowReductionRemainder>,
}
impl ScheduledRetention for SourceScheduledRetention {
    type Head = TimbreStages;
    fn head(packet: TimbreStages) -> TimbreStages { packet }
    fn head_output(packet: &TimbreStages) -> &Vec<f32> { &packet.out8 }
    fn global(&mut self, packet: SourcebankStages) -> Vec<f32> {
        let SourcebankStages { weights, accum384, tail120 } = packet;
        self.global = Some(SourceScheduledReductionRemainder { weights, accum384 });
        tail120
    }
    fn window(&mut self, ordinal: usize, packet: SourcebankStages) -> Vec<f32> {
        let SourcebankStages { weights, accum384, tail120 } = packet;
        self.window = Some(SourceScheduledWindowReductionRemainder { ordinal,
            remainder: SourceScheduledReductionRemainder { weights, accum384 } });
        tail120
    }
}

struct ScheduledPrefix<H, R> {
    stage: SourceScheduledGenerationStage,
    window_ordinal: Option<usize>,
    banks: Option<BankProducerStages>, head: Option<H>,
    trajectory128: Option<Vec<f32>>, initial128: Option<Vec<f32>>, initial4: Option<Vec<f32>>,
    current_row: Option<Vec<f32>>, post: Option<DecimatedRows>, retained: R,
}
struct ScheduledSuffixFailure<H, R> { prefix: ScheduledPrefix<H, R>, error: String }
struct ScheduledRows { rows: GeneratedRows, interpolation: Vec<f32> }

trait ScheduledDecimationChoice {
    type Done;
    type Stop;
    fn run<H, R>(self, prefix: &mut ScheduledPrefix<H, R>, retained_count: usize)
        -> std::result::Result<Self::Done, Self::Stop>;
}
struct HostScheduledDecimation;
impl ScheduledDecimationChoice for HostScheduledDecimation {
    type Done = ();
    type Stop = String;
    fn run<H, R>(self, prefix: &mut ScheduledPrefix<H, R>, retained_count: usize) -> Result<()> {
        let post = greedy_decimate(prefix.initial128.as_ref().unwrap(), prefix.initial4.as_ref().unwrap(), retained_count)?;
        prefix.post = Some(post);
        Ok(())
    }
}
struct SourceScheduledDecimation { math: SourceGenerationDecimationMath }
struct SourceScheduledDecimationDone { math: SourceGenerationDecimationMath, progress: SourceDecimationProgress }
impl ScheduledDecimationChoice for SourceScheduledDecimation {
    type Done = SourceScheduledDecimationDone;
    type Stop = SourceDecimationFailure;
    fn run<H, R>(self, prefix: &mut ScheduledPrefix<H, R>, retained_count: usize)
        -> std::result::Result<Self::Done, Self::Stop>
    {
        let original_rows128 = prefix.initial128.take().unwrap();
        let original_rows4 = prefix.initial4.take().unwrap();
        let SourceDecimationParts { original_rows128, original_rows4, output, math, progress } =
            greedy_decimate_with_source_math(original_rows128, original_rows4, retained_count, self.math)?.into_parts();
        prefix.initial128 = Some(original_rows128);
        prefix.initial4 = Some(original_rows4);
        prefix.post = Some(output);
        Ok(SourceScheduledDecimationDone { math, progress })
    }
}
struct ScheduledResamplePrepared<D> { output: ResampledRows, done: D }
trait ScheduledResampleChoice {
    type Done;
    type Stop;
    fn run<H, T>(self, prefix: &mut ScheduledPrefix<H, T>)
        -> std::result::Result<ScheduledResamplePrepared<Self::Done>, Self::Stop>;
}
struct HostScheduledResample;
impl ScheduledResampleChoice for HostScheduledResample {
    type Done = ();
    type Stop = String;
    fn run<H, T>(self, prefix: &mut ScheduledPrefix<H, T>) -> Result<ScheduledResamplePrepared<()>> {
        let post = prefix.post.as_ref().unwrap();
        let output = resample_decimated(&post.rows128, &post.rows4)?;
        Ok(ScheduledResamplePrepared { output, done: () })
    }
}
struct SourceScheduledResample { math: SourceInterpolationRemapMath }
struct SourceScheduledResampleStop { failure: SourceResampleFailure, removed_indices: Vec<usize> }
impl ScheduledResampleChoice for SourceScheduledResample {
    type Done = SourceResampleContext;
    type Stop = SourceScheduledResampleStop;
    fn run<H, T>(self, prefix: &mut ScheduledPrefix<H, T>)
        -> std::result::Result<ScheduledResamplePrepared<Self::Done>, Self::Stop>
    {
        let DecimatedRows { rows128, rows4, removed_indices } = prefix.post.take().unwrap();
        let parts = match resample_decimated_with_source_remap_math(rows128, rows4, self.math) {
            Ok(completed) => completed.into_parts(),
            Err(failure) => return Err(SourceScheduledResampleStop { failure, removed_indices }),
        };
        let SourceResampleParts { original_rows128, original_rows4, output, context } = parts;
        prefix.post = Some(DecimatedRows { rows128: original_rows128, rows4: original_rows4, removed_indices });
        Ok(ScheduledResamplePrepared { output, done: context })
    }
}
enum FinalizeStop<D, E, R = HostScheduledResample, RE = String> {
    Decimation { failure: E, resample: R },
    Resample { done: D, failure: RE },
}
#[derive(Clone, Copy)]
enum TargetTiming { Immediate, Deferred }

fn scheduled_prefix<R: ScheduledRetention>(retained: R) -> ScheduledPrefix<R::Head, R> {
    ScheduledPrefix { stage: SourceScheduledGenerationStage::Banks,
        window_ordinal: None, banks: None, head: None, trajectory128: None,
        initial128: None, initial4: None, current_row: None, post: None, retained }
}

pub struct SourceScheduledGenerationProgress {
    prefix: ScheduledPrefix<TimbreStages, SourceScheduledRetention>,
}
impl SourceScheduledGenerationProgress {
    pub fn stage(&self) -> SourceScheduledGenerationStage { self.prefix.stage }

    pub fn window_ordinal(&self) -> Option<usize> { self.prefix.window_ordinal }
    pub fn banks(&self) -> Option<&BankProducerStages> { self.prefix.banks.as_ref() }
    pub fn head(&self) -> Option<&TimbreStages> { self.prefix.head.as_ref() }
    pub fn trajectory128(&self) -> Option<&[f32]> { self.prefix.trajectory128.as_deref() }
    pub fn initial128(&self) -> Option<&[f32]> { self.prefix.initial128.as_deref() }
    pub fn initial4(&self) -> Option<&[f32]> { self.prefix.initial4.as_deref() }
    pub fn current_row(&self) -> Option<&[f32]> { self.prefix.current_row.as_deref() }
    pub fn postdecimation(&self) -> Option<&DecimatedRows> { self.prefix.post.as_ref() }
    pub fn global_reduction(&self) -> Option<&SourceScheduledReductionRemainder> {
        self.prefix.retained.global.as_ref()
    }

    pub fn window_reduction(&self) -> Option<&SourceScheduledWindowReductionRemainder> {
        self.prefix.retained.window.as_ref()
    }
}

enum SourceScheduledFailureOwner {
    Before { request: SourceScheduledGenerationRequest, error: String },
    Selection { rows72: Vec<f32>, failure: SourceGenerationWindowsFailure },
    Later { rows72: Vec<f32>, selection: SourceGenerationWindowsParts,
        progress: SourceScheduledGenerationProgress, error: String },
}
pub enum SourceScheduledGenerationError<'a> {
    Before(&'a str), Selection(&'a SourceGenerationWindowsFailure), Later(&'a str),
}
pub struct SourceScheduledGenerationFailure { owner: SourceScheduledFailureOwner }
impl SourceScheduledGenerationFailure {
    pub fn error(&self) -> SourceScheduledGenerationError<'_> {
        match &self.owner {
            SourceScheduledFailureOwner::Before { error, .. } => SourceScheduledGenerationError::Before(error),
            SourceScheduledFailureOwner::Selection { failure, .. } => SourceScheduledGenerationError::Selection(failure),
            SourceScheduledFailureOwner::Later { error, .. } => SourceScheduledGenerationError::Later(error),
        }
    }
    pub fn rows72(&self) -> &[f32] {
        match &self.owner {
            SourceScheduledFailureOwner::Before { request, .. } => &request.rows72,
            SourceScheduledFailureOwner::Selection { rows72, .. }
            | SourceScheduledFailureOwner::Later { rows72, .. } => rows72,
        }
    }
    pub fn request(&self) -> Option<&SourceScheduledGenerationRequest> {
        match &self.owner { SourceScheduledFailureOwner::Before { request, .. } => Some(request), _ => None }
    }
    pub fn selection_failure(&self) -> Option<&SourceGenerationWindowsFailure> {
        match &self.owner { SourceScheduledFailureOwner::Selection { failure, .. } => Some(failure), _ => None }
    }
    pub fn selection(&self) -> Option<&SourceGenerationWindowsParts> {
        match &self.owner { SourceScheduledFailureOwner::Later { selection, .. } => Some(selection), _ => None }
    }
    pub fn math(&self) -> &SourceGenerationWindowMath {
        match &self.owner {
            SourceScheduledFailureOwner::Before { request, .. } => &request.math,
            SourceScheduledFailureOwner::Selection { failure, .. } => failure.math(),
            SourceScheduledFailureOwner::Later { selection, .. } => &selection.math,
        }
    }
    pub fn energies(&self) -> &[f32] {
        match &self.owner {
            SourceScheduledFailureOwner::Before { request, .. } => &request.energies,
            SourceScheduledFailureOwner::Selection { failure, .. } => failure.energies(),
            SourceScheduledFailureOwner::Later { selection, .. } => &selection.energies,
        }
    }
    pub fn windows(&self) -> Option<&[AcceptedWindow]> {
        match &self.owner {
            SourceScheduledFailureOwner::Before { .. } => None,
            SourceScheduledFailureOwner::Selection { failure, .. } => Some(failure.windows()),
            SourceScheduledFailureOwner::Later { selection, .. } => Some(&selection.windows),
        }
    }
    pub fn progress(&self) -> Option<&SourceScheduledGenerationProgress> {
        match &self.owner { SourceScheduledFailureOwner::Later { progress, .. } => Some(progress), _ => None }
    }
}

fn scheduled_input(rows72: &[f32], sample_count: usize) -> Result<GenerationSchedule> {
    let schedule = GenerationSchedule::from_normalized_sample_count(sample_count)?;
    let expected = sample_count.checked_add(272).ok_or("feature frame count overflow")?.div_ceil(240);
    if rows72.len() % 72 != 0 || rows72.len() / 72 != expected {
        return Err("whole-final feature rows do not match normalized sample count".into());
    }
    Ok(schedule)
}
fn source_scheduled_input(request: &SourceScheduledGenerationRequest) -> Result<GenerationSchedule> {
    let schedule = scheduled_input(&request.rows72, request.sample_count)?;
    if request.energies.len() != request.rows72.len() / 72 {
        return Err("source window energies do not match feature row count".into());
    }
    Ok(schedule)
}

#[derive(Clone, Copy, Debug)]
pub enum SourceInterleavedGenerationStage {
    Prelude, PrepareWindow, Assemble128, CopyStorage, Copy128, TargetStorage, Target4, Finalize,
}

pub struct SourceInterleavedPrepared {
    progress: SourceScheduledGenerationProgress,
    stage: SourceInterleavedGenerationStage,
    copied128: Option<Vec<f32>>,
    target_input_range: Option<(usize, usize)>,
}
impl SourceInterleavedPrepared {
    pub fn progress(&self) -> &SourceScheduledGenerationProgress { &self.progress }
    pub fn stage(&self) -> SourceInterleavedGenerationStage { self.stage }
    pub fn copied128(&self) -> Option<&[f32]> { self.copied128.as_deref() }
    pub fn target_input_range(&self) -> Option<(usize, usize)> { self.target_input_range }
    pub fn target_input(&self) -> Option<&[f32]> {
        self.target_input_range.map(|(start, end)| &self.copied128.as_ref().unwrap()[start..end])
    }
}
pub enum SourceInterleavedGenerationParts {
    Insufficient { rows72: Vec<f32>, selection: SourceGenerationWindowsParts, prepared: SourceInterleavedPrepared },
    Generated { rows72: Vec<f32>, generation: ComputedGeneration, context: SourceScheduledWindowContext },
}
pub struct SourceInterleavedGenerationCompleted { parts: SourceInterleavedGenerationParts }
impl SourceInterleavedGenerationCompleted {
    pub fn parts(&self) -> &SourceInterleavedGenerationParts { &self.parts }
    pub fn windows(&self) -> &[AcceptedWindow] {
        match &self.parts {
            SourceInterleavedGenerationParts::Insufficient { selection, .. } => &selection.windows,
            SourceInterleavedGenerationParts::Generated { generation, .. } => &generation.windows,
        }
    }
    pub fn math(&self) -> &SourceGenerationWindowMath {
        match &self.parts {
            SourceInterleavedGenerationParts::Insufficient { selection, .. } => &selection.math,
            SourceInterleavedGenerationParts::Generated { context, .. } => &context.math,
        }
    }
    pub fn into_parts(self) -> SourceInterleavedGenerationParts { self.parts }
}

struct InterleavedOwner<C = HostScheduledDecimation, R = HostScheduledResample> {
    rows72: Vec<f32>, prepared: SourceInterleavedPrepared, decimation: C, resample: R,
}
enum InterleavedConsumerCause { Storage(TryReserveError), Model(String) }
enum SourceInterleavedFailureOwner<C = HostScheduledDecimation, D = (), E = String,
    R = HostScheduledResample, RE = String>
{
    Before { request: SourceScheduledGenerationRequest, decimation: C, resample: R, error: String },
    Prelude { request: SourceScheduledGenerationRequest, prepared: SourceInterleavedPrepared,
        decimation: C, resample: R, error: String },
    Interrupted(InterruptedSourceWindows<InterleavedOwner<C, R>, InterleavedConsumerCause>),
    PostSelection { rows72: Vec<f32>, selection: SourceGenerationWindowsParts,
        prepared: SourceInterleavedPrepared, decimation: C, resample: R, cause: InterleavedConsumerCause },
    Finalize { rows72: Vec<f32>, selection: SourceGenerationWindowsParts,
        prepared: SourceInterleavedPrepared, failure: FinalizeStop<D, E, R, RE> },
}
#[derive(Clone, Copy, Debug)]
pub enum SourceInterleavedFailurePhase { Before, Prelude, Selector, Prepare, Accept, PostSelection }
#[derive(Clone, Copy, Debug)]
pub enum SourceInterleavedSelectorCause { Input, Extent, Start, End, Math }

pub struct SourceInterleavedGenerationFailure { owner: SourceInterleavedFailureOwner }
impl<C, D, E, R, RE> SourceInterleavedFailureOwner<C, D, E, R, RE> {
    pub fn phase(&self) -> SourceInterleavedFailurePhase {
        match self {
            SourceInterleavedFailureOwner::Before { .. } => SourceInterleavedFailurePhase::Before,
            SourceInterleavedFailureOwner::Prelude { .. } => SourceInterleavedFailurePhase::Prelude,
            SourceInterleavedFailureOwner::Interrupted(failure) => match failure.cause() {
                SourceWindowConsumptionCause::Consumer { phase: ConsumerPhase::Prepare, .. } => SourceInterleavedFailurePhase::Prepare,
                SourceWindowConsumptionCause::Consumer { phase: ConsumerPhase::Accept, .. } => SourceInterleavedFailurePhase::Accept,
                _ => SourceInterleavedFailurePhase::Selector,
            },
            SourceInterleavedFailureOwner::PostSelection { .. }
            | SourceInterleavedFailureOwner::Finalize { .. } => SourceInterleavedFailurePhase::PostSelection,
        }
    }
    pub fn request(&self) -> Option<&SourceScheduledGenerationRequest> {
        match self {
            SourceInterleavedFailureOwner::Before { request, .. }
            | SourceInterleavedFailureOwner::Prelude { request, .. } => Some(request), _ => None,
        }
    }
    pub fn rows72(&self) -> &[f32] {
        match self {
            SourceInterleavedFailureOwner::Before { request, .. }
            | SourceInterleavedFailureOwner::Prelude { request, .. } => &request.rows72,
            SourceInterleavedFailureOwner::Interrupted(failure) => &failure.owner().rows72,
            SourceInterleavedFailureOwner::PostSelection { rows72, .. }
            | SourceInterleavedFailureOwner::Finalize { rows72, .. } => rows72,
        }
    }
    pub fn math(&self) -> &SourceGenerationWindowMath {
        match self {
            SourceInterleavedFailureOwner::Before { request, .. }
            | SourceInterleavedFailureOwner::Prelude { request, .. } => &request.math,
            SourceInterleavedFailureOwner::Interrupted(failure) => failure.math(),
            SourceInterleavedFailureOwner::PostSelection { selection, .. }
            | SourceInterleavedFailureOwner::Finalize { selection, .. } => &selection.math,
        }
    }
    pub fn energies(&self) -> &[f32] {
        match self {
            SourceInterleavedFailureOwner::Before { request, .. }
            | SourceInterleavedFailureOwner::Prelude { request, .. } => &request.energies,
            SourceInterleavedFailureOwner::Interrupted(failure) => failure.energies(),
            SourceInterleavedFailureOwner::PostSelection { selection, .. }
            | SourceInterleavedFailureOwner::Finalize { selection, .. } => &selection.energies,
        }
    }
    pub fn windows(&self) -> Option<&[AcceptedWindow]> {
        match self {
            SourceInterleavedFailureOwner::Interrupted(failure) => Some(failure.windows()),
            SourceInterleavedFailureOwner::PostSelection { selection, .. }
            | SourceInterleavedFailureOwner::Finalize { selection, .. } => Some(&selection.windows), _ => None,
        }
    }
    pub fn completed_selection(&self) -> Option<&SourceGenerationWindowsParts> {
        match self {
            SourceInterleavedFailureOwner::PostSelection { selection, .. }
            | SourceInterleavedFailureOwner::Finalize { selection, .. } => Some(selection), _ => None,
        }
    }
    pub fn schedule(&self) -> Option<GenerationSchedule> {
        match self {
            SourceInterleavedFailureOwner::Interrupted(failure) => Some(failure.schedule()),
            SourceInterleavedFailureOwner::PostSelection { selection, .. }
            | SourceInterleavedFailureOwner::Finalize { selection, .. } => Some(selection.schedule), _ => None,
        }
    }
    pub fn bank_factor(&self) -> Option<usize> {
        match self {
            SourceInterleavedFailureOwner::Interrupted(failure) => Some(failure.bank_factor()),
            SourceInterleavedFailureOwner::PostSelection { selection, .. }
            | SourceInterleavedFailureOwner::Finalize { selection, .. } => Some(selection.bank_factor), _ => None,
        }
    }

    pub fn current_selection(&self) -> Option<SourceGenerationWindow> {
        match self { SourceInterleavedFailureOwner::Interrupted(failure) => failure.current(), _ => None }
    }
    pub fn selected_ordinal(&self) -> Option<usize> {
        match self { SourceInterleavedFailureOwner::Interrupted(failure) => failure.selected_ordinal(), _ => None }
    }
    pub fn accepted_current(&self) -> Option<bool> {
        match self { SourceInterleavedFailureOwner::Interrupted(failure) => Some(failure.accepted_current()), _ => None }
    }
    pub fn prepared(&self) -> Option<&SourceInterleavedPrepared> {
        match self {
            SourceInterleavedFailureOwner::Prelude { prepared, .. }
            | SourceInterleavedFailureOwner::PostSelection { prepared, .. }
            | SourceInterleavedFailureOwner::Finalize { prepared, .. } => Some(prepared),
            SourceInterleavedFailureOwner::Interrupted(failure) => Some(&failure.owner().prepared), _ => None,
        }
    }
    fn error_with<'a>(&'a self, finalize: impl FnOnce(&'a FinalizeStop<D, E, R, RE>) -> Option<&'a str>) -> Option<&'a str> {
        match self {
            SourceInterleavedFailureOwner::Before { error, .. }
            | SourceInterleavedFailureOwner::Prelude { error, .. } => Some(error),
            SourceInterleavedFailureOwner::Interrupted(failure) => match failure.cause() {
                SourceWindowConsumptionCause::Consumer { error: InterleavedConsumerCause::Model(error), .. } => Some(error), _ => None,
            },
            SourceInterleavedFailureOwner::PostSelection { cause: InterleavedConsumerCause::Model(error), .. } => Some(error),
            SourceInterleavedFailureOwner::Finalize { failure, .. } => finalize(failure),
            _ => None,
        }
    }
    pub fn storage_error(&self) -> Option<&TryReserveError> {
        match self {
            SourceInterleavedFailureOwner::Interrupted(failure) => match failure.cause() {
                SourceWindowConsumptionCause::WindowStorage(error)
                | SourceWindowConsumptionCause::Consumer { error: InterleavedConsumerCause::Storage(error), .. } => Some(error), _ => None,
            },
            SourceInterleavedFailureOwner::PostSelection { cause: InterleavedConsumerCause::Storage(error), .. } => Some(error), _ => None,
        }
    }
    pub fn selector_cause(&self) -> Option<SourceInterleavedSelectorCause> {
        match self {
            SourceInterleavedFailureOwner::Interrupted(failure) => match failure.cause() {
                SourceWindowConsumptionCause::Input => Some(SourceInterleavedSelectorCause::Input),
                SourceWindowConsumptionCause::Extent => Some(SourceInterleavedSelectorCause::Extent),
                SourceWindowConsumptionCause::Start => Some(SourceInterleavedSelectorCause::Start),
                SourceWindowConsumptionCause::End => Some(SourceInterleavedSelectorCause::End),
                SourceWindowConsumptionCause::Math => Some(SourceInterleavedSelectorCause::Math), _ => None,
            }, _ => None,
        }
    }
}

impl SourceInterleavedGenerationFailure {
    pub fn phase(&self) -> SourceInterleavedFailurePhase { self.owner.phase() }
    pub fn request(&self) -> Option<&SourceScheduledGenerationRequest> { self.owner.request() }
    pub fn rows72(&self) -> &[f32] { self.owner.rows72() }
    pub fn math(&self) -> &SourceGenerationWindowMath { self.owner.math() }
    pub fn energies(&self) -> &[f32] { self.owner.energies() }
    pub fn windows(&self) -> Option<&[AcceptedWindow]> { self.owner.windows() }
    pub fn completed_selection(&self) -> Option<&SourceGenerationWindowsParts> { self.owner.completed_selection() }
    pub fn schedule(&self) -> Option<GenerationSchedule> { self.owner.schedule() }
    pub fn bank_factor(&self) -> Option<usize> { self.owner.bank_factor() }
    pub fn current_selection(&self) -> Option<SourceGenerationWindow> { self.owner.current_selection() }
    pub fn selected_ordinal(&self) -> Option<usize> { self.owner.selected_ordinal() }
    pub fn accepted_current(&self) -> Option<bool> { self.owner.accepted_current() }
    pub fn prepared(&self) -> Option<&SourceInterleavedPrepared> { self.owner.prepared() }
    pub fn error(&self) -> Option<&str> {
        self.owner.error_with(|failure| match failure {
            FinalizeStop::Decimation { failure: error, .. } | FinalizeStop::Resample { failure: error, .. } => Some(error),
        })
    }
    pub fn storage_error(&self) -> Option<&TryReserveError> { self.owner.storage_error() }
    pub fn selector_cause(&self) -> Option<SourceInterleavedSelectorCause> { self.owner.selector_cause() }
}

pub struct SourceInterleavedDecimationGenerationRequest {
    pub generation: SourceScheduledGenerationRequest,
    pub decimation_math: SourceGenerationDecimationMath,
}
pub struct SourceAnalysisGenerationRequest<U: ScalarUpper> {
    pub analysis: SourceMathJoinAnalysis<U>,
    pub sample_count: usize,
    pub window_math: SourceGenerationWindowMath,
    pub decimation_math: SourceGenerationDecimationMath,
}

pub struct SourceAnalysisGenerationContext<U: ScalarUpper> {
    pub network72: Vec<f32>,
    pub paired72: Vec<f32>,
    pub records: Vec<PitchRecord>,
    pub source_rows: usize,
    pub pitch_rows: usize,
    pub pending: (usize, usize),
    pub spectrum_math: crate::audio_spectrum::SourceSpectrumMath,
    pub postlog_math: crate::pitch_audio_spectrum::SourcePitchPostlogMath,
    pub join_math: SourceJoinMath<U>,
}
pub struct SourceAnalysisGenerationParts<U: ScalarUpper> {
    pub generation: SourceInterleavedDecimationGenerationParts,
    pub context: SourceAnalysisGenerationContext<U>,
}
pub struct SourceAnalysisGenerationCompleted<U: ScalarUpper> {
    generation: SourceInterleavedDecimationGenerationCompleted,
    context: SourceAnalysisGenerationContext<U>,
}
impl<U: ScalarUpper> SourceAnalysisGenerationCompleted<U> {
    pub fn generation(&self) -> &SourceInterleavedDecimationGenerationCompleted { &self.generation }
    pub fn context(&self) -> &SourceAnalysisGenerationContext<U> { &self.context }
    pub fn into_parts(self) -> SourceAnalysisGenerationParts<U> {
        SourceAnalysisGenerationParts { generation: self.generation.into_parts(), context: self.context }
    }
}

pub struct SourceAnalysisGenerationFailure<U: ScalarUpper> {
    generation_failure: SourceInterleavedDecimationGenerationFailure,
    context: SourceAnalysisGenerationContext<U>,
}
impl<U: ScalarUpper> SourceAnalysisGenerationFailure<U> {
    pub fn generation_failure(&self) -> &SourceInterleavedDecimationGenerationFailure { &self.generation_failure }
    pub fn context(&self) -> &SourceAnalysisGenerationContext<U> { &self.context }
}
pub struct SourceAnalysisRemapGenerationRequest<U: ScalarUpper> {
    pub generation: SourceAnalysisGenerationRequest<U>,
    pub remap_math: SourceInterpolationRemapMath,
}
pub struct SourceAnalysisRemapGenerationParts<U: ScalarUpper> {
    pub generation: SourceInterleavedRemapGenerationParts,
    pub context: SourceAnalysisGenerationContext<U>,
}
pub struct SourceAnalysisRemapGenerationCompleted<U: ScalarUpper> {
    generation: SourceInterleavedRemapGenerationCompleted,
    context: SourceAnalysisGenerationContext<U>,
}
impl<U: ScalarUpper> SourceAnalysisRemapGenerationCompleted<U> {
    pub fn generation(&self) -> &SourceInterleavedRemapGenerationCompleted { &self.generation }
    pub fn context(&self) -> &SourceAnalysisGenerationContext<U> { &self.context }
    pub fn into_parts(self) -> SourceAnalysisRemapGenerationParts<U> {
        SourceAnalysisRemapGenerationParts { generation: self.generation.into_parts(), context: self.context }
    }
}

pub struct SourceAnalysisRemapGenerationFailure<U: ScalarUpper> {
    generation_failure: SourceInterleavedRemapGenerationFailure,
    context: SourceAnalysisGenerationContext<U>,
}
impl<U: ScalarUpper> SourceAnalysisRemapGenerationFailure<U> {
    pub fn generation_failure(&self) -> &SourceInterleavedRemapGenerationFailure { &self.generation_failure }
    pub fn context(&self) -> &SourceAnalysisGenerationContext<U> { &self.context }
}
pub struct SourceInterleavedDecimationGenerationContext {
    pub window: SourceScheduledWindowContext,
    pub decimation_math: SourceGenerationDecimationMath,
    pub decimation_progress: SourceDecimationProgress,
}
pub enum SourceInterleavedDecimationGenerationParts {
    Insufficient { rows72: Vec<f32>, selection: SourceGenerationWindowsParts,
        prepared: SourceInterleavedPrepared, decimation_math: SourceGenerationDecimationMath },
    Generated { rows72: Vec<f32>, generation: ComputedGeneration,
        context: SourceInterleavedDecimationGenerationContext },
}
pub struct SourceInterleavedDecimationGenerationCompleted { parts: SourceInterleavedDecimationGenerationParts }
impl SourceInterleavedDecimationGenerationCompleted {
    pub fn parts(&self) -> &SourceInterleavedDecimationGenerationParts { &self.parts }
    pub fn windows(&self) -> &[AcceptedWindow] {
        match &self.parts {
            SourceInterleavedDecimationGenerationParts::Insufficient { selection, .. } => &selection.windows,
            SourceInterleavedDecimationGenerationParts::Generated { generation, .. } => &generation.windows,
        }
    }
    pub fn math(&self) -> &SourceGenerationWindowMath {
        match &self.parts {
            SourceInterleavedDecimationGenerationParts::Insufficient { selection, .. } => &selection.math,
            SourceInterleavedDecimationGenerationParts::Generated { context, .. } => &context.window.math,
        }
    }
    pub fn decimation_math(&self) -> &SourceGenerationDecimationMath {
        match &self.parts {
            SourceInterleavedDecimationGenerationParts::Insufficient { decimation_math, .. } => decimation_math,
            SourceInterleavedDecimationGenerationParts::Generated { context, .. } => &context.decimation_math,
        }
    }
    pub fn into_parts(self) -> SourceInterleavedDecimationGenerationParts { self.parts }
}
type SourceDecimationGenerationOwner = SourceInterleavedFailureOwner<SourceScheduledDecimation,
    SourceScheduledDecimationDone, SourceDecimationFailure>;

pub struct SourceInterleavedDecimationGenerationFailure { owner: SourceDecimationGenerationOwner }
impl SourceInterleavedDecimationGenerationFailure {
    pub fn phase(&self) -> SourceInterleavedFailurePhase { self.owner.phase() }
    pub fn request(&self) -> Option<&SourceScheduledGenerationRequest> { self.owner.request() }
    pub fn rows72(&self) -> &[f32] { self.owner.rows72() }
    pub fn math(&self) -> &SourceGenerationWindowMath { self.owner.math() }
    pub fn energies(&self) -> &[f32] { self.owner.energies() }
    pub fn windows(&self) -> Option<&[AcceptedWindow]> { self.owner.windows() }
    pub fn completed_selection(&self) -> Option<&SourceGenerationWindowsParts> { self.owner.completed_selection() }
    pub fn schedule(&self) -> Option<GenerationSchedule> { self.owner.schedule() }
    pub fn bank_factor(&self) -> Option<usize> { self.owner.bank_factor() }
    pub fn current_selection(&self) -> Option<SourceGenerationWindow> { self.owner.current_selection() }
    pub fn selected_ordinal(&self) -> Option<usize> { self.owner.selected_ordinal() }
    pub fn accepted_current(&self) -> Option<bool> { self.owner.accepted_current() }
    pub fn prepared(&self) -> Option<&SourceInterleavedPrepared> { self.owner.prepared() }
    pub fn error(&self) -> Option<&str> {
        self.owner.error_with(|failure| match failure {
            FinalizeStop::Decimation { .. } => None,
            FinalizeStop::Resample { failure: error, .. } => Some(error),
        })
    }
    pub fn storage_error(&self) -> Option<&TryReserveError> { self.owner.storage_error() }
    pub fn selector_cause(&self) -> Option<SourceInterleavedSelectorCause> { self.owner.selector_cause() }
    pub fn decimation_failure(&self) -> Option<&SourceDecimationFailure> {
        match &self.owner {
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Decimation { failure, .. }, .. } => Some(failure),
            _ => None,
        }
    }

    pub fn decimation_math(&self) -> &SourceGenerationDecimationMath {
        match &self.owner {
            SourceInterleavedFailureOwner::Before { decimation, .. }
            | SourceInterleavedFailureOwner::Prelude { decimation, .. }
            | SourceInterleavedFailureOwner::PostSelection { decimation, .. } => &decimation.math,
            SourceInterleavedFailureOwner::Interrupted(failure) => &failure.owner().decimation.math,
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Decimation { failure, .. }, .. } => failure.math(),
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { done, .. }, .. } => &done.math,
        }
    }

    pub fn decimation_progress(&self) -> Option<&SourceDecimationProgress> {
        match &self.owner {
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { done, .. }, .. } => Some(&done.progress),
            _ => None,
        }
    }
}

pub struct SourceInterleavedRemapGenerationRequest {
    pub generation: SourceInterleavedDecimationGenerationRequest,
    pub remap_math: SourceInterpolationRemapMath,
}
pub struct SourceInterleavedRemapGenerationContext {
    pub window: SourceScheduledWindowContext,
    pub decimation_math: SourceGenerationDecimationMath,
    pub decimation_progress: SourceDecimationProgress,
    pub resample: SourceResampleContext,
}
pub enum SourceInterleavedRemapGenerationParts {
    Insufficient { rows72: Vec<f32>, selection: SourceGenerationWindowsParts, prepared: SourceInterleavedPrepared,
        decimation_math: SourceGenerationDecimationMath, remap_math: SourceInterpolationRemapMath },
    Generated { rows72: Vec<f32>, generation: ComputedGeneration, context: SourceInterleavedRemapGenerationContext },
}
pub struct SourceInterleavedRemapGenerationCompleted { parts: SourceInterleavedRemapGenerationParts }
impl SourceInterleavedRemapGenerationCompleted {
    pub fn parts(&self) -> &SourceInterleavedRemapGenerationParts { &self.parts }
    pub fn windows(&self) -> &[AcceptedWindow] {
        match &self.parts {
            SourceInterleavedRemapGenerationParts::Insufficient { selection, .. } => &selection.windows,
            SourceInterleavedRemapGenerationParts::Generated { generation, .. } => &generation.windows,
        }
    }
    pub fn math(&self) -> &SourceGenerationWindowMath {
        match &self.parts {
            SourceInterleavedRemapGenerationParts::Insufficient { selection, .. } => &selection.math,
            SourceInterleavedRemapGenerationParts::Generated { context, .. } => &context.window.math,
        }
    }
    pub fn decimation_math(&self) -> &SourceGenerationDecimationMath {
        match &self.parts {
            SourceInterleavedRemapGenerationParts::Insufficient { decimation_math, .. } => decimation_math,
            SourceInterleavedRemapGenerationParts::Generated { context, .. } => &context.decimation_math,
        }
    }
    pub fn remap_math(&self) -> &SourceInterpolationRemapMath {
        match &self.parts {
            SourceInterleavedRemapGenerationParts::Insufficient { remap_math, .. } => remap_math,
            SourceInterleavedRemapGenerationParts::Generated { context, .. } => &context.resample.arc.context.math,
        }
    }
    pub fn into_parts(self) -> SourceInterleavedRemapGenerationParts { self.parts }
}
type SourceRemapGenerationOwner = SourceInterleavedFailureOwner<SourceScheduledDecimation,
    SourceScheduledDecimationDone, SourceDecimationFailure, SourceScheduledResample, SourceScheduledResampleStop>;

pub struct SourceInterleavedRemapGenerationFailure { owner: SourceRemapGenerationOwner }
impl SourceInterleavedRemapGenerationFailure {
    pub fn phase(&self) -> SourceInterleavedFailurePhase { self.owner.phase() }
    pub fn request(&self) -> Option<&SourceScheduledGenerationRequest> { self.owner.request() }
    pub fn rows72(&self) -> &[f32] { self.owner.rows72() }
    pub fn math(&self) -> &SourceGenerationWindowMath { self.owner.math() }
    pub fn energies(&self) -> &[f32] { self.owner.energies() }
    pub fn windows(&self) -> Option<&[AcceptedWindow]> { self.owner.windows() }
    pub fn completed_selection(&self) -> Option<&SourceGenerationWindowsParts> { self.owner.completed_selection() }
    pub fn schedule(&self) -> Option<GenerationSchedule> { self.owner.schedule() }
    pub fn bank_factor(&self) -> Option<usize> { self.owner.bank_factor() }
    pub fn current_selection(&self) -> Option<SourceGenerationWindow> { self.owner.current_selection() }
    pub fn selected_ordinal(&self) -> Option<usize> { self.owner.selected_ordinal() }
    pub fn accepted_current(&self) -> Option<bool> { self.owner.accepted_current() }
    pub fn prepared(&self) -> Option<&SourceInterleavedPrepared> { self.owner.prepared() }
    pub fn error(&self) -> Option<&str> {
        self.owner.error_with(|failure| match failure {
            FinalizeStop::Decimation { .. } => None,
            FinalizeStop::Resample { failure, .. } => failure.failure.error(),
        })
    }
    pub fn storage_error(&self) -> Option<&TryReserveError> { self.owner.storage_error() }
    pub fn selector_cause(&self) -> Option<SourceInterleavedSelectorCause> { self.owner.selector_cause() }
    pub fn decimation_failure(&self) -> Option<&SourceDecimationFailure> {
        match &self.owner {
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Decimation { failure, .. }, .. } => Some(failure),
            _ => None,
        }
    }
    pub fn resample_failure(&self) -> Option<&SourceResampleFailure> {
        match &self.owner {
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { failure, .. }, .. } => Some(&failure.failure),
            _ => None,
        }
    }
    pub fn decimation_math(&self) -> &SourceGenerationDecimationMath {
        match &self.owner {
            SourceInterleavedFailureOwner::Before { decimation, .. }
            | SourceInterleavedFailureOwner::Prelude { decimation, .. }
            | SourceInterleavedFailureOwner::PostSelection { decimation, .. } => &decimation.math,
            SourceInterleavedFailureOwner::Interrupted(failure) => &failure.owner().decimation.math,
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Decimation { failure, .. }, .. } => failure.math(),
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { done, .. }, .. } => &done.math,
        }
    }

    pub fn remap_math(&self) -> &SourceInterpolationRemapMath {
        match &self.owner {
            SourceInterleavedFailureOwner::Before { resample, .. }
            | SourceInterleavedFailureOwner::Prelude { resample, .. }
            | SourceInterleavedFailureOwner::PostSelection { resample, .. } => &resample.math,
            SourceInterleavedFailureOwner::Interrupted(failure) => &failure.owner().resample.math,
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Decimation { resample, .. }, .. } => &resample.math,
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { failure, .. }, .. } => failure.failure.math(),
        }
    }
    pub fn decimation_progress(&self) -> Option<&SourceDecimationProgress> {
        match &self.owner {
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { done, .. }, .. } => Some(&done.progress),
            _ => None,
        }
    }

    pub fn removed_indices(&self) -> Option<&[usize]> {
        match &self.owner {
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Decimation { failure, .. }, .. } => failure.removed_indices(),
            SourceInterleavedFailureOwner::Finalize { failure: FinalizeStop::Resample { failure, .. }, .. } => Some(&failure.removed_indices),
            _ => self.prepared().and_then(|prepared| prepared.progress().postdecimation())
                .map(|post| post.removed_indices.as_slice()),
        }
    }
}

enum InterleavedCompleted<C, D, R = HostScheduledResample, RD = ()> {
    Insufficient { rows72: Vec<f32>, selection: SourceGenerationWindowsParts,
        prepared: SourceInterleavedPrepared, decimation: C, resample: R },
    Generated { rows72: Vec<f32>, generation: ComputedGeneration,
        window: SourceScheduledWindowContext, done: D, resampled: RD },
}

impl Generator {
    pub fn from_dnni(model: &[u8]) -> Result<Self> {
        let tree = crate::model::ModelTree::parse(model)?;
        let producer = tree
            .root
            .children
            .get(3)
            .and_then(|p| p.children.get(1))
            .ok_or("bank producer controls missing")?;
        producer.require(0x7fca41ff, b"", &[0x3edd783df01e3990])?;
        if producer.data.len() != 10 || producer.children.len() != 4 {
            return Err("bank producer control layout".into());
        }
        let fields: Vec<_> = producer
            .data
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes(p.try_into().unwrap()))
            .collect();
        if fields[..4] != [60, 384, 120, 8] || fields[4] < 4 || fields[4] % 2 != 0 {
            return Err("unsupported bank producer control fields".into());
        }
        Ok(Self {
            producer: BankProducer::from_dnni(model)?,
            head: Head8::from_dnni(model)?,
            reducer: SourcebankReducer::from_dnni(model)?,
            target4: Target4::from_dnni(model)?,
            bank_factor: usize::from(fields[4]),
        })
    }

    pub fn from_vec90_with_schedule(
        &self,
        rows72: &[f32],
        sample_count: usize,
    ) -> Result<ComputedGeneration> {
        self.try_from_vec90_with_schedule(rows72, sample_count)?
            .ok_or_else(|| "fewer than two accepted feature windows".into())
    }

    pub fn try_from_vec90_with_schedule(
        &self,
        rows72: &[f32],
        sample_count: usize,
    ) -> Result<Option<ComputedGeneration>> {
        use crate::source_pitch_join::normalized_energy;
        let schedule = scheduled_input(rows72, sample_count)?;
        let energies = rows72
            .chunks_exact(72)
            .map(normalized_energy)
            .collect::<Result<Vec<_>>>()?;
        let windows = schedule.accepted_windows(&energies, self.bank_factor)?;
        let completed = self.finish_scheduled(rows72, schedule, &windows, HostScheduledRetention)
            .map_err(|failure| failure.error)?;
        Ok(completed.map(|completed| ComputedGeneration { rows: completed.rows,
            interpolation: completed.interpolation, schedule, windows }))
    }

    pub fn try_from_vec90_with_source_window_math(&self, request: SourceScheduledGenerationRequest)
        -> std::result::Result<SourceScheduledGenerationCompleted, SourceScheduledGenerationFailure>
    {
        let admission = source_scheduled_input(&request);
        let schedule = match admission {
            Ok(schedule) => schedule,
            Err(error) => return Err(SourceScheduledGenerationFailure {
                owner: SourceScheduledFailureOwner::Before { request, error } }),
        };
        let SourceScheduledGenerationRequest { rows72, sample_count: _, energies, math } = request;
        let selection = match schedule.accepted_windows_with_source_math(energies, self.bank_factor, math) {
            Ok(completed) => completed.into_parts(),
            Err(failure) => return Err(SourceScheduledGenerationFailure {
                owner: SourceScheduledFailureOwner::Selection { rows72, failure } }),
        };
        let completed = self.finish_scheduled(&rows72, selection.schedule, &selection.windows,
            SourceScheduledRetention { global: None, window: None });
        match completed {
            Ok(None) => Ok(SourceScheduledGenerationCompleted {
                parts: SourceScheduledGenerationParts::Insufficient { rows72, selection } }),
            Ok(Some(completed)) => {
                let SourceGenerationWindowsParts { energies, math, windows, schedule, bank_factor } = selection;
                Ok(SourceScheduledGenerationCompleted { parts: SourceScheduledGenerationParts::Generated {
                    rows72, generation: ComputedGeneration { rows: completed.rows,
                        interpolation: completed.interpolation, schedule, windows },
                    context: SourceScheduledWindowContext { energies, math, bank_factor } } })
            }
            Err(failure) => Err(SourceScheduledGenerationFailure { owner: SourceScheduledFailureOwner::Later {
                rows72, selection, progress: SourceScheduledGenerationProgress { prefix: failure.prefix },
                error: failure.error } }),
        }
    }

    pub fn try_from_vec90_with_interleaved_source_window_math(&self, request: SourceScheduledGenerationRequest)
        -> std::result::Result<SourceInterleavedGenerationCompleted, SourceInterleavedGenerationFailure>
    {
        match self.interleaved_with_decimation(request, HostScheduledDecimation) {
            Ok(InterleavedCompleted::Insufficient { rows72, selection, prepared, decimation: _, resample: _ }) =>
                Ok(SourceInterleavedGenerationCompleted { parts: SourceInterleavedGenerationParts::Insufficient {
                    rows72, selection, prepared } }),
            Ok(InterleavedCompleted::Generated { rows72, generation, window, done: (), resampled: () }) =>
                Ok(SourceInterleavedGenerationCompleted { parts: SourceInterleavedGenerationParts::Generated {
                    rows72, generation, context: window } }),
            Err(owner) => Err(SourceInterleavedGenerationFailure { owner }),
        }
    }

    pub fn try_from_vec90_with_interleaved_source_window_and_decimation_math(&self,
        request: SourceInterleavedDecimationGenerationRequest)
        -> std::result::Result<SourceInterleavedDecimationGenerationCompleted, SourceInterleavedDecimationGenerationFailure>
    {
        let SourceInterleavedDecimationGenerationRequest { generation, decimation_math } = request;
        match self.interleaved_with_decimation(generation, SourceScheduledDecimation { math: decimation_math }) {
            Ok(InterleavedCompleted::Insufficient { rows72, selection, prepared, decimation, resample: _ }) =>
                Ok(SourceInterleavedDecimationGenerationCompleted {
                    parts: SourceInterleavedDecimationGenerationParts::Insufficient {
                        rows72, selection, prepared, decimation_math: decimation.math } }),
            Ok(InterleavedCompleted::Generated { rows72, generation, window, done, resampled: () }) =>
                Ok(SourceInterleavedDecimationGenerationCompleted {
                    parts: SourceInterleavedDecimationGenerationParts::Generated { rows72, generation,
                        context: SourceInterleavedDecimationGenerationContext { window,
                            decimation_math: done.math, decimation_progress: done.progress } } }),
            Err(owner) => Err(SourceInterleavedDecimationGenerationFailure { owner }),
        }
    }

    pub fn try_from_analysis_with_source_window_and_decimation_math<U: ScalarUpper>(&self,
        request: SourceAnalysisGenerationRequest<U>)
        -> std::result::Result<SourceAnalysisGenerationCompleted<U>, SourceAnalysisGenerationFailure<U>>
    {
        let SourceAnalysisGenerationRequest { analysis, sample_count, window_math, decimation_math } = request;
        let (rows, spectrum_math, postlog_math, join_math) = analysis.into_parts();
        let AnalysisRows { network72, joined, source_rows, pitch_rows, pending } = rows;
        let JoinedRows { paired72, normalized72, records, energies } = joined;
        let context = SourceAnalysisGenerationContext { network72, paired72, records,
            source_rows, pitch_rows, pending, spectrum_math, postlog_math, join_math };
        let request = SourceInterleavedDecimationGenerationRequest {
            generation: SourceScheduledGenerationRequest { rows72: normalized72, sample_count,
                energies, math: window_math }, decimation_math,
        };
        match self.try_from_vec90_with_interleaved_source_window_and_decimation_math(request) {
            Ok(generation) => Ok(SourceAnalysisGenerationCompleted { generation, context }),
            Err(generation_failure) => Err(SourceAnalysisGenerationFailure { generation_failure, context }),
        }
    }

    pub fn try_from_analysis_with_source_window_decimation_and_remap_math<U: ScalarUpper>(&self,
        request: SourceAnalysisRemapGenerationRequest<U>)
        -> std::result::Result<SourceAnalysisRemapGenerationCompleted<U>, SourceAnalysisRemapGenerationFailure<U>>
    {
        let SourceAnalysisRemapGenerationRequest { generation, remap_math } = request;
        let SourceAnalysisGenerationRequest { analysis, sample_count, window_math, decimation_math } = generation;
        let (rows, spectrum_math, postlog_math, join_math) = analysis.into_parts();
        let AnalysisRows { network72, joined, source_rows, pitch_rows, pending } = rows;
        let JoinedRows { paired72, normalized72, records, energies } = joined;
        let context = SourceAnalysisGenerationContext { network72, paired72, records,
            source_rows, pitch_rows, pending, spectrum_math, postlog_math, join_math };
        let request = SourceInterleavedRemapGenerationRequest {
            generation: SourceInterleavedDecimationGenerationRequest {
                generation: SourceScheduledGenerationRequest { rows72: normalized72, sample_count,
                    energies, math: window_math }, decimation_math,
            }, remap_math,
        };
        match self.try_from_vec90_with_interleaved_source_window_decimation_and_remap_math(request) {
            Ok(generation) => Ok(SourceAnalysisRemapGenerationCompleted { generation, context }),
            Err(generation_failure) => Err(SourceAnalysisRemapGenerationFailure { generation_failure, context }),
        }
    }

    pub fn try_from_vec90_with_interleaved_source_window_decimation_and_remap_math(&self,
        request: SourceInterleavedRemapGenerationRequest)
        -> std::result::Result<SourceInterleavedRemapGenerationCompleted, SourceInterleavedRemapGenerationFailure>
    {
        let SourceInterleavedRemapGenerationRequest { generation, remap_math } = request;
        let SourceInterleavedDecimationGenerationRequest { generation, decimation_math } = generation;
        match self.interleaved_with_finalizers(generation, SourceScheduledDecimation { math: decimation_math },
            SourceScheduledResample { math: remap_math })
        {
            Ok(InterleavedCompleted::Insufficient { rows72, selection, prepared, decimation, resample }) =>
                Ok(SourceInterleavedRemapGenerationCompleted { parts: SourceInterleavedRemapGenerationParts::Insufficient {
                    rows72, selection, prepared, decimation_math: decimation.math, remap_math: resample.math } }),
            Ok(InterleavedCompleted::Generated { rows72, generation, window, done, resampled }) =>
                Ok(SourceInterleavedRemapGenerationCompleted { parts: SourceInterleavedRemapGenerationParts::Generated {
                    rows72, generation, context: SourceInterleavedRemapGenerationContext { window,
                        decimation_math: done.math, decimation_progress: done.progress, resample: resampled } } }),
            Err(owner) => Err(SourceInterleavedRemapGenerationFailure { owner }),
        }
    }

    fn interleaved_with_decimation<C: ScheduledDecimationChoice>(&self,
        request: SourceScheduledGenerationRequest, decimation: C)
        -> std::result::Result<InterleavedCompleted<C, C::Done>, SourceInterleavedFailureOwner<C, C::Done, C::Stop>>
    {
        self.interleaved_with_finalizers(request, decimation, HostScheduledResample)
    }

    fn interleaved_with_finalizers<C: ScheduledDecimationChoice, R: ScheduledResampleChoice>(&self,
        request: SourceScheduledGenerationRequest, decimation: C, resample: R)
        -> std::result::Result<InterleavedCompleted<C, C::Done, R, R::Done>,
            SourceInterleavedFailureOwner<C, C::Done, C::Stop, R, R::Stop>>
    {
        let schedule = match source_scheduled_input(&request) {
            Ok(schedule) => schedule,
            Err(error) => return Err(SourceInterleavedFailureOwner::Before { request, decimation, resample, error }),
        };
        let mut prepared = SourceInterleavedPrepared {
            progress: SourceScheduledGenerationProgress { prefix: scheduled_prefix(SourceScheduledRetention {
                global: None, window: None }) },
            stage: SourceInterleavedGenerationStage::Prelude, copied128: None, target_input_range: None,
        };
        if let Err(error) = self.initialize_scheduled(&request.rows72, &mut prepared.progress.prefix, TargetTiming::Deferred) {
            return Err(SourceInterleavedFailureOwner::Prelude { request, prepared, decimation, resample, error });
        }
        let SourceScheduledGenerationRequest { rows72, sample_count: _, energies, math } = request;
        let consumed = consume_source_windows(schedule, energies, self.bank_factor, math,
            InterleavedOwner { rows72, prepared, decimation, resample },
            Self::prepare_interleaved_window,
            |owner, _, visit| self.accept_interleaved_window(owner, visit));
        let consumed = match consumed {
            Ok(consumed) => consumed,
            Err(failure) => return Err(SourceInterleavedFailureOwner::Interrupted(failure)),
        };
        let selection = consumed.selection;
        let InterleavedOwner { rows72, mut prepared, decimation, resample } = consumed.owner;
        if selection.windows.len() < 2 {
            return Ok(InterleavedCompleted::Insufficient { rows72, selection, prepared, decimation, resample });
        }
        if let Err(cause) = self.finish_interleaved_targets(&mut prepared) {
            return Err(SourceInterleavedFailureOwner::PostSelection { rows72, selection, prepared, decimation, resample, cause });
        }
        let (completed, done, resampled) = match self.finalize_scheduled_with_choices(
            &mut prepared.progress.prefix, selection.schedule, decimation, resample)
        {
            Ok(completed) => completed,
            Err(failure) => return Err(SourceInterleavedFailureOwner::Finalize { rows72, selection, prepared, failure }),
        };
        let SourceGenerationWindowsParts { energies, math, windows, schedule, bank_factor } = selection;
        Ok(InterleavedCompleted::Generated {
            rows72, generation: ComputedGeneration { rows: completed.rows,
                interpolation: completed.interpolation, schedule, windows },
            window: SourceScheduledWindowContext { energies, math, bank_factor }, done, resampled })
    }

    fn prepare_interleaved_window<C, R>(mut owner: InterleavedOwner<C, R>, _: &[f32], _: SourceWindowVisit)
        -> std::result::Result<InterleavedOwner<C, R>, ConsumerStop<InterleavedOwner<C, R>, InterleavedConsumerCause>>
    {
        owner.prepared.stage = SourceInterleavedGenerationStage::PrepareWindow;
        if let Err(error) = owner.prepared.progress.prefix.initial128.as_mut().unwrap().try_reserve(128) {
            return Err(ConsumerStop { owner, error: InterleavedConsumerCause::Storage(error) });
        }
        Ok(owner)
    }

    fn accept_interleaved_window<C, R>(&self, mut owner: InterleavedOwner<C, R>, visit: SourceWindowVisit)
        -> std::result::Result<InterleavedOwner<C, R>, ConsumerStop<InterleavedOwner<C, R>, InterleavedConsumerCause>>
    {
        owner.prepared.stage = SourceInterleavedGenerationStage::Assemble128;
        let prefix = &mut owner.prepared.progress.prefix;
        if let Err(error) = self.assemble_scheduled_window(prefix, &visit.current.window, visit.selected_ordinal) {
            return Err(ConsumerStop { owner, error: InterleavedConsumerCause::Model(error) });
        }
        prefix.initial128.as_mut().unwrap().extend(prefix.current_row.take().unwrap());
        Ok(owner)
    }

    fn finish_interleaved_targets(&self, prepared: &mut SourceInterleavedPrepared)
        -> std::result::Result<(), InterleavedConsumerCause>
    {
        prepared.stage = SourceInterleavedGenerationStage::CopyStorage;
        prepared.copied128 = Some(Vec::new());
        let original = prepared.progress.prefix.initial128.as_ref().unwrap();
        let copied = prepared.copied128.as_mut().unwrap();
        copied.try_reserve_exact(original.len()).map_err(InterleavedConsumerCause::Storage)?;
        prepared.stage = SourceInterleavedGenerationStage::Copy128;
        copied.extend_from_slice(original);
        prepared.stage = SourceInterleavedGenerationStage::TargetStorage;
        prepared.progress.prefix.initial4 = Some(Vec::new());
        let rows = copied.len() / 128;
        prepared.progress.prefix.initial4.as_mut().unwrap().try_reserve_exact(rows * 4)
            .map_err(InterleavedConsumerCause::Storage)?;
        for ordinal in 0..rows {
            prepared.stage = SourceInterleavedGenerationStage::Target4;
            let range = (ordinal * 128, (ordinal + 1) * 128);
            prepared.target_input_range = Some(range);
            prepared.progress.prefix.stage = SourceScheduledGenerationStage::Target4;
            prepared.progress.prefix.window_ordinal = Some(ordinal);
            let target = self.target_scheduled_row(&prepared.copied128.as_ref().unwrap()[range.0..range.1])
                .map_err(InterleavedConsumerCause::Model)?;
            prepared.progress.prefix.initial4.as_mut().unwrap().extend(target);
            prepared.target_input_range = None;
        }
        prepared.stage = SourceInterleavedGenerationStage::Finalize;
        Ok(())
    }

    fn finish_scheduled<R: ScheduledRetention>(&self, rows72: &[f32], schedule: GenerationSchedule,
        windows: &[AcceptedWindow], retained: R)
        -> std::result::Result<Option<ScheduledRows>, ScheduledSuffixFailure<R::Head, R>>
    {
        if windows.len() < 2 { return Ok(None); }
        let mut prefix = scheduled_prefix(retained);
        let result = (|| {
            self.initialize_scheduled(rows72, &mut prefix, TargetTiming::Immediate)?;
            for (ordinal, window) in windows.iter().enumerate() {
                self.assemble_scheduled_window(&mut prefix, window, ordinal)?;
                prefix.stage = SourceScheduledGenerationStage::Target4;
                let target = self.target_scheduled_row(prefix.current_row.as_ref().unwrap())?;
                prefix.initial4.as_mut().unwrap().extend(target);
                prefix.initial128.as_mut().unwrap().extend(prefix.current_row.take().unwrap());
            }
            self.finalize_scheduled(&mut prefix, schedule)
        })();
        match result {
            Ok(completed) => Ok(Some(completed)),
            Err(error) => Err(ScheduledSuffixFailure { prefix, error }),
        }
    }

    fn initialize_scheduled<R: ScheduledRetention>(&self, rows72: &[f32],
        prefix: &mut ScheduledPrefix<R::Head, R>, timing: TargetTiming) -> Result<()>
    {
        let banks = self.producer.forward_vec90(rows72)?;
        prefix.banks = Some(banks);
        prefix.stage = SourceScheduledGenerationStage::Head;
        let head = self.head.forward_vec90(rows72)?;
        prefix.head = Some(R::head(head));
        prefix.trajectory128 = Some(R::head_output(prefix.head.as_ref().unwrap()).clone());
        prefix.stage = SourceScheduledGenerationStage::GlobalReducer;
        let banks = prefix.banks.as_ref().unwrap();
        let global = self.reducer.forward(&banks.bank8, &banks.bank384)?;
        prefix.trajectory128.as_mut().unwrap().extend(prefix.retained.global(global));
        prefix.initial128 = Some(Vec::new());
        if matches!(timing, TargetTiming::Immediate) { prefix.initial4 = Some(Vec::new()); }
        Ok(())
    }

    fn assemble_scheduled_window<R: ScheduledRetention>(&self, prefix: &mut ScheduledPrefix<R::Head, R>,
        window: &AcceptedWindow, ordinal: usize) -> Result<()>
    {
        prefix.stage = SourceScheduledGenerationStage::WindowExtent;
        prefix.window_ordinal = Some(ordinal);
        let banks = prefix.banks.as_ref().unwrap();
        let (rows8, rows384) = if window.bank_start < window.bank_end {
            if window.bank_end > banks.bank8.len() / 8 {
                return Err("accepted generation window has outside-bank extent".into());
            }
            (&banks.bank8[window.bank_start * 8..window.bank_end * 8],
                &banks.bank384[window.bank_start * 384..window.bank_end * 384])
        } else { (&banks.bank8[..0], &banks.bank384[..0]) };
        prefix.stage = SourceScheduledGenerationStage::WindowReducer;
        let reduction = self.reducer.forward(rows8, rows384)?;
        let tail = prefix.retained.window(ordinal, reduction);
        let mut row = R::head_output(prefix.head.as_ref().unwrap()).clone();
        row.extend(tail);
        prefix.current_row = Some(row);
        Ok(())
    }

    fn target_scheduled_row(&self, row: &[f32]) -> Result<Vec<f32>> { self.target4.forward(row) }

    fn finalize_scheduled<R: ScheduledRetention>(&self, prefix: &mut ScheduledPrefix<R::Head, R>,
        schedule: GenerationSchedule) -> Result<ScheduledRows>
    {
        match self.finalize_scheduled_with_decimation(prefix, schedule, HostScheduledDecimation) {
            Ok((rows, ())) => Ok(rows),
            Err(FinalizeStop::Decimation { failure: error, .. } | FinalizeStop::Resample { failure: error, .. }) => Err(error),
        }
    }

    fn finalize_scheduled_with_decimation<R: ScheduledRetention, C: ScheduledDecimationChoice>(&self,
        prefix: &mut ScheduledPrefix<R::Head, R>, schedule: GenerationSchedule, decimation: C)
        -> std::result::Result<(ScheduledRows, C::Done), FinalizeStop<C::Done, C::Stop>>
    {
        self.finalize_scheduled_with_choices(prefix, schedule, decimation, HostScheduledResample)
            .map(|(rows, done, ())| (rows, done))
    }

    fn finalize_scheduled_with_choices<T: ScheduledRetention, C: ScheduledDecimationChoice, R: ScheduledResampleChoice>(&self,
        prefix: &mut ScheduledPrefix<T::Head, T>, schedule: GenerationSchedule, decimation: C, resample: R)
        -> std::result::Result<(ScheduledRows, C::Done, R::Done), FinalizeStop<C::Done, C::Stop, R, R::Stop>>
    {
        prefix.window_ordinal = None;
        prefix.stage = SourceScheduledGenerationStage::Decimate;
        let initial4 = prefix.initial4.as_ref().unwrap();
        let retained_count = (initial4.len() / 4).min(schedule.requested_points);
        let done = match decimation.run(prefix, retained_count) {
            Ok(done) => done,
            Err(failure) => return Err(FinalizeStop::Decimation { failure, resample }),
        };
        prefix.stage = SourceScheduledGenerationStage::Resample;
        let ScheduledResamplePrepared { output: final_rows, done: resampled } = match resample.run(prefix) {
            Ok(prepared) => prepared,
            Err(failure) => return Err(FinalizeStop::Resample { done, failure }),
        };
        prefix.stage = SourceScheduledGenerationStage::Complete;
        let banks = prefix.banks.take().unwrap();
        let post = prefix.post.take().unwrap();
        Ok((ScheduledRows { interpolation: final_rows.table, rows: GeneratedRows {
            bank8: banks.bank8, bank384: banks.bank384,
            trajectory128: prefix.trajectory128.take().unwrap(),
            initial128: prefix.initial128.take().unwrap(), initial4: prefix.initial4.take().unwrap(),
            postdecim128: post.rows128, postdecim4: post.rows4,
            points128: final_rows.points128, points4: final_rows.points4,
            removed_indices: post.removed_indices } }, done, resampled))
    }

    pub fn from_vec90_with_control(
        &self,
        rows72: &[f32],
        target_count: usize,
        table: &[f32],
    ) -> Result<GeneratedRows> {
        if table.len() != target_count {
            return Err("point count/control table length mismatch".into());
        }
        let banks = self.producer.forward_vec90(rows72)?;
        let head = self.head.forward_vec90(rows72)?.out8;
        let global = self.reducer.forward(&banks.bank8, &banks.bank384)?;
        let mut trajectory128 = head.clone();
        trajectory128.extend(global.tail120);
        let windows = current_windows(banks.bank8.len() / 8);
        let mut initial128 = Vec::new();
        let mut initial4 = Vec::new();
        for (start, count) in windows {
            let tail = self
                .reducer
                .forward(
                    &banks.bank8[start * 8..(start + count) * 8],
                    &banks.bank384[start * 384..(start + count) * 384],
                )?
                .tail120;
            let mut row = head.clone();
            row.extend(tail);
            initial4.extend(self.target4.forward(&row)?);
            initial128.extend(row);
        }
        let post = greedy_decimate(&initial128, &initial4, target_count)?;
        let points128 = interpolate_rows(&post.rows128, 128, table)?;
        let points4 = interpolate_rows(&post.rows4, 4, table)?;
        Ok(GeneratedRows {
            bank8: banks.bank8,
            bank384: banks.bank384,
            trajectory128,
            initial128,
            initial4,
            postdecim128: post.rows128,
            postdecim4: post.rows4,
            points128,
            points4,
            removed_indices: post.removed_indices,
        })
    }
}
