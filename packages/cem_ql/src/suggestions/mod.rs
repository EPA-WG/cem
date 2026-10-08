//! Native suggestions adaptation. Sources remain original nodes; this module
//! neither opens a surface nor writes an editor or manufactures placement grants.
use crate::eval::{
    AtomValue, Item, ItemStream, QueryContextScope, QueryItemView, QueryItemViewKind,
    QueryNodeAccessError, QueryNodeIterator, QueryNodeTextIterator,
};
use cem_ml::{source_map::SourceMapStack, value::artifact::CemValueArtifactLimits};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{BTreeMap, BTreeSet},
    sync::{Arc, OnceLock},
};

pub const UNICODE_VERSION: &str = "17.0.0";
pub const VIEW_NAMESPACE: &str = "https://cem.dev/ns/runtime/suggestions/v1";
pub const CONSUMER_IDENTITY: &str = "cem-suggestions-v1-unicode-17.0.0";
const XHTML: &str = "http://www.w3.org/1999/xhtml";

#[derive(Debug, Clone)]
pub struct SuggestionsError {
    pub code: &'static str,
    pub message: String,
    pub source: SourceMapStack,
}
impl std::fmt::Display for SuggestionsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for SuggestionsError {}
fn error(
    code: &'static str,
    message: impl Into<String>,
    source: Option<&Item>,
) -> SuggestionsError {
    SuggestionsError {
        code,
        message: message.into(),
        source: source.and_then(Item::source_map).unwrap_or_default(),
    }
}
fn invalid(message: impl Into<String>, source: &Item) -> SuggestionsError {
    error("cem.suggestions.source_invalid", message, Some(source))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default, deny_unknown_fields)]
