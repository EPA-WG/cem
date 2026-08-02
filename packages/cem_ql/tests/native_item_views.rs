use cem_ml::scheduler::ScopePolicy;
use cem_ml::source::{ByteRange, SourceId};
use cem_ml::source_map::{FrameSpan, SourceMapFrame, SourceMapStack, TransformKind};
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{
    AtomValue, Item, ItemStream, QueryContextScope, QueryItemView, QueryItemViewKind,
};
use std::collections::BTreeMap;

#[derive(Debug)]
struct FixtureView {
    id: &'static str,
    fields: BTreeMap<String, Vec<Item>>,
    atom: Option<AtomValue>,
    members: Option<Vec<Item>>,
    source_map: SourceMapStack,
}

impl QueryItemView for FixtureView {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn representation_id(&self) -> &'static str {
        "test.native-view"
    }

    fn identity(&self) -> String {
        self.id.to_owned()
    }

    fn kind(&self) -> QueryItemViewKind {
        if self.atom.is_some() {
            QueryItemViewKind::Atomic
        } else if self.members.is_some() {
            QueryItemViewKind::Array
        } else {
            QueryItemViewKind::Record
        }
    }

    fn fields(&self) -> Option<Vec<(String, Vec<Item>)>> {
        (!self.fields.is_empty()).then(|| {
            self.fields
                .iter()
                .map(|(name, items)| (name.clone(), items.clone()))
                .collect()
        })
    }

    fn field(&self, name: &str) -> Option<Vec<Item>> {
        self.fields.get(name).cloned()
    }

    fn members(&self) -> Option<Vec<Item>> {
        self.members.clone()
    }

    fn atom(&self) -> Option<AtomValue> {
        self.atom.clone()
    }

    fn source_map(&self) -> Option<SourceMapStack> {
        Some(self.source_map.clone())
    }
}

fn source_map(offset: u64) -> SourceMapStack {
    SourceMapStack {
        frames: vec![SourceMapFrame {
            source_id: SourceId(7),
            span: FrameSpan::Single(ByteRange::new(offset, 3)),
            transform: TransformKind::Query,
        }],
    }
}

fn native_atom(id: &'static str, value: AtomValue, offset: u64) -> Item {
    Item::native(FixtureView {
        id,
        fields: BTreeMap::new(),
        atom: Some(value),
        members: None,
        source_map: source_map(offset),
    })
}

#[test]
fn native_item_view_projects_fields_members_atoms_identity_and_source_maps() {
    let first = native_atom("name:0", AtomValue::String("first".to_owned()), 10);
    let second = native_atom("name:1", AtomValue::String("second".to_owned()), 20);
    let names = Item::native(FixtureView {
        id: "names",
        fields: BTreeMap::new(),
        atom: None,
        members: Some(vec![first.clone(), second.clone()]),
        source_map: source_map(9),
    });
    let input = Item::native(FixtureView {
        id: "root",
        fields: BTreeMap::from([("names".to_owned(), vec![names])]),
        atom: None,
        members: None,
        source_map: source_map(0),
    });

    let mut compile_context = CompileContext::default();
    compile_context
        .policy_bindings
        .insert("input".to_owned(), ItemStream::empty());
    let query = compile("input.names", &compile_context).expect("native-view field query compiles");
    let result = evaluate(
        &query,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root(),
            diagnostics: Vec::new(),
            policy_bindings: BTreeMap::from([("input".to_owned(), ItemStream::once(input))]),
            current_item: None,
        },
    );

    assert!(result.error.is_none(), "{:?}", result.error);
    let members = result.items[0]
        .view()
        .expect("array remains a native view")
        .members()
        .expect("native array exposes members");
    assert_eq!(members.len(), 2);
    assert_eq!(
        members[0].atom(),
        Some(AtomValue::String("first".to_owned()))
    );
    assert_eq!(
        members[1].atom(),
        Some(AtomValue::String("second".to_owned()))
    );
    assert_eq!(members[0].identity().as_deref(), Some("name:0"));
    assert_eq!(
        members[1]
            .source_map()
            .and_then(|source_map| source_map.current().map(|frame| frame.source_id)),
        Some(SourceId(7))
    );
}
