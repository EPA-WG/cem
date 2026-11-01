//! Explicit real native owners held by one installed application session.
use super::*;

pub enum RetainedApplicationOwner {
    Rule(Arc<FreshDatatypeRuleBinding>),
    Operation(Arc<FreshNativeOperation>),
    Tokenizer(Arc<FreshNativeTokenizer>),
    QuerySource(Arc<FreshQuerySource>),
    QueryCode(Arc<FreshQueryCodeOwner>),
    QueryRegistry(Arc<FreshQueryRegistryOwner>),
    QueryProfile(Arc<FreshQueryResultProfileOwner>),
    NativeRuleCode(Arc<FreshNativeRuleCodeOwner>),
}
impl RetainedApplicationOwner {
    pub(crate) fn is_live_on(&self, host: &FreshStructuralHost) -> bool {
        match self {
            Self::Rule(o) => o.is_live(host),
            Self::Operation(o) => o.is_live_on(host),
            Self::Tokenizer(o) => o.is_live_on(host),
            Self::QuerySource(o) => o.live(host),
            Self::QueryCode(o) => o.is_live_on(host),
            Self::QueryRegistry(o) => o.is_live_on(host),
            Self::QueryProfile(o) => o.is_live_on(host),
            Self::NativeRuleCode(o) => o.is_live_on(host),
        }
    }
}
