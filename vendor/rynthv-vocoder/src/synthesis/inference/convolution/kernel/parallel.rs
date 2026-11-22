use crate::synthesis::inference::convolution::ConvolutionError;
pub(super) fn for_frames(output:&mut [f32], _channels:usize, _macs_per_frame:usize,
    execute:impl Fn(usize,&mut [f32])->Result<(),ConvolutionError>+Send+Sync)->Result<(),ConvolutionError> {
    execute(0, output)
}
