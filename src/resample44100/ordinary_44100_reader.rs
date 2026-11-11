use super::ordinary_44100_chain::{Ordinary44100Chain,Ordinary44100Math,OrdinaryMathNames};
use std::collections::VecDeque;

#[derive(Clone,Copy,Debug,Eq,PartialEq)]
pub struct ReaderState {
    pub requested_source_position:usize,
    pub pending_fifo_words:usize,
    pub rounded_f32_words:usize,
    pub source_read_calls:usize,
}

pub struct Mono44100Reader<'a> {
    source:&'a [f32],
    chain:Ordinary44100Chain,
    fifo:VecDeque<f32>,
    source_position:usize,
    rounded_words:usize,
    read_calls:usize,
    pub prepared_source_capacity:usize,
}

pub fn short_import_output_count(source_count:usize)->Result<usize,&'static str> {

    if source_count==0 || source_count>44100*300 {
        return Err("own nonempty decoded44100 duration<=300s scope required");
    }

    let duration=source_count as f64/44100.0;
    Ok((duration*48000.0).trunc() as usize)
}

impl<'a> Mono44100Reader<'a> {
    pub fn new<M:Ordinary44100Math>(source:&'a [f32],names:OrdinaryMathNames,math:&mut M)
        ->Result<Self,&'static str> {
        short_import_output_count(source.len())?;
        if source.iter().any(|x|!x.is_finite()) {
            return Err("finite decoded channel0 input required");
        }

        let prepared_bits=(512.0*(44100.0/48000.0)+6755399441055744.0f64).to_bits();
        let prepared_source_capacity=(prepared_bits as u32) as usize+64;
        let chain=Ordinary44100Chain::new(names,math)?;
        Ok(Self {source,chain,fifo:VecDeque::new(),source_position:0,
            rounded_words:0,read_calls:0,prepared_source_capacity})
    }

    pub fn state(&self)->ReaderState {
        ReaderState {requested_source_position:self.source_position,
            pending_fifo_words:self.fifo.len(),rounded_f32_words:self.rounded_words,
            source_read_calls:self.read_calls}
    }

    pub fn read_block<M:Ordinary44100Math>(&mut self,count:usize,math:&mut M)
        ->Result<Vec<f32>,&'static str> {
        if count>512 {return Err("own ordinary prepare512 output-block scope required");}
        while self.fifo.len()<count {
            let missing=count-self.fifo.len();
            let mut requested=missing as f64;
            requested*=44100.0/48000.0;
            requested+=64.0;
            let requested=requested.trunc() as usize;
            if requested==0 || requested>self.prepared_source_capacity {
                return Err("source prepared-capacity/progress invariant failed");
            }
            let begin=self.source_position.min(self.source.len());
            let available=requested.min(self.source.len()-begin);
            let mut widened=vec![0.0;requested];
            for (target,source) in widened[..available].iter_mut()
                .zip(&self.source[begin..begin+available]) {
                *target=*source as f64;
            }

            self.source_position+=requested;
            self.read_calls+=1;
            let produced=self.chain.push_f64_no_drain(&widened,math)?;
            self.rounded_words+=produced.len();

            self.fifo.extend(produced.into_iter().map(|x|x as f32));
        }
        Ok(self.fifo.drain(..count).collect())
    }
}

pub fn convert_short_channel0<M:Ordinary44100Math>(source:&[f32],names:OrdinaryMathNames,
    math:&mut M)->Result<Vec<f32>,&'static str> {
    let count=short_import_output_count(source.len())?;
    let mut reader=Mono44100Reader::new(source,names,math)?;
    let mut output=Vec::with_capacity(count);
    while output.len()<count {
        let requested=(count-output.len()).min(512);
        output.extend(reader.read_block(requested,math)?);
    }
    Ok(output)
}
