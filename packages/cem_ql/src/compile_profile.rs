//! Opt-in, thread-local measurements for native compiler and rendering profiling tests only.
use std::{
    cell::RefCell,
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Default, Debug)]
pub(crate) struct Stage {
    pub calls: usize,
    pub elapsed: Duration,
}
pub(crate) type Stages = BTreeMap<&'static str, Stage>;
thread_local! {
    static ACTIVE: RefCell<Option<Stages>> = const { RefCell::new(None) };
}

pub(crate) fn measure<T>(operation: impl FnOnce() -> T) -> (T, Stages) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            ACTIVE.with(|active| *active.borrow_mut() = None);
        }
    }
    ACTIVE.with(|active| {
        assert!(active.borrow().is_none(), "nested profiling session");
        *active.borrow_mut() = Some(Stages::new());
    });
    let _reset = Reset;
    let value = operation();
    let stages = ACTIVE.with(|active| active.borrow_mut().take().unwrap());
    (value, stages)
}

pub(crate) struct Span {
    label: &'static str,
    start: Option<Instant>,
}
impl Span {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            start: ACTIVE.with(|active| active.borrow().as_ref().map(|_| Instant::now())),
        }
    }
    pub fn next(&mut self, label: &'static str) {
        self.finish();
        *self = Self::new(label);
    }
    fn finish(&mut self) {
        if let Some(start) = self.start.take() {
            let elapsed = start.elapsed();
            ACTIVE.with(|active| {
                if let Some(stages) = active.borrow_mut().as_mut() {
                    let stage = stages.entry(self.label).or_default();
                    stage.calls += 1;
                    stage.elapsed += elapsed;
                }
            });
        }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        self.finish();
    }
}
