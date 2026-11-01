use serde::Serialize;
use std::sync::Arc;

/// Writes the same values as ordinary serialization, with reusable asset segments.
pub trait Capture {
    type Error;
    fn value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Self::Error>;
    fn shared<T: Serialize + Clone + PartialEq + 'static>(
        &mut self,
        value: &T,
    ) -> Result<(), Self::Error>;
    fn words(&mut self, value: &[u16]) -> Result<(), Self::Error>;
    fn shared_arc<T: Serialize + 'static>(
        &mut self,
        value: &Arc<T>,
        stable: bool,
    ) -> Result<(), Self::Error>;
}
