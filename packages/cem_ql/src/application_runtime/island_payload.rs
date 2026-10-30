//! Review the rendered payload closure without rewriting its retained source.
use super::*;
const HTML: &str = "http://www.w3.org/1999/xhtml";
fn check(work: &mut usize, control: &OperationControl) -> Result<(), String> {
    control
        .check_scope(ROOT_EXECUTION_SCOPE_ID)
        .map_err(|_| INVALID)?;
    *work = work.saturating_add(1);
    if *work > super::super::inputs::LIMITS.max_values {
        return Err(INVALID.into());
    }
    Ok(())
}
fn targets(item: &Item) -> Result<Vec<Item>, String> {
    item.view()
        .and_then(|view| view.field("targets"))
        .ok_or(INVALID.into())
}
pub(super) fn slots(
    payload: &[Item],
    control: &OperationControl,
) -> Result<BTreeMap<String, Vec<Item>>, String> {
    // The shared CEMV decoder has already checked the complete graph's edges,
    // DAG, depth and expanded work. Review every rendered target, including
    // targets below nested HTML, before publishing any slot projection.
    let mut pending = payload.to_vec();
    let mut work = 0;
    while let Some(item) = pending.pop() {
        check(&mut work, control)?;
        if !has_source(&item) {
            return Err(INVALID.into());
        }
        match field(&item, "kind").as_deref() {
            Some("reference") => pending.extend(targets(&item)?),
            Some("element") => {
                if field(&item, "namespace").as_deref() != Some(HTML) {
                    return Err(INVALID.into());
                }
                let attributes = attrs(&item)?;
                let tag = field(&item, "name").ok_or(INVALID)?;
                if tag != tag.to_ascii_lowercase()
                    || attributes
                        .keys()
                        .any(|name| *name != name.to_ascii_lowercase())
                {
                    return Err(INVALID.into());
                }
                if (matches!(tag.as_str(), "input" | "textarea")
                    && attributes.get("type").is_some_and(|v| {
                        matches!(v.to_ascii_lowercase().as_str(), "password" | "file")
                    }))
                    || attributes.get("autocomplete").is_some_and(|v| {
                        v.split_ascii_whitespace().any(|part| {
                            matches!(
                                part.to_ascii_lowercase().as_str(),
                                "one-time-code" | "current-password" | "new-password"
                            )
                        })
                    })
                {
                    return Err(INVALID.into());
                }
                pending.extend(children(&item)?);
            }
            Some("text" | "whitespace" | "cdata" | "comment") => {}
            _ => return Err(INVALID.into()),
        }
    }
    // Only direct rendered roots determine slots. Preserve repetitions and
    // encounter order; nested references stay inside their source elements.
    // Original reference occurrences remain visible through datadom.payload.
    let mut pending = payload.iter().rev().cloned().collect::<Vec<_>>();
    let mut slots = BTreeMap::<String, Vec<Item>>::new();
    let mut work = 0;
    while let Some(item) = pending.pop() {
        check(&mut work, control)?;
        if field(&item, "kind").as_deref() == Some("reference") {
            pending.extend(targets(&item)?.into_iter().rev());
            continue;
        }
        let name = if field(&item, "kind").as_deref() == Some("element") {
            attrs(&item)?.remove("slot").unwrap_or_default()
        } else {
            String::new()
        };
        slots.entry(name).or_default().push(item);
    }
    Ok(slots)
}
