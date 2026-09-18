//! Bounded reader retention and compatibility XPath reports over common CEM trees.
use super::string;
use crate::{
    eval::{Item, QueryItemView, QueryItemViewKind},
    xpath::functions::XPathQueryItem,
};
use cem_ml::{parser::tree::RetainedCemTree, validation::xpath::XPathNativeNode};
use std::{
    any::Any,
    collections::VecDeque,
    sync::{Arc, Mutex},
};

const MAX_READERS: usize = 16;

/// Runtime-only reader retention. Clones share a bounded LRU; defaults are
/// isolated. A host can reuse it across evaluations/renders, then clear or drop
/// it on disposal. Returned nodes independently retain their original owners.
/// Successful imports are cached by exact source, format and projection.
/// At most 16 bounded imported trees are retained across all formats.
#[derive(Debug, Clone, Default)]
pub struct DataReaderCache(Arc<Mutex<VecDeque<ReaderEntry>>>);

#[derive(Debug)]
struct ReaderEntry {
    source: String,
    format: String,
    projection: String,
    report: Item,
}

impl DataReaderCache {
    pub fn clear(&self) {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    pub(crate) fn read(&self, source: &str, format: &str, projection: &str) -> Item {
        // Serializing cache misses also prevents concurrent reads of the same
        // source from manufacturing different owners. Parsing is bounded.
        let mut cache = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(index) = cache
            .iter()
            .position(|e| e.source == source && e.format == format && e.projection == projection)
        {
            let entry = cache.remove(index).expect("existing reader entry");
            let report = entry.report.clone();
            cache.push_back(entry);
            return report;
        }
        let report = super::read(source, format, projection);
        let successful = report
            .view()
            .and_then(|v| v.field("root"))
            .is_some_and(|items| !items.is_empty());
        if !successful {
            return report;
        }
        if cache.len() == MAX_READERS {
            cache.pop_front();
        }
        cache.push_back(ReaderEntry {
            source: source.into(),
            format: format.into(),
            projection: projection.into(),
            report: report.clone(),
        });
        report
    }
}

#[derive(Debug)]
struct XPathReadReport {
    root: Option<Item>,
    error: String,
}

pub(super) fn report(tree: Option<Arc<RetainedCemTree>>, error: String) -> Item {
    Item::native(XPathReadReport {
        root: tree.map(|tree| XPathQueryItem::from_node(XPathNativeNode::cem_document(tree))),
        error,
    })
}

impl QueryItemView for XPathReadReport {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn representation_id(&self) -> &'static str {
        "cem.ql.xpath-read-report/1"
    }
    fn identity(&self) -> String {
        self.root
            .as_ref()
            .and_then(Item::view)
            .map(|v| v.identity())
            .unwrap_or_else(|| format!("xpath-read-error:{}", self.error))
    }
    fn kind(&self) -> QueryItemViewKind {
        QueryItemViewKind::Record
    }
    fn field(&self, name: &str) -> Option<Vec<Item>> {
        match name {
            "root" => Some(self.root.iter().cloned().collect()),
            "error" => Some(string(&self.error)),
            _ => None,
        }
    }
}
