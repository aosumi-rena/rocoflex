use crate::{Result,model::ModelTree,normalization::MaxMinNormalizer,root_binding::VocoderConfigV2};

pub(crate) use crate::prepare_quality::SynthesisFamily;

pub(crate) struct FeatureTransform {normalizer:MaxMinNormalizer}
impl FeatureTransform {
    pub(crate) fn from_userdata(bytes:&[u8])->Result<Self> {
        Self::from_userdata_for_family(bytes,SynthesisFamily::Primary)
    }

    pub(crate) fn from_userdata_for_family(bytes:&[u8],family:SynthesisFamily)->Result<Self> {
        let tree=ModelTree::parse(bytes)?;
        if tree.version!=2 {return Err("Synthesis feature preprocessing requires DNNIv2".into());}
        let index=match family {SynthesisFamily::Primary=>0,SynthesisFamily::Alternate=>1};
        let selected=tree.root.children.get(index)
            .ok_or("Missing synthesis vocoder feature descriptor")?;

        if family==SynthesisFamily::Alternate && (selected.kind!=0x7fca41ff
            || selected.tag!=0xc3b2e0d90e2fdb1b || selected.data.len()!=16 || selected.children.len()!=12) {
            return Err("Alternate synthesis family topology differs".into());
        }
        let vocoder=selected.children.first().ok_or("Missing synthesis vocoder feature descriptor")?;
        if vocoder.data.len()!=52 || vocoder.children.len()!=4 {
            return Err("Synthesis vocoder feature topology differs".into());
        }
        let tag=match family {
            SynthesisFamily::Primary=>0xeb8a194b072d26c9,
            SynthesisFamily::Alternate=>0x5531d27be3b42ef3,
        };
        vocoder.require(0x7fca40ff,b"_vocfv2",&[tag^0x40dab9846f846a19])?;
        let config=VocoderConfigV2::parse(vocoder.data).map_err(|e|format!("Vocoder feature configuration: {e:?}"))?;
        if config.sample_rate!=48_000 || config.hop_seconds_bits!=0x3ba3_d70a {
            return Err("Connected synthesis requires the selected 48kHz/5ms feature configuration".into());
        }
        if config.field_80!=72 || config.field_84!=1 || config.fields_104_to_116[3]!=1 || config.native_field_88()!=0
            || config.field_100!=72 || config.flags_120_to_123[3] {
            return Err("Synthesis feature preprocessing requires actual width72/history1/mode0/flag123false".into());
        }
        Ok(Self {normalizer:MaxMinNormalizer::from_node(&vocoder.children[0],72)?})
    }
    pub(crate) fn transform(&self,decoded72:&[f32])->Result<[f32;72]> {
        if decoded72.len()!=72 || decoded72.iter().any(|x|!x.is_finite()) || decoded72[1]<=0. {
            return Err("Synthesis preprocessing requires finite decoder72 with positive pitchHz".into());
        }
        let mut scratch:[f32;72]=decoded72.try_into().unwrap();

        scratch[1]=scratch[1].ln();
        self.normalizer.forward(&scratch,true)?.try_into()
            .map_err(|_|"Synthesis preprocessing output width differs".into())
    }
}

