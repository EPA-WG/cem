//! Name only source-map metadata reached through an original native document
//! view. Formatter-generated values and ordinary JSON data are outside this view.
use super::*;

impl<'a> CemtEvaluatorValueRef<'a> {
    pub(crate) fn with_native_source_uri(self, source_uri: &'a str) -> Self {
        if source_uri.is_empty() {
            return self;
        }
        match self {
            Self::Record(value @ CemtEvaluatorRecordRef::NativeSource { .. }) => {
                Self::Record(value)
            }
            Self::Sequence(value @ CemtEvaluatorSequenceRef::NativeSource { .. }) => {
                Self::Sequence(value)
            }
            Self::Record(value) => Self::Record(CemtEvaluatorRecordRef::NativeSource {
                value: Box::new(value),
                source_uri,
            }),
            Self::Sequence(value) => Self::Sequence(CemtEvaluatorSequenceRef::NativeSource {
                value: Box::new(value),
                source_uri,
            }),
            Self::SourceMap(value) => {
                Self::OwnedSourceMap(Arc::new(value.clone().with_source_uri(source_uri)))
            }
            Self::OwnedSourceMap(value) => {
                Self::OwnedSourceMap(Arc::new((*value).clone().with_source_uri(source_uri)))
            }
            value => value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        source::{ByteRange, SourceId},
        source_map::{FrameSpan, SourceMapFrame, TransformKind},
    };

    #[test]
    fn naming_native_views_preserves_foreign_boundaries_and_generated_values() {
        let source = SourceMapStack {
            frames: vec![SourceMapFrame {
                source_id: SourceId(1),
                span: FrameSpan::Single(ByteRange::new(3, 5)),
                transform: TransformKind::CemAstBuilder,
            }],
        };
        let named =
            CemtEvaluatorValueRef::SourceMap(&source).with_native_source_uri("studio://original");
        let CemtEvaluatorValueRef::OwnedSourceMap(map) = named else {
            panic!("native source map");
        };
        assert!(
            matches!(&map.current().unwrap().transform, TransformKind::SourceBoundary { uri } if uri == "studio://original")
        );
        assert_eq!(map.current().unwrap().span, source.current().unwrap().span);
        assert_eq!(
            source.frames.len(),
            1,
            "the borrowed native owner is unchanged"
        );
        let foreign = source.clone().with_source_uri("module://foreign");
        let CemtEvaluatorValueRef::OwnedSourceMap(retained) =
            CemtEvaluatorValueRef::SourceMap(&foreign).with_native_source_uri("studio://original")
        else {
            panic!("native source map");
        };
        assert_eq!(
            *retained, foreign,
            "a module origin must not be rebound to the input"
        );
        let mut scss = source.clone();
        scss.frames[0].transform = TransformKind::ScssOrigin {
            origin_kind: crate::source_map::ScssOriginKind::CallSite,
            module_uri: "module://foreign.scss".into(),
            name: None,
        };
        let CemtEvaluatorValueRef::OwnedSourceMap(retained) =
            CemtEvaluatorValueRef::SourceMap(&scss).with_native_source_uri("studio://original")
        else {
            panic!("native source map");
        };
        assert_eq!(
            *retained, scss,
            "foreign SCSS module coordinates cannot name the root input"
        );
        let mut mixed = scss.clone();
        mixed.frames.insert(
            0,
            SourceMapFrame {
                source_id: SourceId(9),
                span: FrameSpan::Single(ByteRange::new(17, 4)),
                transform: TransformKind::ScssOrigin {
                    origin_kind: crate::source_map::ScssOriginKind::CallSite,
                    module_uri: "studio://original".into(),
                    name: None,
                },
            },
        );
        let CemtEvaluatorValueRef::OwnedSourceMap(named) =
            CemtEvaluatorValueRef::SourceMap(&mixed).with_native_source_uri("studio://original")
        else {
            panic!("native source map");
        };
        assert_eq!(&named.frames[..mixed.frames.len()], &mixed.frames);
        assert_eq!(
            named.current().unwrap().span,
            mixed.frames[0].span,
            "choose only the native origin whose URI matches this owned buffer"
        );
        assert_eq!(named.current().unwrap().source_id, SourceId(9));
        let generated = SourceMapStack::default();
        let CemtEvaluatorValueRef::OwnedSourceMap(empty) =
            CemtEvaluatorValueRef::SourceMap(&generated)
                .with_native_source_uri("studio://original")
        else {
            panic!("native source map");
        };
        assert!(
            empty.frames.is_empty(),
            "generated layout gains no invented range"
        );
        assert!(matches!(
            CemtEvaluatorValueRef::String("{sourceMap:forged}")
                .with_native_source_uri("studio://original"),
            CemtEvaluatorValueRef::String(_)
        ));
    }
}
