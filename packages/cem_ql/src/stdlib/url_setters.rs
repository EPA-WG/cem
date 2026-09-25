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

pub fn set_host(url: &mut Url, value: &str) -> SetterOutcome {
    // WHATWG preprocessing removes ASCII tabs/newlines, not arbitrary whitespace.
    let value: String = value
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    if quirks::set_host(url, &value).is_err() {
        return SetterOutcome::Ignored("host");
    }
    let special = matches!(
        url.scheme(),
        "http" | "https" | "ftp" | "ws" | "wss" | "file"
    );
    let authority = value
        .split(|c| matches!(c, '/' | '?' | '#') || (special && c == '\\'))
        .next()
        .unwrap();
    // A colon inside an IPv6 literal is not a port separator.
    let port = if authority.starts_with('[') {
        authority
            .split_once(']')
            .and_then(|(_, tail)| tail.strip_prefix(':'))
    } else {
        authority.split_once(':').map(|(_, port)| port)
    };
    if let Some(port) = port.filter(|port| !port.is_empty()) {
        // Preserve the already-applied hostname. Probe the same setter grammar
        // on a private copy so invalid/overflow ports are classified even when
        // the dependency's composite host setter swallowed their failure.
        let mut probe = url.clone();
        if quirks::set_port(&mut probe, port).is_err() {
            return SetterOutcome::Ignored("host.port");
        }
    }
    SetterOutcome::Applied
}

pub fn set_protocol(url: &mut Url, value: &str) -> SetterOutcome {
    match quirks::set_protocol(url, value) {
        Ok(()) => SetterOutcome::Applied,
        Err(()) => SetterOutcome::Ignored("protocol"),
    }
}
