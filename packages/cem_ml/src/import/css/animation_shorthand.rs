//! A bounded static shorthand profile, classified only at CSS import.
use super::*;

impl CssImport<'_> {
    pub(super) fn animation_shorthand(&mut self, parent: AstNodeId, start: usize, end: usize) {
        let list = self.node(parent, "animation-name-list", start, end);
        // Substitution can supply multiple tokens, commas, or an entire animation.
        if self.events[start..end].iter().any(|e| {
            e.token_kind == "function-open"
                && e.value.as_deref().is_some_and(|v| {
                    ["var", "env", "attr"]
                        .iter()
                        .any(|n| v.eq_ignore_ascii_case(n))
                })
        }) {
            self.attr(list, "analysis-status", "dynamic");
            return;
        }
        let slots = self.condition_tokens(start, end).and_then(|top| {
            let mut slots = Vec::new();
            for group in top.split(|i| self.events[*i].token_kind == "comma") {
                if group.is_empty() {
                    return None;
                }
                let mut used = std::collections::BTreeSet::new();
                for &i in group {
                    let event = &self.events[i];
                    let (role, kind) = match event.token_kind.as_str() {
                        "string" => ("name", "string"),
                        "ident" => {
                            if self.keyword(i, "default") {
                                return None;
                            }
                            if ["initial", "inherit", "unset", "revert", "revert-layer"]
                                .iter()
                                .any(|k| self.keyword(i, k))
                            {
                                if top.len() != 1 {
                                    return None;
                                }
                                ("name", "keyword")
                            } else {
                                let category =
                                    [
                                        (
                                            "easing",
                                            &[
                                                "ease",
                                                "linear",
                                                "ease-in",
                                                "ease-out",
                                                "ease-in-out",
                                                "step-start",
                                                "step-end",
                                            ][..],
                                        ),
                                        ("iteration", &["infinite"][..]),
                                        (
                                            "direction",
                                            &[
                                                "normal",
                                                "reverse",
                                                "alternate",
                                                "alternate-reverse",
                                            ][..],
                                        ),
                                        ("fill", &["none", "forwards", "backwards", "both"][..]),
                                        ("play", &["running", "paused"][..]),
                                    ]
                                    .into_iter()
                                    .find(|(role, keys)| {
                                        !used.contains(role)
                                            && keys.iter().any(|k| self.keyword(i, k))
                                    });
                                if let Some((role, _)) = category {
                                    (role, role)
                                } else {
                                    (
                                        "name",
                                        if self.keyword(i, "none") {
                                            "keyword"
                                        } else {
                                            "ident"
                                        },
                                    )
                                }
                            }
                        }
                        "dimension" => {
                            let mut input = cssparser::ParserInput::new(&event.lexeme);
                            let mut parser = cssparser::Parser::new(&mut input);
                            let cssparser::Token::Dimension { value, unit, .. } =
                                parser.next().ok()?
                            else {
                                return None;
                            };
                            if !value.is_finite()
                                || !(unit.eq_ignore_ascii_case("s")
                                    || unit.eq_ignore_ascii_case("ms"))
                            {
                                return None;
                            }
                            let role = if used.contains("duration") {
                                "delay"
                            } else {
                                "duration"
                            };
                            let number_end = event
                                .lexeme
                                .find(|c: char| {
                                    !c.is_ascii_digit() && !matches!(c, '+' | '-' | '.' | 'e' | 'E')
                                })
                                .unwrap_or(event.lexeme.len());
                            let precise = event.lexeme[..number_end].parse::<f64>().ok()?;
                            if !precise.is_finite() || (role == "duration" && precise < 0.0) {
                                return None;
                            }
                            (role, role)
                        }
                        "number" => {
                            if event
                                .lexeme
                                .parse::<f64>()
                                .ok()
                                .is_none_or(|v| !v.is_finite() || v < 0.0)
                            {
                                return None;
                            }
                            ("iteration", "iteration")
                        }
                        "function-open" if self.animation_easing(i, end) => ("easing", "easing"),
                        _ => return None,
                    };
                    if !used.insert(role) {
                        return None;
                    }
                    let next = if event.kind == "block-open" {
                        self.close(i, end) + 1
                    } else {
                        i + 1
                    };
                    slots.push((i, next, role, kind));
                }
            }
            Some(slots)
        });
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
            for (i, next, role, kind) in slots {
                let slot = self.node(
                    list,
                    if role == "name" {
                        "animation-name-slot"
                    } else {
                        "animation-value-slot"
                    },
                    i,
                    next,
                );
                self.attr(slot, "kind", kind);
                self.attr(
                    slot,
                    "value",
                    self.events[i].value.as_deref().unwrap_or_default(),
                );
                self.attr(slot, "token", &self.text(i, next));
            }
        }
    }

    fn animation_easing(&self, i: usize, end: usize) -> bool {
        let Some(tokens) = self.condition_tokens(i + 1, self.close(i, end)) else {
            return false;
        };
        let number = |j: usize| {
            (self.events[j].token_kind == "number")
                .then(|| self.events[j].lexeme.parse::<f64>().ok())
                .flatten()
                .filter(|v| v.is_finite())
        };
        let name = self.events[i].value.as_deref().unwrap_or_default();
        if name.eq_ignore_ascii_case("cubic-bezier") {
            if tokens.len() != 7
                || [1, 3, 5]
                    .iter()
                    .any(|p| self.events[tokens[*p]].token_kind != "comma")
            {
                return false;
            }
            let values: Option<Vec<_>> = [0, 2, 4, 6].iter().map(|p| number(tokens[*p])).collect();
            return values
                .is_some_and(|v| (0.0..=1.0).contains(&v[0]) && (0.0..=1.0).contains(&v[2]));
        }
        if name.eq_ignore_ascii_case("steps") && matches!(tokens.len(), 1 | 3) {
            let mut input = cssparser::ParserInput::new(&self.events[tokens[0]].lexeme);
            let mut parser = cssparser::Parser::new(&mut input);
            let Ok(cssparser::Token::Number {
                int_value: Some(count),
                ..
            }) = parser.next()
            else {
                return false;
            };
            if *count <= 0 {
                return false;
            }
            return tokens.len() == 1
                || (self.events[tokens[1]].token_kind == "comma"
                    && [
                        "start",
                        "end",
                        "jump-start",
                        "jump-end",
                        "jump-none",
                        "jump-both",
                    ]
                    .iter()
                    .any(|v| self.keyword(tokens[2], v))
                    && (!self.keyword(tokens[2], "jump-none") || *count > 1));
        }
        false
    }
}
