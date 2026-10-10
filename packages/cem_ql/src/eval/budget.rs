//! Runtime-only work shared by an enclosing invocation and cooperative reentry.
use super::{BudgetAxis, ItemStream};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Debug, Default)]
struct State {
    counters: HashMap<BudgetAxis, u64>,
    ceilings: HashMap<BudgetAxis, u64>,
    depth: u64,
    failure: Option<ItemStream>,
}
/// Clones share cumulative work, active call depth and the first budget failure.
/// Keep one per enclosing invocation; independent requests use fresh budgets.
#[derive(Debug, Clone, Default)]
pub struct QueryExecutionBudget(Arc<Mutex<State>>);
impl QueryExecutionBudget {
    pub(crate) fn charge(&self, axis: BudgetAxis, amount: u64, limit: u64) -> Result<u64, ()> {
        let mut state = self.0.lock().unwrap();
        let ceiling = state.ceilings.entry(axis).or_insert(limit);
        *ceiling = (*ceiling).min(limit);
        let limit = *ceiling;
        let used = state.counters.entry(axis).or_default();
        *used = used.saturating_add(amount);
        if *used > limit {
            Err(())
        } else {
            Ok(*used)
        }
    }
    pub(crate) fn enter(&self, limit: u64) -> bool {
        let mut state = self.0.lock().unwrap();
        let ceiling = state.ceilings.entry(BudgetAxis::CallDepth).or_insert(limit);
        *ceiling = (*ceiling).min(limit);
        let ceiling = *ceiling;
        if state.depth >= ceiling {
            return false;
        }
        state.depth += 1;
        true
    }
    pub(crate) fn exit(&self) {
        let mut state = self.0.lock().unwrap();
        state.depth = state.depth.saturating_sub(1);
    }
    pub(crate) fn retain_failure(&self, stream: ItemStream) -> ItemStream {
        self.0.lock().unwrap().failure.get_or_insert(stream).clone()
    }
    pub(crate) fn failure(&self) -> Option<ItemStream> {
        self.0.lock().unwrap().failure.clone()
    }
}
