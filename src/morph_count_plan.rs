pub(crate) const COUNT_HOP:u64=240;
pub(crate) const COUNT_POLICY_VERSION:u32=1;
pub(crate) const ALTERNATE_COUNT_POLICY_VERSION:u32=2;
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) enum Unsupported {
    Head2, AdvancedCursor, FailedCursor,
    InnerVocoderWorker, UnknownTopology,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) enum Invalid { Overflow, MissingCursor, StructuralMetadata, InputDomain }
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) enum Unavailable { Unsupported(Unsupported), Invalid(Invalid) }
pub(crate) type Query<T>=std::result::Result<T,Unavailable>;
fn overflow()->Unavailable {Unavailable::Invalid(Invalid::Overflow)}
pub(crate) fn checked_rows(rows:usize)->Query<u64> {
    u64::try_from(rows).map_err(|_|overflow())
}
pub(crate) fn cold(advanced:bool,failed:bool)->Query<()> {
    if failed {Err(Unavailable::Unsupported(Unsupported::FailedCursor))}
    else if advanced {Err(Unavailable::Unsupported(Unsupported::AdvancedCursor))}
    else {Ok(())}
}
pub(crate) fn head0(is_head0:bool)->Query<()> {
    if is_head0 {Ok(())} else {Err(Unavailable::Unsupported(Unsupported::Head2))}
}
pub(crate) fn inner_serial(is_worker:bool)->Query<()> {
    if is_worker {Err(Unavailable::Unsupported(Unsupported::InnerVocoderWorker))} else {Ok(())}
}
pub(crate) fn publication_policy(version:u32,initial_state_matches:bool)->Query<()> {
    if version!=COUNT_POLICY_VERSION {Err(Unavailable::Unsupported(Unsupported::UnknownTopology))}
    else if !initial_state_matches {Err(Unavailable::Invalid(Invalid::StructuralMetadata))}
    else {Ok(())}
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) struct DecoderCounts {pub front_right_rows:u64,pub tail_right_rows:u64}
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) struct EngineCounts {
    pub decoder:DecoderCounts,pub coefficient_right_rows:u64,pub policy_version:u32,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) enum HeadKind {Head0,Head2}
#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) struct HeadCounts {kind:HeadKind,right_rows:u64}
pub(crate) fn head_counts(kind:HeadKind,right_rows:u64)->Query<HeadCounts> {

    if !matches!((kind,right_rows),(HeadKind::Head0,0)|(HeadKind::Head2,2)) {
        return Err(Unavailable::Invalid(Invalid::StructuralMetadata));
    }
    Ok(HeadCounts {kind,right_rows})
}

#[derive(Clone,Copy,Debug,PartialEq,Eq,serde::Serialize)]
pub(crate) struct PreparedCountPlan {
    policy_version:u32,head:HeadCounts,decoder:DecoderCounts,coefficient_right_rows:u64,
    delay_hops:u64,all_phase_deficit_bound:u64,whole_hop_deficit_bound:u64,
}
impl PreparedCountPlan {
    pub(crate) fn from_prepared(counts:EngineCounts)->Query<Self> {
        Self::from_prepared_head(head_counts(HeadKind::Head0,0)?,counts)
    }
    pub(crate) fn from_prepared_head(head:HeadCounts,counts:EngineCounts)->Query<Self> {
        let decoder_rows=counts.decoder.front_right_rows.checked_add(counts.decoder.tail_right_rows).ok_or_else(overflow)?;
        let branch_rows=match counts.policy_version {
            COUNT_POLICY_VERSION=>counts.coefficient_right_rows.checked_add(4).ok_or_else(overflow)?.max(5),
            ALTERNATE_COUNT_POLICY_VERSION=>counts.coefficient_right_rows.checked_add(1).ok_or_else(overflow)?.max(2),
            _=>return Err(Unavailable::Unsupported(Unsupported::UnknownTopology)),
        };
        let delay_hops=head.right_rows.checked_add(decoder_rows)
            .and_then(|v|v.checked_add(branch_rows)).ok_or_else(overflow)?;
        let delayed=COUNT_HOP.checked_mul(delay_hops).ok_or_else(overflow)?;
        let all_phase_deficit_bound=delayed.checked_add(COUNT_HOP-1).ok_or_else(overflow)?;
        let whole_hop_deficit_bound=delay_hops.checked_add(1).and_then(|h|h.checked_mul(COUNT_HOP)).ok_or_else(overflow)?;
        Ok(Self {policy_version:counts.policy_version,head,decoder:counts.decoder,
            coefficient_right_rows:counts.coefficient_right_rows,delay_hops,
            all_phase_deficit_bound,whole_hop_deficit_bound})
    }
    pub(crate) fn policy_version(self)->u32 {self.policy_version}
    pub(crate) fn head_kind(self)->HeadKind {self.head.kind}
    pub(crate) fn head_right_rows(self)->u64 {self.head.right_rows}
    pub(crate) fn delay_hops(self)->u64 {self.delay_hops}
    pub(crate) fn all_phase_deficit_bound(self)->u64 {self.all_phase_deficit_bound}
    pub(crate) fn whole_hop_deficit_bound(self)->u64 {self.whole_hop_deficit_bound}

    pub(crate) fn predicted_output_samples(self,admitted_input:u64)->Query<u64> {
        if admitted_input>i32::MAX as u64 {return Err(Unavailable::Invalid(Invalid::InputDomain));}
        (admitted_input/COUNT_HOP).saturating_sub(self.delay_hops)
            .checked_mul(COUNT_HOP).ok_or_else(overflow)
    }
}