pub struct SuggestionsConfig {
    pub query: String,
    pub query_revision: u64,
    pub filter: String,
    pub filter_by: Option<String>,
    pub expanded: bool,
    pub active: Option<usize>,
    pub committed: Option<usize>,
}
impl Default for SuggestionsConfig {
    fn default() -> Self {
        Self {
            query: String::new(),
            query_revision: 0,
            filter: "contains".into(),
            filter_by: None,
            expanded: false,
            active: None,
            committed: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Canonical,
    Data,
    Html,
}
#[derive(Debug)]
struct Row {
    handle: String,
    source: Item,
    content: Vec<Item>,
    label: String,
    value: String,
    label_key: String,
    value_key: String,
    disabled: bool,
    hidden: bool,
    group: Option<usize>,
}
#[derive(Debug)]
struct Group {
    source: Item,
    label: String,
    disabled: bool,
    hidden: bool,
    rows: Vec<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Node {
    Root,
    Group(usize),
    Row(usize),
}
#[derive(Debug)]
pub struct SuggestionsPlan {
    rows: Vec<Row>,
    groups: Vec<Group>,
    roots: Vec<Node>,
    limits: CemValueArtifactLimits,
    warnings: Vec<SuggestionsError>,
}

struct Budget {
    work: usize,
    bytes: usize,
    limits: CemValueArtifactLimits,
}
impl Budget {
    fn visit(&mut self, node: &Item, depth: usize) -> Result<(), SuggestionsError> {
        self.work = self
            .work
            .checked_add(1)
            .ok_or_else(|| error("cem.suggestions.limit", "Work limit exceeded", Some(node)))?;
        if self.work > self.limits.max_values || depth > self.limits.max_depth {
            return Err(error(
                "cem.suggestions.limit",
                "Source work/depth limit exceeded",
                Some(node),
            ));
        }
        Ok(())
    }
    fn text(&mut self, node: &Item, bytes: usize) -> Result<(), SuggestionsError> {
        self.bytes = self.bytes.checked_add(bytes).ok_or_else(|| {
            error(
                "cem.suggestions.limit",
                "Text byte limit exceeded",
                Some(node),
            )
        })?;
        if self.bytes > self.limits.max_bytes {
            return Err(error(
                "cem.suggestions.limit",
                "Text byte limit exceeded",
                Some(node),
            ));
        }
        Ok(())
    }
}
fn lexical(node: &Item, name: &str) -> String {
    node.view()
        .and_then(|v| v.field(name))
        .and_then(|v| v.first().and_then(Item::atom))
        .map(|v| crate::render::item_to_string(&Item::Atomic(v)))
        .unwrap_or_default()
}
fn axis(node: &Item, attributes: bool, budget: &Budget) -> Result<Vec<Item>, SuggestionsError> {
    let view = node
        .view()
        .filter(|v| v.kind() == QueryItemViewKind::Node)
        .ok_or_else(|| invalid("A native node is required", node))?;
    let values = if attributes {
        view.attributes(QueryContextScope(0))
    } else {
        view.children(QueryContextScope(0))
    };
    let ceiling = budget.limits.max_values.saturating_sub(budget.work);
    let values = values
        .map_err(|_| {
            error(
                "cem.suggestions.source_unavailable",
                "Owning axis is unavailable or denied",
                Some(node),
            )
        })?
        .take(ceiling.saturating_add(1))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| {
            error(
                "cem.suggestions.source_unavailable",
                "Owning axis is unavailable or denied",
                Some(node),
            )
        })?;
    if values.len() > ceiling {
        return Err(error(
            "cem.suggestions.limit",
            "Owning axis work limit exceeded",
            Some(node),
        ));
    }
    Ok(values)
}
fn attributes(
    node: &Item,
    budget: &mut Budget,
) -> Result<BTreeMap<String, String>, SuggestionsError> {
    let mut result = BTreeMap::new();
    for attribute in axis(node, true, budget)? {
        budget.visit(&attribute, 0)?;
        if !lexical(&attribute, "namespace").is_empty() {
            continue;
        }
        let name = lexical(&attribute, "name");
        if !["value", "label", "disabled", "hidden", "selected", "alt"].contains(&name.as_str()) {
            continue;
        }
        if matches!(name.as_str(), "disabled" | "hidden" | "selected") {
            if result.insert(name, String::new()).is_some() {
                return Err(invalid("Repeated adapter attribute", &attribute));
            }
            continue;
        }
        let view = attribute.view().unwrap();
        let native = view.field("valueNodes").or_else(|| view.field("values"));
        if native.as_ref().is_some_and(|v| {
            v.iter().any(|n| {
                n.atom().is_none() && !matches!(lexical(n, "kind").as_str(), "text" | "cdata")
            })
        }) {
            return Err(invalid(
                "Adapter attributes must be materialized scalar text",
                &attribute,
            ));
        }
        let value = lexical(&attribute, "value");
        budget.text(&attribute, value.len())?;
        if result.insert(name, value).is_some() {
            return Err(invalid("Repeated adapter attribute", &attribute));
        }
    }
    Ok(result)
}
fn source_identity(node: &Item) -> String {
    if let Some(n) = crate::eval::retained_cem_node(node) {
        format!("{:p}:{}", Arc::as_ptr(n.owner().ast_owner()), n.node_id())
    } else {
        node.view()
            .map(|v| format!("{}:{}", v.representation_id(), v.identity()))
            .unwrap_or_default()
    }
}
fn family(node: &Item) -> Result<(Family, bool), SuggestionsError> {
    if lexical(node, "kind") != "element" {
        return Err(invalid("Expected an option or group element", node));
    }
    let namespace = lexical(node, "namespace");
    if !namespace.is_empty() && namespace != XHTML {
        return Err(invalid("Unsupported option/group namespace", node));
    }
    match lexical(node, "name").as_str() {
        "cem-option" => Ok((Family::Canonical, false)),
        "cem-option-group" => Ok((Family::Canonical, true)),
        "data" => Ok((Family::Data, false)),
        "option" => Ok((Family::Html, false)),
        "optgroup" => Ok((Family::Html, true)),
        _ => Err(invalid("Unsupported suggestion source family", node)),
    }
}
fn ascii_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{c}' | '\r' | ' ')
}
fn collapse(text: &str) -> String {
    text.split(ascii_space)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
fn folded(text: &str) -> String {
    static MAPPINGS: OnceLock<BTreeMap<char, String>> = OnceLock::new();
    let mappings = MAPPINGS.get_or_init(|| {
        include_str!("CaseFolding-17.0.0.txt")
            .lines()
            .filter_map(|line| {
                let fields = line
                    .split('#')
                    .next()?
                    .split(';')
                    .map(str::trim)
                    .collect::<Vec<_>>();
                if fields.len() < 3 || !matches!(fields[1], "C" | "F") {
                    return None;
                }
                let from = char::from_u32(u32::from_str_radix(fields[0], 16).ok()?)?;
                let to = fields[2]
                    .split_whitespace()
                    .map(|n| char::from_u32(u32::from_str_radix(n, 16).unwrap()).unwrap())
                    .collect();
                Some((from, to))
            })
            .collect()
    });
    let mut result = String::new();
    for c in collapse(text).chars() {
        if let Some(to) = mappings.get(&c) {
            result.push_str(to);
        } else {
            result.push(c);
        }
    }
    result
}
// Traverse owning edges only. References remain content handles, never text
// evaluators. Element boundaries insert no spaces; image alt is display-only.
fn option_text(
    content: &[Item],
    budget: &mut Budget,
    initial_depth: usize,
) -> Result<(String, String), SuggestionsError> {
    let mut stored = String::new();
    let mut display = String::new();
    let mut pending = content
        .iter()
        .rev()
        .map(|n| (n.clone(), initial_depth))
        .collect::<Vec<_>>();
    while let Some((node, depth)) = pending.pop() {
        budget.visit(&node, depth)?;
        match lexical(&node, "kind").as_str() {
            "text" | "cdata" | "whitespace" | "raw-text" => {
                let text = lexical(&node, "value");
                budget.text(&node, text.len())?;
                stored.push_str(&text);
                display.push_str(&text);
            }
            "element" => {
                let html = matches!(lexical(&node, "namespace").as_str(), "" | XHTML);
                if html && lexical(&node, "name") == "script" {
                    continue;
                }
                if html && lexical(&node, "name") == "img" {
                    if let Some(alt) = attributes(&node, budget)?.get("alt") {
                        display.push_str(alt);
                    }
                }
                for child in axis(&node, false, budget)?.into_iter().rev() {
                    pending.push((child, depth + 1));
                }
            }
            "comment" | "reference" | "processing-instruction" => {}
            _ => return Err(invalid("Unsupported option text node", &node)),
        }
    }
    Ok((collapse(&stored), collapse(&display)))
}
fn formatting(node: &Item) -> bool {
    lexical(node, "kind") == "comment"
        || (matches!(
            lexical(node, "kind").as_str(),
            "text" | "cdata" | "whitespace"
        ) && lexical(node, "value").chars().all(ascii_space))
}
impl SuggestionsPlan {
    pub fn prepare(
        values: &ItemStream,
        limits: CemValueArtifactLimits,
    ) -> Result<Arc<Self>, SuggestionsError> {
        if values.error.is_some() {
            return Err(error(
                "cem.suggestions.source_unavailable",
                "Source evaluation is incomplete",
                None,
            ));
        }
        if limits.max_values == 0 || limits.max_bytes == 0 || limits.max_depth == 0 {
            return Err(error(
                "cem.suggestions.limit",
                "Positive native bounds are required",
                None,
            ));
        }
        let mut plan = Self {
            rows: vec![],
            groups: vec![],
            roots: vec![],
            limits: limits.clone(),
            warnings: vec![],
        };
        let mut budget = Budget {
            work: 0,
            bytes: 0,
            limits,
        };
        let mut seen = BTreeSet::new();
        let mut selected_family = None;
        for source in &values.items {
            budget.visit(source, 0)?;
            let (kind, group) = family(source)?;
            if selected_family.is_some_and(|f| f != kind) {
                return Err(invalid(
                    "A source revision must use one option family",
                    source,
                ));
            }
            selected_family = Some(kind);
            if !seen.insert(source_identity(source)) {
                return Err(invalid("Repeated original source identity", source));
            }
            if group {
                let attrs = attributes(source, &mut budget)?;
                let label = attrs
                    .get("label")
                    .filter(|s| !collapse(s).is_empty())
                    .ok_or_else(|| invalid("Groups require a nonempty label", source))?
                    .clone();
                let index = plan.groups.len();
                plan.groups.push(Group {
                    source: source.clone(),
                    label,
                    disabled: attrs.contains_key("disabled"),
                    hidden: attrs.contains_key("hidden"),
                    rows: vec![],
                });
                plan.roots.push(Node::Group(index));
                for child in axis(source, false, &budget)? {
                    if formatting(&child) {
                        budget.visit(&child, 1)?;
                        continue;
                    }
                    budget.visit(&child, 1)?;
                    if family(&child)? != (kind, false) {
                        return Err(invalid(
                            "Groups accept only direct options of the same family",
                            &child,
                        ));
                    }
                    if !seen.insert(source_identity(&child)) {
                        return Err(invalid("Repeated original source identity", &child));
                    }
                    let row = plan.add_row(child, Some(index), kind, &mut budget)?;
                    plan.groups[index].rows.push(row);
                }
            } else {
                let row = plan.add_row(source.clone(), None, kind, &mut budget)?;
                plan.roots.push(Node::Row(row));
            }
        }
        Ok(Arc::new(plan))
    }
    fn add_row(
        &mut self,
        source: Item,
        group: Option<usize>,
        family: Family,
        budget: &mut Budget,
    ) -> Result<usize, SuggestionsError> {
        let attrs = attributes(&source, budget)?;
        let content = axis(&source, false, budget)?;
        let (stored, display) = option_text(&content, budget, if group.is_some() { 2 } else { 1 })?;
        let value = match attrs.get("value") {
            Some(value) => value.clone(),
            None if family == Family::Html => stored,
            None => {
                return Err(invalid(
                    "Canonical/data options require explicit value",
                    &source,
                ))
            }
        };
        let label = attrs
            .get("label")
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or(display);
        if collapse(&label).is_empty() {
            return Err(invalid("Options require a nonempty display label", &source));
        }
        let label_key = folded(&label);
        let value_key = folded(&value);
        budget.text(
            &source,
            label.len() + value.len() + label_key.len() + value_key.len() + 96,
        )?;
        let disabled =
            attrs.contains_key("disabled") || group.is_some_and(|i| self.groups[i].disabled);
        let hidden = attrs.contains_key("hidden") || group.is_some_and(|i| self.groups[i].hidden);
        if attrs.contains_key("selected") {
            self.warnings.push(error(
                "cem.suggestions.selected_ignored",
                "Source selected does not initialize the editor or preview",
                Some(&source),
            ));
        }
        let index = self.rows.len();
        self.rows.push(Row {
            handle: cem_ml::content_cache::ContentHash::from_blake3(
                source_identity(&source).as_bytes(),
            )
            .header_value(),
            source,
            content,
            label,
            value,
            label_key,
            value_key,
            disabled,
            hidden,
            group,
        });
        Ok(index)
    }
    pub fn len(&self) -> usize {
        self.rows.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }
    pub fn warnings(&self) -> &[SuggestionsError] {
        &self.warnings
    }
    pub fn view(
        self: &Arc<Self>,
        config: &SuggestionsConfig,
    ) -> Result<SuggestionsView, SuggestionsError> {
        let failure = |message| error("cem.suggestions.configuration", message, None);
        if !["contains", "prefix", "external", "none"].contains(&config.filter.as_str()) {
            return Err(failure("Unknown filter mode"));
        }
        if config.query_revision > 9_007_199_254_740_991 {
            return Err(failure("Invalid query revision"));
        }
        if config.query.len() > self.limits.max_bytes {
            return Err(error(
                "cem.suggestions.limit",
                "Query byte limit exceeded",
                None,
            ));
        }
        let keys = config
            .filter_by
            .as_deref()
            .unwrap_or("label")
            .split(ascii_space)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if keys.is_empty()
            || keys.iter().any(|s| !matches!(*s, "label" | "value"))
            || keys.iter().collect::<BTreeSet<_>>().len() != keys.len()
        {
            return Err(failure("Invalid filter-by tokens"));
        }
        if matches!(config.filter.as_str(), "external" | "none") && config.filter_by.is_some() {
            return Err(failure("filter-by conflicts with external/none"));
        }
        let query = folded(&config.query);
        if query.len() > self.limits.max_bytes {
            return Err(error(
                "cem.suggestions.limit",
                "Folded query byte limit exceeded",
                None,
            ));
        }
        let matches = |key: &str| {
            if config.filter == "prefix" {
                key.starts_with(&query)
            } else {
                key.contains(&query)
            }
        };
        let matched = self
            .rows
            .iter()
            .map(|r| {
                matches!(config.filter.as_str(), "external" | "none")
                    || keys.iter().any(|key| {
                        matches(if *key == "label" {
                            &r.label_key
                        } else {
                            &r.value_key
                        })
                    })
            })
            .collect::<Vec<_>>();
        let eligible = |i: usize| {
            self.rows
                .get(i)
                .is_some_and(|r| matched[i] && !r.hidden && !r.disabled)
        };
        let eligible_count = (0..self.rows.len()).filter(|i| eligible(*i)).count();
        if config.expanded && eligible_count == 0
            || config
                .active
                .is_some_and(|i| !config.expanded || !eligible(i))
        {
            return Err(failure(
                "Expanded/active state requires an eligible current row",
            ));
        }
        if config
            .committed
            .is_some_and(|i| self.rows.get(i).is_none_or(|r| r.hidden || r.disabled))
        {
            return Err(failure("Committed row is unavailable"));
        }
        Ok(SuggestionsView(Arc::new(View {
            plan: self.clone(),
            config: config.clone(),
            matched,
            eligible_count,
        })))
    }
}

#[derive(Debug)]
struct View {
    plan: Arc<SuggestionsPlan>,
    config: SuggestionsConfig,
    matched: Vec<bool>,
    eligible_count: usize,
}
/// Prepared scalar commit controls. Original source/content stay native.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SuggestionRowControl {
    pub handle: String,
    pub value: String,
    pub eligible: bool,
    pub available: bool,
}
#[derive(Debug, Clone)]
pub struct SuggestionsView(Arc<View>);
impl SuggestionsView {
    pub fn row_controls(&self) -> Vec<SuggestionRowControl> {
        self.0
            .plan
            .rows
            .iter()
            .enumerate()
            .map(|(index, row)| SuggestionRowControl {
                handle: row.handle.clone(),
                value: row.value.clone(),
                eligible: self.0.matched[index] && !row.disabled && !row.hidden,
                available: !row.disabled && !row.hidden,
            })
            .collect()
    }
    pub fn row_control(&self, handle: &str) -> Result<SuggestionRowControl, SuggestionsError> {
        let index = self
            .0
            .plan
            .rows
            .iter()
            .position(|row| row.handle == handle)
            .ok_or_else(|| {
                error(
                    "cem.suggestions.configuration",
                    "Unknown admitted row handle",
                    None,
                )
            })?;
        let row = &self.0.plan.rows[index];
        Ok(SuggestionRowControl {
            handle: row.handle.clone(),
            value: row.value.clone(),
            eligible: self.0.matched[index] && !row.disabled && !row.hidden,
            available: !row.disabled && !row.hidden,
        })
    }
    pub fn root(&self) -> Item {
        self.node(Node::Root)
    }
    pub fn row(&self, index: usize) -> Result<Item, SuggestionsError> {
        if index >= self.0.plan.rows.len() {
            return Err(error(
                "cem.suggestions.configuration",
                "Unknown native row handle",
                None,
            ));
        }
        Ok(self.node(Node::Row(index)))
    }
    pub fn group(&self, index: usize) -> Result<Item, SuggestionsError> {
        if index >= self.0.plan.groups.len() {
            return Err(error(
                "cem.suggestions.configuration",
                "Unknown native group handle",
                None,
            ));
        }
        Ok(self.node(Node::Group(index)))
    }
    fn node(&self, node: Node) -> Item {
        Item::native(SuggestionNode {
            view: self.0.clone(),
            node,
            attribute: None,
        })
    }
    pub(crate) fn owns(&self, item: &Item) -> bool {
        item.view()
            .and_then(|v| v.downcast_ref::<SuggestionNode>())
            .is_some_and(|n| Arc::ptr_eq(&self.0, &n.view))
    }
}
#[derive(Debug, Clone)]
struct SuggestionNode {
    view: Arc<View>,
    node: Node,
    attribute: Option<usize>,
}
impl SuggestionNode {
    fn item(&self, node: Node) -> Item {
        Item::native(Self {
            view: self.view.clone(),
            node,
            attribute: None,
        })
    }
    fn source(&self) -> Option<&Item> {
        match self.node {
            Node::Root => None,
            Node::Group(i) => Some(&self.view.plan.groups[i].source),
            Node::Row(i) => Some(&self.view.plan.rows[i].source),
        }
    }
    fn hidden(&self, i: usize) -> bool {
        self.view.plan.rows[i].hidden || !self.view.matched[i]
    }
    fn attrs(&self) -> Vec<(&'static str, AtomValue)> {
        let text = |s: &str| AtomValue::String(s.into());
        let boolean = AtomValue::Boolean;
        match self.node {
            Node::Root => vec![
                ("query", text(&self.view.config.query)),
                (
                    "query-revision",
                    AtomValue::Integer(self.view.config.query_revision as i64),
                ),
                ("expanded", boolean(self.view.config.expanded)),
                (
                    "eligible-count",
                    AtomValue::Integer(self.view.eligible_count as i64),
                ),
                ("unicode-version", text(UNICODE_VERSION)),
            ],
            Node::Group(i) => {
                let g = &self.view.plan.groups[i];
                vec![
                    ("label", text(&g.label)),
                    ("disabled", boolean(g.disabled)),
                    (
                        "hidden",
                        boolean(g.hidden || g.rows.iter().all(|i| self.hidden(*i))),
                    ),
                ]
            }
            Node::Row(i) => {
                let r = &self.view.plan.rows[i];
                vec![
                    ("label", text(&r.label)),
                    ("value", text(&r.value)),
                    ("matched", boolean(self.view.matched[i])),
                    ("disabled", boolean(r.disabled)),
                    ("hidden", boolean(self.hidden(i))),
                    ("active", boolean(self.view.config.active == Some(i))),
                    ("committed", boolean(self.view.config.committed == Some(i))),
                ]
            }
        }
    }
    fn children_vec(&self) -> Vec<Item> {
        if self.attribute.is_some() {
            return vec![];
        }
        match self.node {
            Node::Root => self.view.plan.roots.iter().map(|n| self.item(*n)).collect(),
            Node::Group(i) => self.view.plan.groups[i]
                .rows
                .iter()
                .map(|i| self.item(Node::Row(*i)))
                .collect(),
            Node::Row(_) => vec![],
        }
    }
}
impl QueryItemView for SuggestionNode {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        CONSUMER_IDENTITY
    }
    fn identity(&self) -> String {
        format!(
            "cem:suggestions:{:p}:{:?}:{:?}",
            Arc::as_ptr(&self.view.plan),
            self.node,
            self.attribute
        )
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Node
    }
    fn source_map(&self) -> Option<SourceMapStack> {
        self.source().and_then(Item::source_map)
    }
    fn parent(&self, _: QueryContextScope) -> Result<Option<Item>, QueryNodeAccessError> {
        Ok(if self.attribute.is_some() {
            Some(self.item(self.node))
        } else {
            match self.node {
                Node::Root => None,
                Node::Group(_) => Some(self.item(Node::Root)),
                Node::Row(i) => Some(
                    self.item(
                        self.view.plan.rows[i]
                            .group
                            .map(Node::Group)
                            .unwrap_or(Node::Root),
                    ),
                ),
            }
        })
    }
    fn children(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(self.children_vec().into_iter().map(Ok)))
    }
    fn attributes(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeIterator<'_>, QueryNodeAccessError> {
        let values = if self.attribute.is_some() {
            vec![]
        } else {
            (0..self.attrs().len())
                .map(|i| {
                    Item::native(Self {
                        attribute: Some(i),
                        ..self.clone()
                    })
                })
                .collect()
        };
        Ok(Box::new(values.into_iter().map(Ok)))
    }
    fn text_fragments(
        &self,
        _: QueryContextScope,
    ) -> Result<QueryNodeTextIterator<'_>, QueryNodeAccessError> {
        Ok(Box::new(std::iter::empty()))
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        let text = |s: &str| vec![Item::Atomic(AtomValue::String(s.into()))];
        if name == "id" {
            return Some(text(&self.identity()));
        }
        if let Some(index) = self.attribute {
            let (key, value) = self.attrs().into_iter().nth(index)?;
            return match name {
                "kind" => Some(text("attribute")),
                "name" => Some(text(key)),
                "namespace" => Some(text("")),
                "value" | "values" => Some(vec![Item::Atomic(value)]),
                "parent" => Some(vec![self.item(self.node)]),
                _ => None,
            };
        }
        match name {
            "kind" => Some(text("element")),
            "name" => Some(text(match self.node {
                Node::Root => "suggestions",
                Node::Group(_) => "group",
                Node::Row(_) => "option",
            })),
            "namespace" => Some(text(VIEW_NAMESPACE)),
            "source" => Some(self.source().cloned().into_iter().collect()),
            "content" => Some(if let Node::Row(i) = self.node {
                self.view.plan.rows[i].content.clone()
            } else {
                vec![]
            }),
            "children" => Some(self.children_vec()),
            "attributes" => Some(
                self.attributes(QueryContextScope(0))
                    .ok()?
                    .map(Result::unwrap)
                    .collect(),
            ),
            "parent" => Some(
                self.parent(QueryContextScope(0))
                    .ok()?
                    .into_iter()
                    .collect(),
            ),
            _ => None,
        }
    }
}
