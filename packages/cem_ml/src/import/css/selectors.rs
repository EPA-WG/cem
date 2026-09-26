//! Project stylesheet selector structure only at the shared CSS import boundary.
use super::*;
use crate::validation::css_selector::{
    stylesheet_selector_structure, CssSelectorAttributeModifier, CssSelectorAttributeOperator,
    CssSelectorCombinator, CssSelectorListAst, CssSelectorNamespace, CssSelectorSimpleSelector,
    CssSelectorSourceRange,
};

impl CssImport<'_> {
    pub(super) fn selectors(&mut self, parent: AstNodeId, start: usize, end: usize, nested: bool) {
        if let Some(list) = stylesheet_selector_structure(&self.events[start..end], nested) {
            self.selector_list(parent, &list, nested);
        } else {
            let id = self.node(parent, "selector-list", start, end);
            self.attr(id, "analysis-status", "unsupported");
        }
    }

    fn selector_node(
        &mut self,
        parent: AstNodeId,
        name: &str,
        range: CssSelectorSourceRange,
    ) -> AstNodeId {
        let id = self
            .b
            .element(parent, CSS_SCHEMA_URI, name, range.source_map());
        self.b.semantics.ranges.insert(
            id,
            CemTreeRange {
                line: range.start.line,
                column: range.start.column,
                offset: range.start.byte_offset,
                length: range.byte_length,
            },
        );
        id
    }

    fn selector_list(&mut self, parent: AstNodeId, list: &CssSelectorListAst, nested: bool) {
        let id = self.selector_node(parent, "selector-list", list.source_range);
        self.attr(id, "analysis-status", "complete");
        for selector in &list.selectors {
            let node = self.selector_node(id, "selector", selector.source_range);
            if nested
                || selector
                    .compounds
                    .iter()
                    .flat_map(|c| &c.simple_selectors)
                    .any(contains_nesting)
            {
                self.attr(node, "specificity-kind", "parent-dependent");
            } else {
                let (a, b, c) = selector.specificity;
                self.attr(node, "specificity", &format!("{a}-{b}-{c}"));
            }
            let mut previous = selector.source_range.start.byte_offset;
            for (index, compound) in selector.compounds.iter().enumerate() {
                let combinator = if index == 0 {
                    selector.leading_combinator
                } else {
                    Some(selector.combinators[index - 1])
                };
                if let Some(combinator) = combinator {
                    let start = self
                        .events
                        .partition_point(|e| e.source_range.start.byte_offset < previous);
                    let end = self.events.partition_point(|e| {
                        e.source_range.start.byte_offset < compound.source_range.start.byte_offset
                    });
                    let comb = self.node(node, "combinator", start, end);
                    self.attr(
                        comb,
                        "kind",
                        match combinator {
                            CssSelectorCombinator::Descendant => "descendant",
                            CssSelectorCombinator::Child => "child",
                            CssSelectorCombinator::NextSibling => "next-sibling",
                            CssSelectorCombinator::SubsequentSibling => "subsequent-sibling",
                        },
                    );
                }
                let compound_node =
                    self.selector_node(node, "compound-selector", compound.source_range);
                for simple in &compound.simple_selectors {
                    self.simple_selector(compound_node, simple);
                }
                previous = compound.source_range.end_byte_offset();
            }
        }
    }

    fn selector_namespace(&mut self, id: AstNodeId, namespace: &CssSelectorNamespace) {
        self.attr(
            id,
            "namespace",
            match namespace {
                CssSelectorNamespace::Any => "*",
                CssSelectorNamespace::None => "",
                CssSelectorNamespace::Default { namespace_uri }
                | CssSelectorNamespace::Named { namespace_uri, .. } => namespace_uri,
            },
        );
    }

    fn simple_selector(&mut self, parent: AstNodeId, simple: &CssSelectorSimpleSelector) {
        let range = match simple {
            CssSelectorSimpleSelector::Nesting { source_range }
            | CssSelectorSimpleSelector::Type { source_range, .. }
            | CssSelectorSimpleSelector::Id { source_range, .. }
            | CssSelectorSimpleSelector::Class { source_range, .. }
            | CssSelectorSimpleSelector::Attribute { source_range, .. }
            | CssSelectorSimpleSelector::PseudoClass { source_range, .. }
            | CssSelectorSimpleSelector::PseudoElement { source_range, .. } => *source_range,
        };
        let id = self.selector_node(parent, "simple-selector", range);
        match simple {
            CssSelectorSimpleSelector::Nesting { .. } => {
                self.attr(id, "kind", "nesting");
            }
            CssSelectorSimpleSelector::Type {
                namespace,
                local_name,
                universal,
                ..
            } => {
                self.attr(id, "kind", if *universal { "universal" } else { "type" });
                self.attr(id, "name", local_name);
                self.selector_namespace(id, namespace);
            }
            CssSelectorSimpleSelector::Id { value, .. }
            | CssSelectorSimpleSelector::Class { value, .. } => {
                self.attr(
                    id,
                    "kind",
                    if matches!(simple, CssSelectorSimpleSelector::Id { .. }) {
                        "id"
                    } else {
                        "class"
                    },
                );
                self.attr(id, "value", value);
            }
            CssSelectorSimpleSelector::Attribute {
                namespace,
                local_name,
                operator,
                value,
                modifier,
                ..
            } => {
                self.attr(id, "kind", "attribute");
                self.attr(id, "name", local_name);
                self.selector_namespace(id, namespace);
                if let Some(operator) = operator {
                    self.attr(
                        id,
                        "operator",
                        match operator {
                            CssSelectorAttributeOperator::Equals => "=",
                            CssSelectorAttributeOperator::Includes => "~=",
                            CssSelectorAttributeOperator::DashMatch => "|=",
                            CssSelectorAttributeOperator::PrefixMatch => "^=",
                            CssSelectorAttributeOperator::SuffixMatch => "$=",
                            CssSelectorAttributeOperator::SubstringMatch => "*=",
                        },
                    );
                }
                if let Some(value) = value {
                    self.attr(id, "value", value);
                }
                if let Some(modifier) = modifier {
                    self.attr(
                        id,
                        "modifier",
                        match modifier {
                            CssSelectorAttributeModifier::AsciiInsensitive => "i",
                            CssSelectorAttributeModifier::CaseSensitive => "s",
                        },
                    );
                }
            }
            CssSelectorSimpleSelector::PseudoElement { name, .. } => {
                self.attr(id, "kind", "pseudo-element");
                self.attr(id, "name", name);
            }
            CssSelectorSimpleSelector::PseudoClass {
                name,
                selectors,
                relative,
                ..
            } => {
                self.attr(id, "kind", "pseudo-class");
                self.attr(id, "name", name);
                self.attr(id, "relative", if *relative { "true" } else { "false" });
                if let Some(selectors) = selectors {
                    self.selector_list(id, selectors, false);
                }
            }
        }
    }
}

fn contains_nesting(simple: &CssSelectorSimpleSelector) -> bool {
    match simple {
        CssSelectorSimpleSelector::Nesting { .. } => true,
        CssSelectorSimpleSelector::PseudoClass {
            selectors: Some(list),
            ..
        } => list
            .selectors
            .iter()
            .flat_map(|s| &s.compounds)
            .flat_map(|c| &c.simple_selectors)
            .any(contains_nesting),
        _ => false,
    }
}
