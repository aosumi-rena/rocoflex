use crate::pitch_selected_head::PitchRecord;

const PRESENT: u64 = 1 << 33;
const LOG_HZ: u64 = 1 << 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Observation { pub word:u64, pub ordinal:u64 }

pub(crate) struct Source {
    tag:[u64;3], mixed_hop:bool, sample:Observation,
}
impl Default for Source {
    fn default()->Self {Self{tag:[1,0,3],mixed_hop:false,sample:Observation::default()}}
}
impl Source {
    pub(crate) fn change(&mut self,epoch:u64,channel:u32,bus:u64,buffered:bool) {
        let tag=[epoch,u64::from(channel),bus];
        if tag!=self.tag {
            self.tag=tag;self.sample=Observation::default();self.mixed_hop|=buffered;
        }
    }
    pub(crate) fn begin_hop(&mut self)->[u64;3] {
        if std::mem::take(&mut self.mixed_hop) {[0;3]} else {self.tag}
    }
    pub(crate) fn observe(&mut self,tag:[u64;3],ordinal:u64,record:PitchRecord) {
        self.sample=if tag==self.tag && tag[0]!=0 && tag[1]<=1 && tag[2]>>1!=0 && tag[2]&1!=0 {
            ordinal.checked_add(1).map_or_else(Observation::default,|ordinal|Observation{word:pack(record),ordinal})
        } else {Observation::default()};
    }
    pub(crate) fn sample(&self)->Observation {self.sample}
}

pub(crate) fn pack(record: PitchRecord) -> u64 {
    let bits=record.value.to_bits();
    if !record.voiced || bits & 0x7f80_0000 == 0x7f80_0000 || (!record.valid && (bits & 0x8000_0000 != 0 || bits & 0x7fff_ffff == 0)) {
        return 0;
    }
    PRESENT | if record.valid { LOG_HZ } else { 0 } | u64::from(bits)
}

pub(crate) fn valid(word: u64) -> bool {
    if word == 0 { return true; }
    if word >> 34 != 0 || word & PRESENT == 0 { return false; }
    let bits=word as u32;
    bits & 0x7f80_0000 != 0x7f80_0000 && (word & LOG_HZ != 0 || (bits & 0x8000_0000 == 0 && bits & 0x7fff_ffff != 0))
}

pub(crate) fn midi(word: u64) -> Option<f32> {
    if word == 0 || !valid(word) { return None; }
    let value = f64::from(f32::from_bits(word as u32));
    let log_hz = if word & LOG_HZ != 0 { value } else { value.ln() };
    let midi = (69.0 + (log_hz - 440.0_f64.ln()) * (12.0 / std::f64::consts::LN_2)) as f32;
    midi.is_finite().then_some(midi)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Statistics(pub [f32; 5]);
impl Statistics {
    pub(crate) fn from_source(source: &serde_json::Value) -> Option<Self> {
        let stats = source.get("pitchStats")?;
        let mut values = [0.0; 5];
        for (out, key) in values.iter_mut().zip(["q10", "q25", "q50", "q75", "q90"]) {
            *out = stats.get(key)?.as_f64()? as f32;
        }
        if values.iter().any(|v| !v.is_finite() || v.abs() > 256.0) || values.windows(2).any(|v| v[0] > v[1]) {
            return None;
        }
        Some(Self(values))
    }
    pub(crate) fn position(self, note: f32) -> Option<f32> {
        if !note.is_finite() { return None; }
        let low = self.0[0]; let high = self.0[4];
        let (low, high) = if high - low < 1.0 { (self.0[2] - 0.5, self.0[2] + 0.5) } else { (low, high) };
        Some(((note - low) / (high - low)).clamp(0.0, 1.0))
    }
}
