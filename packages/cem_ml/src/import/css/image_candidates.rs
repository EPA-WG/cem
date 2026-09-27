//! Classify static image candidates at import; consumers only read retained roles.
use super::CssImport;
use std::collections::BTreeSet;

impl CssImport<'_> {
    pub(super) fn image_candidate_strings(
        &self,
        start: usize,
        end: usize,
    ) -> Option<BTreeSet<usize>> {
        let top = self.condition_tokens(start, end)?;
        let mut strings = BTreeSet::new();
        for option in top.split(|i| self.events[*i].token_kind == "comma") {
            let (&first, descriptors) = option.split_first()?;
            let event = &self.events[first];
            match event.token_kind.as_str() {
                "string" => {
                    strings.insert(first);
                }
                "url" => {}
                "function-open"
                    if event.value.as_deref().is_some_and(|name| {
                        [
                            "url",
                            "linear-gradient",
                            "radial-gradient",
                            "conic-gradient",
                            "repeating-linear-gradient",
                            "repeating-radial-gradient",
                            "repeating-conic-gradient",
                        ]
                        .iter()
                        .any(|known| name.eq_ignore_ascii_case(known))
                    }) => {}
                _ => return None,
            }
            let mut resolution = false;
            let mut media_type = false;
            for &i in descriptors {
                let event = &self.events[i];
                if event.token_kind == "dimension" && !resolution {
                    // Decode the imported dimension token, including escaped units.
                    let mut input = cssparser::ParserInput::new(&event.lexeme);
                    let mut parser = cssparser::Parser::new(&mut input);
                    let Ok(cssparser::Token::Dimension { unit, .. }) = parser.next() else {
                        return None;
                    };
                    if !["x", "dppx", "dpi", "dpcm"]
                        .iter()
                        .any(|known| unit.eq_ignore_ascii_case(known))
                    {
                        return None;
                    }
                    resolution = true;
                } else if event.token_kind == "function-open"
                    && !media_type
                    && event
                        .value
                        .as_deref()
                        .is_some_and(|name| name.eq_ignore_ascii_case("type"))
                {
                    let args = self.condition_tokens(i + 1, self.close(i, end))?;
                    if args.len() != 1 || self.events[args[0]].token_kind != "string" {
                        return None;
                    }
                    media_type = true;
                } else {
                    return None;
                }
            }
        }
        Some(strings)
    }
}
