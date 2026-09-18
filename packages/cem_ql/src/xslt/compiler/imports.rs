//! Link preflighted authoring sources; no document import or runtime I/O.
use super::*;

pub(super) fn diagnostic(uri: &str, message: &str) -> Vec<Diagnostic> {
    vec![Diagnostic {
        uri: Some(uri.into()),
        code: "cem.xslt.compile_import".into(),
        severity: Severity::Error,
        message: message.into(),
        ..Default::default()
    }]
}
pub(super) struct Declaration<'a> {
    pub source: usize,
    pub precedence: usize,
    pub node: AuthorNode<'a>,
}

pub(super) fn link<'a>(
    compiler: &mut Compiler<'a>,
    roots: &[AuthorNode<'a>],
    modules: &[XsltModuleSource],
    manifests: &mut [StylesheetSource],
) -> CompileResult<Vec<Declaration<'a>>> {
    let indices: BTreeMap<_, _> = manifests
        .iter()
        .enumerate()
        .map(|(i, s)| (s.source.uri.clone(), i))
        .collect();
    let mut edges = BTreeMap::new();
    for module in modules {
        if !indices.contains_key(&module.parent_uri)
            || edges
                .insert(
                    (module.parent_uri.clone(), module.href.clone()),
                    indices[&module.uri],
                )
                .is_some()
        {
            return Err(diagnostic(
                &module.uri,
                "duplicate or unresolved preflighted import edge",
            ));
        }
    }
    let mut linked = BTreeMap::new();
    let mut consumed = BTreeSet::new();
    for (source, root) in roots.iter().enumerate() {
        compiler.select_source(source);
        for node in &root.children {
            if node.event.namespace_uri.as_deref() != Some(XSLT_NAMESPACE_URI) {
                continue;
            }
            let kind = match node.event.local_name.as_deref() {
                Some("import") => DependencyKind::Import,
                Some("include") => DependencyKind::Include,
                _ => continue,
            };
            compiler.attributes(node.event, &["href"])?;
            compiler.empty(node)?;
            let key = (
                manifests[source].source.uri.clone(),
                compiler.required(node.event, "href")?.to_owned(),
            );
            let target = *edges.get(&key).ok_or_else(|| {
                compiler.error(
                    node.event,
                    "cem.xslt.compile_import",
                    "stylesheet dependency was not preflighted",
                )
            })?;
            consumed.insert(key);
            manifests[source]
                .dependencies
                .push(StylesheetDependency { kind, target });
            linked.insert((source, node.event.index), target);
        }
    }
    if consumed.len() != edges.len() {
        return Err(diagnostic(
            &manifests[0].source.uri,
            "unused preflighted import edge",
        ));
    }
    struct Linker<'a, 'b> {
        roots: &'b [AuthorNode<'a>],
        linked: &'b BTreeMap<(usize, usize), usize>,
        active: BTreeSet<usize>,
        reached: BTreeSet<usize>,
        precedence: usize,
        output: Vec<Declaration<'a>>,
        expansions: usize,
    }
    impl<'a> Linker<'a, '_> {
        fn expand(
            &mut self,
            source: usize,
        ) -> std::result::Result<Vec<(usize, AuthorNode<'a>)>, &'static str> {
            if self.active.len() >= 32 || self.expansions >= 1024 {
                return Err("stylesheet dependency expansion limit exceeded");
            }
            self.expansions += 1;
            if !self.active.insert(source) {
                return Err("cyclic stylesheet dependency");
            }
            self.reached.insert(source);
            let mut expanded = Vec::new();
            for node in self.roots[source]
                .children
                .iter()
                .filter(|node| !ignorable(node))
            {
                if node.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                    && node.event.local_name.as_deref() == Some("include")
                {
                    expanded.extend(self.expand(self.linked[&(source, node.event.index)])?);
                } else {
                    expanded.push((source, node.clone()));
                }
                if expanded.len() > 128 {
                    return Err("stylesheet declaration limit exceeded");
                }
            }
            self.active.remove(&source);
            Ok(expanded)
        }
        fn module(
            &mut self,
            source: usize,
            ancestry: &mut BTreeSet<usize>,
        ) -> std::result::Result<(), &'static str> {
            if ancestry.len() >= 32 || !ancestry.insert(source) {
                return Err("cyclic or excessive stylesheet import nesting");
            }
            let expanded = self.expand(source)?;
            for (owner, node) in &expanded {
                if node.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                    && node.event.local_name.as_deref() == Some("import")
                {
                    self.module(self.linked[&(*owner, node.event.index)], ancestry)?;
                }
            }
            self.precedence += 1;
            for (owner, node) in expanded {
                if node.event.namespace_uri.as_deref() == Some(XSLT_NAMESPACE_URI)
                    && node.event.local_name.as_deref() == Some("import")
                {
                    continue;
                }
                self.output.push(Declaration {
                    source: owner,
                    precedence: self.precedence,
                    node,
                });
                if self.output.len() > 128 {
                    return Err("stylesheet declaration limit exceeded");
                }
            }
            ancestry.remove(&source);
            Ok(())
        }
    }
    let mut linker = Linker {
        roots,
        linked: &linked,
        active: BTreeSet::new(),
        reached: BTreeSet::new(),
        precedence: 0,
        output: Vec::new(),
        expansions: 0,
    };
    linker
        .module(0, &mut BTreeSet::new())
        .map_err(|message| diagnostic(&manifests[0].source.uri, message))?;
    if linker.reached.len() != roots.len() {
        return Err(diagnostic(
            &manifests[0].source.uri,
            "unreachable stylesheet source",
        ));
    }
    Ok(linker.output)
}
