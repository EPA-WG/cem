//! Keyframe syntax belongs to CSS import, independently of DOM selectors.
use super::*;

impl CssImport<'_> {
    pub(super) fn keyframe_name(&mut self, parent: AstNodeId, start: usize, end: usize) {
        let node = self.node(parent, "keyframe-name", start, end);
        let token = self
            .condition_tokens(start, end)
            .and_then(|tokens| (tokens.len() == 1).then(|| tokens[0]));
        let valid = token.is_some_and(|i| {
            self.events[i].token_kind == "string"
                || (self.events[i].token_kind == "ident"
                    && ![
                        "none",
                        "initial",
                        "inherit",
                        "unset",
                        "revert",
                        "revert-layer",
                        "default",
                    ]
                    .iter()
                    .any(|word| self.keyword(i, word)))
        });
        self.attr(node, "syntax-valid", if valid { "true" } else { "false" });
        if valid {
            let event = &self.events[token.unwrap()];
            self.attr(node, "kind", &event.token_kind);
            self.attr(node, "value", event.value.as_deref().unwrap_or_default());
        }
        self.components(node, start, end);
    }

    pub(super) fn keyframe_selectors(&mut self, parent: AstNodeId, start: usize, end: usize) {
        let list = self.node(parent, "keyframe-selector-list", start, end);
        let offsets = self.condition_tokens(start, end).and_then(|tokens| {
            tokens
                .split(|i| self.events[*i].token_kind == "comma")
                .map(|part| {
                    if part.len() != 1 {
                        return None;
                    }
                    let i = part[0];
                    let offset = if self.keyword(i, "from") {
                        0.0
                    } else if self.keyword(i, "to") {
                        1.0
                    } else if self.events[i].token_kind == "percentage" {
                        // cssparser's numeric projection is f32. Check the token's
                        // original number here so out-of-range offsets cannot
                        // round down to 100% before semantic admission.
                        let value: f64 = self.events[i].lexeme.strip_suffix('%')?.parse().ok()?;
                        if !value.is_finite() || !(0.0..=100.0).contains(&value) {
                            return None;
                        }
                        value / 100.0
                    } else {
                        return None;
                    };
                    Some((i, offset))
                })
                .collect::<Option<Vec<_>>>()
        });
        self.attr(
            list,
            "syntax-valid",
            if offsets.is_some() { "true" } else { "false" },
        );
        self.components(list, start, end);
        if let Some(offsets) = offsets {
            for (i, offset) in offsets {
                let node = self.node(list, "keyframe-selector", i, i + 1);
                self.attr(node, "kind", &self.events[i].token_kind);
                self.attr(node, "value", &offset.to_string());
                self.attr(node, "token", &self.text(i, i + 1));
            }
        }
    }
}
