//! Additional in-process liveness restrictions. These never grant admission and
//! are deliberately absent from every portable contract representation.
use std::{fmt::Debug, sync::Arc};
pub trait DatatypeAvailability: Debug + Send + Sync {
    fn is_available(&self) -> bool;
}
#[derive(Debug, Clone, Default)]
pub(crate) struct Availability(Option<Arc<dyn DatatypeAvailability>>);
impl Availability {
    pub(crate) fn restrict(&mut self, guard: Arc<dyn DatatypeAvailability>) {
        self.0 = Some(match self.0.take() {
            Some(previous) => Arc::new(Intersection(previous, guard)),
            None => guard,
        });
    }
    pub(crate) fn is_available(&self) -> bool {
        self.0.as_ref().is_none_or(|guard| guard.is_available())
    }
}
#[derive(Debug)]
struct Intersection(Arc<dyn DatatypeAvailability>, Arc<dyn DatatypeAvailability>);
impl DatatypeAvailability for Intersection {
    fn is_available(&self) -> bool {
        self.0.is_available() && self.1.is_available()
    }
}
