//! Stable wire ordering for passive lookup maps; runtime lookup and source order stay unchanged.
use serde::{Serialize, Serializer};
use std::collections::HashMap;

pub(super) fn serialize_string_map<S: Serializer, V: Serialize>(
    map: &HashMap<String, V>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let mut entries: Vec<_> = map.iter().collect();
    entries.sort_unstable_by(|left, right| left.0.cmp(right.0));
    serializer.collect_map(entries)
}
