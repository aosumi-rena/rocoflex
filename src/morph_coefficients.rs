use std::collections::VecDeque;
use crate::{Result, prototype_synthesis_networks::{ConditioningNetwork, NcwnNetwork, NcwnCausalStream, NcwnPersistentStream, CepstralBands, PointwiseCepstralBands, HarmonicProjection}};
use super::decoder::alternate_inner::AlternateInnerPair;

enum LearnedFilters {
    Cepstral(PointwiseCepstralBands),
    Direct(AlternateInnerPair),
}

pub(crate) struct CoefficientFrame {
    pub ordinal: u64,
    pub condition: [f32;72],
    pub first: Vec<f32>,
    pub second: Vec<f32>,
    pub projection10: [f32;10],
}

pub(crate) struct FeatureSynthesis {
    conditioning: ConditioningNetwork,
    first: NcwnPersistentStream,
    second: NcwnCausalStream,
    filters: LearnedFilters,
    projection: HarmonicProjection,
    first_conditions: VecDeque<Vec<f32>>,
    features: VecDeque<(u64,[f32;72])>,
    ordinal: u64,
    failure: Option<String>,
}
impl FeatureSynthesis {
    pub(crate) fn new(conditioning: ConditioningNetwork, first: NcwnNetwork,
        second: NcwnNetwork, cepstral: CepstralBands, projection: HarmonicProjection) -> Result<Self>
    {
        if (first.input_width,first.condition_width,first.output_width)!=(72,192,256)
            || (second.input_width,second.condition_width,second.output_width)!=(256,192,256)
            || cepstral.input_width!=256 || cepstral.output_width!=512 {
            return Err("Synthesis feature graph dimensions differ from the loaded model".into());
        }

        let cepstral=cepstral.into_pointwise()?;
        Self::with_filters(conditioning,first,second,LearnedFilters::Cepstral(cepstral),projection)
    }

    pub(crate) fn new_alternate(conditioning:ConditioningNetwork,first:NcwnNetwork,
        second:NcwnNetwork,filters:AlternateInnerPair,projection:HarmonicProjection)->Result<Self> {
        if !filters.is_256_384_512() {
            return Err("Alternate filter networks must map 256 to 384 to 512".into());
        }
        Self::with_filters(conditioning,first,second,LearnedFilters::Direct(filters),projection)
    }

    fn with_filters(conditioning:ConditioningNetwork,first:NcwnNetwork,second:NcwnNetwork,
        filters:LearnedFilters,projection:HarmonicProjection)->Result<Self> {
        if (first.input_width,first.condition_width,first.output_width)!=(72,192,256)
            || (second.input_width,second.condition_width,second.output_width)!=(256,192,256) {
            return Err("Synthesis feature network dimensions differ from the loaded model".into());
        }

        #[cfg(target_arch="x86_64")]
        let fused_multiply_add=std::is_x86_feature_detected!("fma");
        #[cfg(not(target_arch="x86_64"))]
        let fused_multiply_add=false;
        let first=first.with_activation_arithmetic(crate::activation::ArithmeticMode {fused_multiply_add});
        let second=second.with_activation_arithmetic(crate::activation::ArithmeticMode {fused_multiply_add});
        Ok(Self { conditioning, first:first.into_persistent_stream()?, second:second.into_causal_stream()?,
            filters, projection, first_conditions:VecDeque::new(), features:VecDeque::new(),
            ordinal:0, failure:None })
    }

    pub(crate) fn prepared_right_rows(&self)->super::count_plan::Query<u64> {
        super::count_plan::cold(self.ordinal!=0 || !self.first_conditions.is_empty() || !self.features.is_empty(),
            self.failure.is_some())?;
        super::count_plan::checked_rows(self.first.lookahead_rows())
    }
    pub(crate) fn process(&mut self, feature:&[f32], output:&mut Vec<CoefficientFrame>) -> Result<()> {
        if let Some(error)=&self.failure { return Err(error.clone()); }
        let result=self.advance(feature,output);
        if let Err(error)=&result { self.failure=Some(error.clone()); }
        result
    }

    fn advance(&mut self, feature:&[f32], output:&mut Vec<CoefficientFrame>) -> Result<()> {
        if feature.len()!=72 || feature.iter().any(|x|!x.is_finite()) {
            return Err("Synthesis requires an actual finite 72-value decoder feature".into());
        }
        let condition=self.conditioning.condition(feature,self.ordinal as u32)?;
        let ordinal=self.ordinal;
        self.ordinal=self.ordinal.checked_add(1).ok_or("Synthesis feature ordinal overflow")?;
        let feature:[f32;72]=feature.try_into().map_err(|_|"Decoder feature width")?;
        self.features.push_back((ordinal,feature));
        self.first_conditions.push_back(condition.clone());
        let mut first=Vec::new();
        self.first.push(&[feature.to_vec()],&[condition],false,&mut first)?;
        if first.is_empty() { return Ok(()); }
        if first.len()>self.first_conditions.len() {return Err("First network condition alignment was lost".into());}
        let conditions:Vec<_>=self.first_conditions.drain(..first.len()).collect();
        let mut second=Vec::new();
        self.second.process(&first,&conditions,&mut second)?;
        if second.is_empty() {return Ok(());}
        let (before,after)=match &mut self.filters {
            LearnedFilters::Cepstral(cepstral)=>{
                let [before,after]=cepstral.forward(&second)?;
                (before,after)
            }
            LearnedFilters::Direct(filters)=>{
                let mut before=Vec::with_capacity(second.len());
                let mut after=Vec::with_capacity(second.len());
                for row in &second {
                    let (a,b)=filters.forward_row(row)?;
                    before.push(a);after.push(b);
                }
                (before,after)
            }
        };
        if before.len()!=second.len() || after.len()!=second.len() || before.len()>self.features.len() {
            return Err("Cepstral output frame alignment differs from the decoder features".into());
        }
        output.try_reserve(before.len()).map_err(|e|e.to_string())?;
        for ((first,second),learned) in before.into_iter().zip(after).zip(&second) {
            if first.len()!=512 || second.len()!=512 || first.iter().chain(&second).any(|x|!x.is_finite()) {
                return Err("Learned filter graph produced invalid cepstral coefficients".into());
            }
            let (ordinal,condition)=self.features.pop_front().ok_or("Missing coefficient feature condition")?;
            let projection10=self.projection.forward(learned)?;
            output.push(CoefficientFrame {ordinal,condition,first,second,projection10});
        }
        Ok(())
    }
}
