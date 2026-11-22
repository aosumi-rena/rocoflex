use super::MatrixError;
pub(super) fn for_frames(output:&mut [f32], _channels:usize,
    execute:impl Fn(usize,&mut [f32])->Result<(),MatrixError>+Send+Sync)->Result<(),MatrixError> {
    execute(0, output)
}
