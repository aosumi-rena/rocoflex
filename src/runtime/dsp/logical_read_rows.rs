pub trait ReadRows {
    fn row_count(&self)->usize;
    fn source_status(&self)->u8;
    fn read_row(&self,index:usize)->Option<&[f32]>;
}
