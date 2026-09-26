//! Classify longhand names at import; emission never parses declaration text.
use super::*;

impl CssImport<'_> {
    pub(super) fn animation_names(&mut self, parent: AstNodeId, start: usize, end: usize) {
        let list = self.node(parent, "animation-name-list", start, end);
        let Some(top) = self.condition_tokens(start, end) else {
            self.attr(list, "analysis-status", "unsupported");
            return;
        };
        if top.iter().any(|i| {
            self.events[*i].token_kind == "function-open"
                && self.events[*i].value.as_deref().is_some_and(|name| {
                    ["var", "env", "attr"]
                        .iter()
                        .any(|v| name.eq_ignore_ascii_case(v))
                })
        }) {
            self.attr(list, "analysis-status", "dynamic");
            return;
        }
        let slots = top
            .split(|i| self.events[*i].token_kind == "comma")
            .map(|part| {
                if part.len() != 1 {
                    return None;
                }
                let i = part[0];
                let kind = match self.events[i].token_kind.as_str() {
                    "string" => "string",
                    "ident" => {
                        if self.keyword(i, "default") {
                            return None;
                        }
                        let wide = ["initial", "inherit", "unset", "revert", "revert-layer"]
                            .iter()
                            .any(|v| self.keyword(i, v));
                        if wide && top.len() != 1 {
                            return None;
                        }
                        if wide || self.keyword(i, "none") {
                            "keyword"
                        } else {
                            "ident"
                        }
                    }
                    _ => return None,
                };
                Some((i, kind))
            })
            .collect::<Option<Vec<_>>>();
        self.attr(
            list,
            "analysis-status",
            if slots.is_some() {
                "complete"
            } else {
                "unsupported"
            },
        );
        if let Some(slots) = slots {
            for (i, kind) in slots {
                let slot = self.node(list, "animation-name-slot", i, i + 1);
                self.attr(slot, "kind", kind);
                self.attr(
                    slot,
                    "value",
                    self.events[i].value.as_deref().unwrap_or_default(),
                );
                self.attr(slot, "token", &self.events[i].lexeme);
            }
        }
    }
}
