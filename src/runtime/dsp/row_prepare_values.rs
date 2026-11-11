pub trait Services {
    type Error;
    fn current_count(&self) -> usize;
    fn retire_current_back(&mut self) -> Result<(), Self::Error>;
    fn row_matches(&self, index: usize, width: usize) -> bool;

    fn create_admit_install(&mut self, index: usize, width: usize) -> Result<(), Self::Error>;
}

pub fn prepare<S: Services>(services: &mut S, count: usize, width: usize) -> Result<(), S::Error> {
    while services.current_count() > count {
        services.retire_current_back()?;
    }
    for index in 0..count {
        if !services.row_matches(index, width) {
            services.create_admit_install(index, width)?;
        }
    }
    Ok(())
}
