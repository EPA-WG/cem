//! XSLT catch selection/scoping over generic buffered CEMT recovery.
use super::*;
use cem_ml::validation::xpath::{
    XPathAxis, XPathExpression, XPathNameTest, XPathNodeTest, XPathPathRoot, XPathStep,
};
const ERR: &str = "http://www.w3.org/2005/xqt-errors";

impl Compiler<'_> {
    pub(super) fn recover(
        &mut self,
        node: &AuthorNode<'_>,
        scope: &Scope,
    ) -> CompileResult<String> {
        self.attributes(node.event, &["select", "rollback-output"])?;
        let catch = |node: &AuthorNode<'_>| {
            node.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                && node.event.local_name.as_deref() == Some("catch")
        };
        let Some(first) = node.children.iter().position(catch) else {
            return Err(self.error(node.event, "XTSE0010", "xsl:try requires a catch"));
        };
        if self.attribute(node.event, "select").is_some()
            && node.children[..first].iter().any(|n| !ignorable(n))
        {
            return Err(self.error(
                node.event,
                "XTSE3140",
                "try select requires empty protected content",
            ));
        }
        if let Some(attribute) = self.attribute(node.event, "rollback-output") {
            match self.literal_attribute(node.event, attribute)?.as_str() {
                "yes" | "true" | "1" | "no" | "false" | "0" => {}
                _ => {
                    return Err(self.error(
                        node.event,
                        "XTSE0020",
                        "rollback-output requires a boolean",
                    ))
                }
            }
            // XSLT permits buffering even when rollback-output is no.
        }
        let body = if self.attribute(node.event, "select").is_some() {
            self.result_select(node.event, scope)?
        } else {
            self.sequence(&node.children[..first], scope.clone())?
        };
        let mut output = format!("{{try | {body}");
        for handler in &node.children[first..] {
            if ignorable(handler) {
                continue;
            }
            if !catch(handler) {
                return Err(self.error(
                    handler.event,
                    "XTSE0010",
                    "only catches may follow the protected sequence",
                ));
            }
            self.attributes(handler.event, &["errors", "select"])?;
            if self.attribute(handler.event, "select").is_some()
                && handler.children.iter().any(|n| !ignorable(n))
            {
                return Err(self.error(
                    handler.event,
                    "XTSE3150",
                    "catch select requires empty content",
                ));
            }
            let error = self.fresh();
            let names = self
                .attribute(handler.event, "errors")
                .and_then(|a| a.entity_decoded_value.as_deref())
                .unwrap_or("*");
            let tests = names
                .split_whitespace()
                .map(|name| self.catch_test(handler.event, name, &error))
                .collect::<CompileResult<Vec<_>>>()?;
            if tests.is_empty() {
                return Err(self.error(
                    handler.event,
                    "XTSE0010",
                    "catch errors requires at least one name test",
                ));
            }
            let test = tests
                .into_iter()
                .map(|s| format!("({s})"))
                .collect::<Vec<_>>()
                .join(" || ");
            let mut inner = scope.clone();
            for (name, field) in [
                ("code", "error_qname"),
                ("description", "message"),
                ("module", "uri"),
                ("line-number", "line"),
                ("column-number", "column"),
            ] {
                inner.variables.insert(
                    XPathExpandedName::new(Some(ERR), name),
                    format!("{error}.{field}"),
                );
            }
            inner
                .variables
                .insert(XPathExpandedName::new(Some(ERR), "value"), "()".into());
            let body = if self.attribute(handler.event, "select").is_some() {
                self.result_select(handler.event, &inner)?
            } else {
                self.sequence(&handler.children, inner)?
            };
            output.push_str(&format!(
                "{{catch @as={} @test={} | {body}}}",
                quote(&error),
                query_attribute(&test)
            ));
        }
        output.push('}');
        Ok(output)
    }

    fn catch_test(&self, event: &XmlEventAst, name: &str, binding: &str) -> CompileResult<String> {
        // Parse the authored NameTest through XPath's namespace-aware grammar.
        let parsed = self.generated_xpath(event, name)?;
        let syntax = parsed
            .syntax_ast
            .as_ref()
            .expect("generated_xpath validates syntax");
        let invalid = || {
            self.error(
                event,
                "XTSE0280",
                "catch errors must contain declared QName/wildcard name tests",
            )
        };
        let [expression] = syntax.root.expressions.as_slice() else {
            return Err(invalid());
        };
        let XPathExpression::Path(path) = &expression.expression else {
            return Err(invalid());
        };
        if path.root != XPathPathRoot::Relative {
            return Err(invalid());
        }
        let [step] = path.steps.as_slice() else {
            return Err(invalid());
        };
        let XPathStep::Axis {
            axis: XPathAxis::Child,
            node_test: XPathNodeTest::Name(test),
            predicates,
        } = &step.step
        else {
            return Err(invalid());
        };
        if !predicates.is_empty() {
            return Err(invalid());
        }
        Ok(match test {
            XPathNameTest::Name(name) => format!(
                "{binding}.error_namespace == {} && {binding}.error_local == {}",
                quote(name.namespace_uri.as_deref().unwrap_or("")),
                quote(&name.local_name)
            ),
            XPathNameTest::Any => "true".into(),
            XPathNameTest::Namespace { namespace_uri } => {
                format!("{binding}.error_namespace == {}", quote(namespace_uri))
            }
            XPathNameTest::AnyNamespace { local_name } => {
                format!("{binding}.error_local == {}", quote(local_name))
            }
        })
    }
}
