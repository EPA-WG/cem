//! Component operations for a private URL clone owned by a future parts adapter.
//! These primitives are not registered query functions.
use url::{quirks, Position, Url};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetterOutcome {
    Applied,
    Ignored(&'static str),
}

pub fn set_port(url: &mut Url, value: &str) -> SetterOutcome {
    // The empty string explicitly clears a port. A nonempty input consisting
    // only of stripped ASCII tab/newline characters instead leaves it alone.
    // url 2.5.8 conflates these two inputs after preprocessing.
    if !value.is_empty() && value.bytes().all(|b| matches!(b, b'\t' | b'\n' | b'\r')) {
        return SetterOutcome::Ignored("port");
    }
    match quirks::set_port(url, value) {
        Ok(()) => SetterOutcome::Applied,
        Err(()) => SetterOutcome::Ignored("port"),
    }
}

pub fn set_pathname(url: &mut Url, value: &str) -> SetterOutcome {
    if url.cannot_be_a_base() {
        return SetterOutcome::Ignored("pathname");
    }
    let hostless = !url.has_authority();
    quirks::set_pathname(url, value);
    if hostless {
        // The dependency normalizes the new path but can retain a stale `/.`
        // prefix or omit the guard required before a path starting with `//`.
        // Rebuild only this hostless hierarchical serialization from components,
        // never from the dependency's ambiguous serialized authority prefix.
        let guard = if url.path().starts_with("//") {
            "/."
        } else {
            ""
        };
        let serialized = format!(
            "{}:{}{}{}",
            url.scheme(),
            guard,
            url.path(),
            &url[Position::AfterPath..]
        );
        *url = Url::parse(&serialized).expect("normalized hostless URL components remain valid");
    }
    SetterOutcome::Applied
}
