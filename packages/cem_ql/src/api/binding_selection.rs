use super::EvaluationContext;
use crate::ir::CompiledQuery;

impl EvaluationContext {
    pub(crate) fn for_query(&self, query: &CompiledQuery) -> Self {
        let Some(used) = query.binding_dependencies() else {
            return self.clone();
        };
        Self {
            policy_bindings: query
                .policy_bindings
                .iter()
                .filter(|(binding, _)| used.contains(binding))
                .filter_map(|(_, name)| {
                    self.policy_bindings
                        .get(name)
                        .map(|value| (name.clone(), value.clone()))
                })
                .collect(),
            scope: self.scope,
            scope_policy: self.scope_policy,
            diagnostics: self.diagnostics.clone(),
            current_item: self.current_item.clone(),
            module_resolution: self.module_resolution.clone(),
            native_functions: self.native_functions.clone(),
            data_readers: self.data_readers.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::{compile, evaluate, CompileContext},
        eval::{AtomValue, Item, ItemStream, QueryContextScope},
        ir::{deserialize::IrDeserializer, serialize::IrSerializer},
    };
    use std::collections::BTreeMap;

    fn integer(n: i64) -> ItemStream {
        ItemStream::once(Item::Atomic(AtomValue::Integer(n)))
    }

    fn fixture(source: &str) -> (CompiledQuery, EvaluationContext) {
        let context = EvaluationContext {
            policy_bindings: BTreeMap::from([
                ("delta".into(), integer(10)),
                ("unused".into(), integer(99)),
            ]),
            scope: QueryContextScope(17),
            current_item: Some(Item::Atomic(AtomValue::String("focus".into()))),
            ..Default::default()
        };
        let query = compile(
            source,
            &CompileContext {
                policy_bindings: context.policy_bindings.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        (query, context)
    }

    #[test]
    fn binding_selection_preserves_captures_forward_functions_and_recovery() {
        for (source, expected) in [
            ("delta + 1", vec![11]),
            ("seq:map((1, 2), fn(n) => n + delta)", vec![11, 12]),
            ("seq:map((1, 2), fn(n) => seq:map((3), fn(m) => n + m + delta))", vec![14, 15]),
            ("declare function first(n) { second(n) } declare function second(n) { n + delta } first(2)", vec![12]),
            ("try { 1 / 0 } catch (code, message) { delta }", vec![10]),
            ("dom::chain((1, 2)).map(|n| n + delta)", vec![11, 12]),
        ] {
            let (query, context) = fixture(source);
            let selected = context.for_query(&query);
            assert_eq!(selected.policy_bindings.keys().map(String::as_str).collect::<Vec<_>>(), ["delta"], "{source}");
            assert_eq!(selected.scope, context.scope);
            assert_eq!(selected.current_item, context.current_item);
            assert_eq!(context.policy_bindings.len(), 2, "outer context stays complete");
            let result = evaluate(&query, &selected);
            assert_eq!(result, evaluate(&query, &context), "{source}");
            assert!(result.error.is_none(), "{source}: {result:?}");
            assert_eq!(result.items, expected.into_iter().map(|n| integer(n).items.remove(0)).collect::<Vec<_>>());
        }
    }

    #[test]
    fn binding_selection_omits_shadowed_values_and_keeps_focus() {
        for source in [
            "42",
            "{ let delta = 3; delta }",
            "seq:map((1), fn(delta) => delta + 2)",
            "dom:text()",
        ] {
            let (query, context) = fixture(source);
            let selected = context.for_query(&query);
            assert!(selected.policy_bindings.is_empty(), "{source}");
            assert_eq!(evaluate(&query, &selected), evaluate(&query, &context));
            assert!(evaluate(&query, &selected).error.is_none(), "{source}");
        }
    }

    #[test]
    fn binding_selection_keeps_full_context_for_native_and_template_callbacks() {
        for source in [
            "native:call(\"probe\")",
            "seq:map((1), fn(n) => native:call(\"probe\", n))",
            "declare function f() { native:call(\"probe\") } f()",
            "cemt:apply_templates((), \"probe\")",
        ] {
            let (query, mut context) = fixture(source);
            context
                .policy_bindings
                .insert("runtimeOnly".into(), integer(123));
            assert_eq!(
                context.for_query(&query).policy_bindings,
                context.policy_bindings,
                "{source}"
            );
        }
    }

    #[test]
    fn binding_selection_derives_from_existing_artifacts_without_new_metadata() {
        let (query, context) = fixture("seq:map((1, 2), fn(n) => n + delta)");
        let bytes = IrSerializer::serialize(&query);
        let reloaded = IrDeserializer::deserialize(&bytes).unwrap();
        // Explicit artifact serialization also used by CEMT's binary envelope.
        let serialized = serde_json::to_vec(&query).unwrap();
        let decoded: CompiledQuery = serde_json::from_slice(&serialized).unwrap();
        for candidate in [&query, &reloaded, &decoded] {
            let selected = context.for_query(candidate);
            assert_eq!(selected.policy_bindings.len(), 1);
            assert_eq!(evaluate(candidate, &selected), evaluate(&query, &context));
            assert_eq!(
                candidate.policy_bindings.len(),
                2,
                "retain the declaration/serialization contract"
            );
        }
        assert_eq!(IrSerializer::serialize(&reloaded), bytes);
    }

    #[test]
    fn binding_selection_preserves_whole_records_and_native_identity() {
        let (mut query, mut context) = fixture("delta");
        let native = evaluate(
            &compile(
                r#"data:read("<r><name>ivy</name></r>", "xml").root"#,
                &CompileContext::default(),
            )
            .unwrap(),
            &EvaluationContext::default(),
        );
        assert!(native.error.is_none(), "{native:?}");
        context.policy_bindings.insert(
            "delta".into(),
            ItemStream::once(Item::Record(BTreeMap::from([
                ("label".into(), integer(5).items),
                ("node".into(), native.items.clone()),
            ]))),
        );
        let selected = context.for_query(&query);
        assert_eq!(
            selected.policy_bindings["delta"],
            context.policy_bindings["delta"]
        );
        for source in [
            "record:entries(delta).key",
            "delta.node",
            "dom:text(delta.node)",
        ] {
            query = compile(
                source,
                &CompileContext {
                    policy_bindings: context.policy_bindings.clone(),
                    ..Default::default()
                },
            )
            .unwrap();
            let result = evaluate(&query, &context.for_query(&query));
            assert!(result.error.is_none(), "{source}: {result:?}");
            assert_eq!(result, evaluate(&query, &context));
            assert!(!result.items.is_empty(), "{source}");
        }
    }

    #[test]
    fn binding_selection_falls_back_for_unresolved_ir_calls_and_modules() {
        use crate::{
            ir::{IrId, IrNode},
            parser::QName,
            resolve::ModuleUri,
        };
        let (mut query, context) = fixture("delta");
        let root = query.tree.root.0 as usize;
        for node in [
            IrNode::Call {
                callee: IrId(u32::MAX),
                args: vec![],
            },
            IrNode::StdlibCall {
                module: ModuleUri("urn:future:extension".into()),
                name: QName {
                    prefix: None,
                    local: "call".into(),
                    range: cem_ml::source::ByteRange::new(0, 0),
                },
                args: vec![],
            },
        ] {
            query.tree.nodes[root] = node;
            assert_eq!(
                context.for_query(&query).policy_bindings,
                context.policy_bindings
            );
        }
    }
}
