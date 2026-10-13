//! Immutable child contexts retain the native resolver's ordered ancestor rules.
use super::*;

impl CemScopedModuleUrlResolver {
    pub(super) fn derive_native_child(
        &self,
        parent: &CemResolutionContextHandle,
        frame: CemModuleUrlFrame,
    ) -> Result<CemModuleUrlResolutionCapability, String> {
        let mut context = self
            .contexts
            .get(parent)
            .cloned()
            .ok_or("unknown parent context")?;
        if frame.frame_id.trim().is_empty()
            || context.frames.iter().any(|f| f.frame_id == frame.frame_id)
            || context.frames.len() >= 256
        {
            return Err("unique nonempty child frame and bounded context required".into());
        }
        parse_base_url(&frame.base_url)?;
        let map_base = parse_base_url(
            frame
                .module_map_base_url
                .as_deref()
                .unwrap_or(&frame.base_url),
        )?;
        for scope in &frame.scopes {
            normalize_map_url(&scope.prefix, &map_base)?;
        }
        context.identity = format!("{}::{}", context.identity, frame.frame_id);
        let handle = CemResolutionContextHandle::new(context.identity.clone());
        if self.contexts.contains_key(&handle) {
            return Err("derived context handle already exists".into());
        }
        context.frames.push(frame);
        let mut resolver = self.clone();
        resolver.insert_child_context(handle.clone(), parent.clone(), context);
        Ok(CemModuleUrlResolutionCapability::new(
            Arc::new(resolver),
            handle,
        ))
    }
}
