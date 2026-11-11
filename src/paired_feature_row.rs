use crate::paired_frames_pool::{CheckedOutFrame, CheckoutComplete, CheckoutOrigin};

pub struct FeatureRow { owner: FeatureRowOwner }

pub enum FeatureRowOwner { Explicit(Vec<f32>), CheckedOut(CheckedOutFrame) }
impl FeatureRow {
    pub fn from_explicit_values(row: Vec<f32>) -> Self { Self { owner: FeatureRowOwner::Explicit(row) } }
    pub fn from_checkout(frame: CheckedOutFrame) -> Self { Self { owner: FeatureRowOwner::CheckedOut(frame) } }
    pub fn samples(&self) -> &[f32] {
        match &self.owner {
            FeatureRowOwner::Explicit(row) => row,
            FeatureRowOwner::CheckedOut(frame) => frame.samples(),
        }
    }
    pub fn samples_mut(&mut self) -> &mut [f32] {
        match &mut self.owner {
            FeatureRowOwner::Explicit(row) => row,
            FeatureRowOwner::CheckedOut(frame) => frame.samples_mut(),
        }
    }
    pub fn checkout(&self) -> Option<&CheckoutComplete> {
        match &self.owner {
            FeatureRowOwner::Explicit(_) => None,
            FeatureRowOwner::CheckedOut(frame) => Some(frame.completion()),
        }
    }
    pub fn checkout_origin(&self) -> Option<&CheckoutOrigin> {
        match &self.owner {
            FeatureRowOwner::Explicit(_) => None,
            FeatureRowOwner::CheckedOut(frame) => Some(frame.origin()),
        }
    }
    pub fn into_owner(self) -> FeatureRowOwner { self.owner }
}
impl std::ops::Deref for FeatureRow {
    type Target = [f32];
    fn deref(&self) -> &[f32] { self.samples() }
}
impl std::ops::DerefMut for FeatureRow {
    fn deref_mut(&mut self) -> &mut [f32] { self.samples_mut() }
}
