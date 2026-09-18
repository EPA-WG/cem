//! Private v3 executable wire grammar. Explicit tags and fixed-width lengths
//! keep it independent of Rust layout, serde, JSON, and source-token spelling.
use super::*;
const MAX_DEPTH: usize = 128;
const MAX_VALUES: usize = 65_536;
const MAX_COLLECTION: usize = 4096;
type Result<T> = std::result::Result<T, XPathArtifactError>;

pub(super) struct Writer {
    bytes: Vec<u8>,
    depth: usize,
    values: usize,
}
impl Writer {
    pub(super) fn new(magic: &[u8]) -> Self {
        Self {
            bytes: magic.to_vec(),
            depth: 0,
            values: 0,
        }
    }
    pub(super) fn write<T: Wire>(&mut self, value: &T) -> Result<()> {
        if self.depth >= MAX_DEPTH || self.values >= MAX_VALUES {
            return Err(XPathArtifactError::limit());
        }
        self.depth += 1;
        self.values += 1;
        let result = value.encode(self);
        self.depth -= 1;
        result
    }
    fn append(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() > XPATH_ARTIFACT_MAX_BYTES.saturating_sub(self.bytes.len()) {
            return Err(XPathArtifactError::limit());
        }
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}
pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
    depth: usize,
    values: usize,
}
impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            depth: 0,
            values: 0,
        }
    }
    pub(super) fn take(&mut self, len: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(XPathArtifactError::limit)?;
        let result = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| XPathArtifactError::invalid("truncated XPath artifact"))?;
        self.offset = end;
        Ok(result)
    }
    pub(super) fn read<T: Wire>(&mut self) -> Result<T> {
        if self.depth >= MAX_DEPTH || self.values >= MAX_VALUES {
            return Err(XPathArtifactError::limit());
        }
        self.depth += 1;
        self.values += 1;
        let result = T::decode(self);
        self.depth -= 1;
        result
    }
    pub(super) fn finish(&self) -> Result<()> {
        if self.offset != self.bytes.len() {
            return Err(XPathArtifactError::invalid("trailing XPath artifact bytes"));
        }
        Ok(())
    }
    fn collection_len(&mut self) -> Result<usize> {
        let len = self.read::<usize>()?;
        if len > MAX_COLLECTION || len > self.bytes.len().saturating_sub(self.offset) {
            return Err(XPathArtifactError::limit());
        }
        Ok(len)
    }
}

pub(super) trait Wire: Sized {
    fn encode(&self, w: &mut Writer) -> Result<()>;
    fn decode(r: &mut Reader<'_>) -> Result<Self>;
}
macro_rules! number {
    ($ty:ty, $size:literal) => {
        impl Wire for $ty {
            fn encode(&self, w: &mut Writer) -> Result<()> {
                w.append(&self.to_le_bytes())
            }
            fn decode(r: &mut Reader<'_>) -> Result<Self> {
                let mut bytes = [0; $size];
                bytes.copy_from_slice(r.take($size)?);
                Ok(Self::from_le_bytes(bytes))
            }
        }
    };
}
number!(u8, 1);
number!(u32, 4);
number!(u64, 8);
impl Wire for usize {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        w.write(&(*self as u64))
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        usize::try_from(r.read::<u64>()?).map_err(|_| XPathArtifactError::limit())
    }
}
impl Wire for char {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        w.write(&(*self as u32))
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        char::from_u32(r.read()?)
            .ok_or_else(|| XPathArtifactError::invalid("invalid Unicode scalar"))
    }
}
impl Wire for String {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        w.write(&self.len())?;
        w.append(self.as_bytes())
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let len = r.read::<usize>()?;
        let text = std::str::from_utf8(r.take(len)?)
            .map_err(|_| XPathArtifactError::invalid("invalid UTF-8"))?;
        Ok(text.to_owned())
    }
}
impl<T: Wire> Wire for Option<T> {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        match self {
            Some(value) => {
                w.write(&1u8)?;
                w.write(value)
            }
            None => w.write(&0u8),
        }
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        match r.read::<u8>()? {
            0 => Ok(None),
            1 => Ok(Some(r.read()?)),
            _ => Err(XPathArtifactError::invalid("invalid optional value tag")),
        }
    }
}
impl<T: Wire> Wire for Box<T> {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        w.write(self.as_ref())
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        r.read().map(Box::new)
    }
}
impl<T: Wire> Wire for Vec<T> {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        if self.len() > MAX_COLLECTION {
            return Err(XPathArtifactError::limit());
        }
        w.write(&self.len())?;
        for value in self {
            w.write(value)?;
        }
        Ok(())
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let len = r.collection_len()?;
        // Grow only as validated values arrive, never reserve an attacker count.
        let mut result = Vec::new();
        for _ in 0..len {
            result.push(r.read()?);
        }
        Ok(result)
    }
}
impl<K: Wire + Ord, V: Wire> Wire for BTreeMap<K, V> {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        if self.len() > MAX_COLLECTION {
            return Err(XPathArtifactError::limit());
        }
        w.write(&self.len())?;
        for (key, value) in self {
            w.write(key)?;
            w.write(value)?;
        }
        Ok(())
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let len = r.collection_len()?;
        let mut result = Self::new();
        for _ in 0..len {
            let key = r.read()?;
            if result
                .last_key_value()
                .is_some_and(|(last, _)| last >= &key)
            {
                return Err(XPathArtifactError::invalid(
                    "unordered or duplicate XPath binding",
                ));
            }
            let value = r.read()?;
            result.insert(key, value);
        }
        Ok(result)
    }
}

macro_rules! structure {
    ($ty:ty { $($field:ident),* $(,)? }) => {
        impl Wire for $ty {
            fn encode(&self, w: &mut Writer) -> Result<()> { $(w.write(&self.$field)?;)* Ok(()) }
            fn decode(r: &mut Reader<'_>) -> Result<Self> { Ok(Self { $($field: r.read()?),* }) }
        }
    };
}
macro_rules! variants {
    ($ty:ty { $($tag:literal => $variant:ident $(($tuple:ident))? $({$($field:ident),*})? ),* $(,)? }) => {
        impl Wire for $ty {
            fn encode(&self, w: &mut Writer) -> Result<()> {
                match self { $(Self::$variant $(($tuple))? $({$($field),*})? => {
                    w.write(&($tag as u8))?;
                    $(w.write($tuple)?;)?
                    $($(w.write($field)?;)*)?
                    Ok(())
                }),* }
            }
            fn decode(r: &mut Reader<'_>) -> Result<Self> {
                match r.read::<u8>()? { $($tag => {
                    $(let $tuple = r.read()?;)?
                    $($(let $field = r.read()?;)*)?
                    Ok(Self::$variant $(($tuple))? $({$($field),*})?)
                }),*, _ => Err(XPathArtifactError::invalid(concat!("unknown ", stringify!($ty), " tag"))) }
            }
        }
    };
}

structure!(ContentHash { scheme, hex });
structure!(XPathArtifactIdentity {
    content_type,
    schema_uri,
    artifact_version,
    program_format,
    grammar_version,
    compiler_version,
    source_hash
});
structure!(XPathExpressionSource {
    uri,
    content_type,
    media_type,
    parameters,
    byte_length
});
structure!(XPathSourcePosition {
    line,
    column,
    byte_offset
});
impl Wire for XPathSourceRange {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        if self
            .start
            .byte_offset
            .checked_add(self.byte_length)
            .is_none()
        {
            return Err(XPathArtifactError::invalid("source range overflow"));
        }
        w.write(&self.start)?;
        w.write(&self.byte_length)
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        let start: XPathSourcePosition = r.read()?;
        let byte_length = r.read()?;
        if start.byte_offset.checked_add(byte_length).is_none() {
            return Err(XPathArtifactError::invalid("source range overflow"));
        }
        Ok(Self { start, byte_length })
    }
}
structure!(XPathStaticContext {
    namespaces,
    default_element_namespace,
    default_function_namespace,
    variable_bindings,
    function_bindings,
    unnamed_decimal_format,
    decimal_formats
});
structure!(XPathDecimalFormat {
    decimal_separator,
    exponent_separator,
    grouping_separator,
    infinity,
    minus_sign,
    nan,
    percent,
    per_mille,
    zero_digit,
    digit,
    pattern_separator
});
structure!(XPathExpandedName {
    namespace_uri,
    local_name
});
structure!(XPathExpectedResult {
    sequence_type,
    min_items,
    max_items
});
structure!(XPathHostOwner {
    source_id,
    source_uri,
    content_type,
    schema_uri,
    node_kind,
    node_id,
    source_range
});
structure!(XPathHostAttachment {
    owner,
    expression_range,
    static_context,
    expected_result,
    evaluation_phase,
    resolver_policy_stamp,
    safety_policy_stamp
});
variants!(XPathAttachment { 0 => Standalone { source_id }, 1 => StandaloneStaticContext { source_id, static_context }, 2 => Host(value) });
variants!(XPathHostNodeKind { 0 => XmlDocument, 1 => XmlSubtree, 2 => XmlElement, 3 => XmlAttribute, 4 => XsltAttribute, 5 => CemtExpressionSlot, 6 => CemQlExpressionSlot });
variants!(XPathEvaluationPhase { 0 => Validate, 1 => Compile, 2 => Transform, 3 => Render, 4 => Runtime });

structure!(XPathExpressionSequence {
    expressions,
    source_range
});
structure!(XPathExpressionNode {
    expression,
    source_range
});
variants!(XPathExpression {
    0 => Path(value), 1 => Unary { operator, operand }, 2 => Binary { operator, left, right },
    3 => SimpleMap { input, mappings }, 4 => CastAs { operand, single_type }, 5 => CastableAs { operand, single_type },
    6 => TreatAs { operand, sequence_type }, 7 => InstanceOf { operand, sequence_type },
    8 => For { binding, binding_expression, return_expression }, 9 => Let { binding, binding_expression, return_expression },
    10 => If { condition, then_expression, else_expression }, 11 => Quantified { quantifier, binding, binding_expression, satisfies_expression },
    12 => Unsupported { production }
});
variants!(XPathQuantifier { 0 => Some, 1 => Every });
variants!(XPathUnaryOperator { 0 => Plus, 1 => Minus });
structure!(XPathSingleType {
    type_name,
    allows_empty,
    source_range
});
impl Wire for bool {
    fn encode(&self, w: &mut Writer) -> Result<()> {
        w.write(&u8::from(*self))
    }
    fn decode(r: &mut Reader<'_>) -> Result<Self> {
        match r.read::<u8>()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(XPathArtifactError::invalid("invalid boolean")),
        }
    }
}
variants!(XPathSequenceType { 0 => Empty { source_range }, 1 => Item { item_type, occurrence, source_range } });
variants!(XPathSequenceItemType {
    0 => AnyItem { source_range }, 1 => Atomic(value), 2 => Kind { kind, lexical, source_range },
    3 => Parenthesized { item_type, source_range }, 4 => Unsupported { production, lexical, source_range }
});
variants!(XPathOccurrenceIndicator { 0 => ExactlyOne, 1 => ZeroOrOne, 2 => ZeroOrMore, 3 => OneOrMore });
structure!(XPathPathExpression {
    root,
    steps,
    source_range
});
variants!(XPathPathRoot { 0 => Relative, 1 => Rooted, 2 => RootedDescendant });
structure!(XPathStepNode { step, source_range });
variants!(XPathStep { 0 => Axis { axis, node_test, predicates }, 1 => Primary(value), 2 => Postfix { primary, postfixes } });
variants!(XPathPrimaryExpression {
    0 => Literal(value), 1 => VariableReference(value), 2 => Parenthesized(value), 3 => ContextItem,
    4 => FunctionCall { name, arguments }, 5 => MapConstructor { entries }, 6 => ArrayConstructor(value), 7 => Unsupported { production }, 8 => UnaryLookup(value),
    9 => InlineFunction { parameters, result_type, body }
});
structure!(XPathFunctionParameter { name, sequence_type });
structure!(XPathMapConstructorEntry {
    key,
    value,
    source_range
});
variants!(XPathArrayConstructor { 0 => Square(value), 1 => Curly(value) });
variants!(XPathPostfixExpression { 0 => Predicate(value), 1 => ArgumentList(value), 2 => Lookup { key } });
structure!(XPathName {
    lexical,
    prefix,
    local_name,
    namespace_uri,
    source_range
});
variants!(XPathLiteralKind { 0 => Integer, 1 => Decimal, 2 => Double, 3 => String });
structure!(XPathLiteral {
    kind,
    lexical,
    value
});
variants!(XPathAxis { 0 => Ancestor, 1 => AncestorOrSelf, 2 => Attribute, 3 => Child, 4 => Descendant,
    5 => DescendantOrSelf, 6 => Following, 7 => FollowingSibling, 8 => Namespace, 9 => Parent,
    10 => Preceding, 11 => PrecedingSibling, 12 => SelfAxis });
variants!(XPathNodeTest { 0 => Name(value), 1 => Kind { kind, lexical, processing_instruction_target } });
variants!(XPathNameTest { 0 => Name(value), 1 => Any, 2 => AnyNamespace { local_name }, 3 => Namespace { namespace_uri } });
variants!(XPathKindTest { 0 => Document, 1 => Element, 2 => Attribute, 3 => SchemaElement, 4 => SchemaAttribute,
    5 => ProcessingInstruction, 6 => Comment, 7 => Text, 8 => NamespaceNode, 9 => AnyNode });
variants!(XPathBinaryOperator {
    0 => Or, 1 => And, 2 => ValueEqual, 3 => ValueNotEqual, 4 => ValueLessThan, 5 => ValueLessThanOrEqual,
    6 => ValueGreaterThan, 7 => ValueGreaterThanOrEqual, 8 => GeneralEqual, 9 => GeneralNotEqual,
    10 => GeneralLessThan, 11 => GeneralLessThanOrEqual, 12 => GeneralGreaterThan, 13 => GeneralGreaterThanOrEqual,
    14 => NodeIs, 15 => NodePrecedes, 16 => NodeFollows, 17 => Concatenate, 18 => Range,
    19 => Add, 20 => Subtract, 21 => Multiply, 22 => Divide, 23 => IntegerDivide, 24 => Modulo,
    25 => Union, 26 => Intersect, 27 => Except, 28 => Sequence
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_tags_counts_scalars_and_ranges_fail_before_allocation() {
        assert!(Reader::new(&[2]).read::<bool>().is_err());
        assert!(Reader::new(&[2]).read::<Option<String>>().is_err());
        assert!(Reader::new(&[255]).read::<XPathExpression>().is_err());
        assert!(Reader::new(&u32::MAX.to_le_bytes()).read::<char>().is_err());
        assert_eq!(
            Reader::new(&u64::MAX.to_le_bytes())
                .read::<Vec<String>>()
                .unwrap_err()
                .code,
            "cem.xpath.artifact_limit"
        );
        let range = XPathSourceRange::new(1, 1, u64::MAX, 1);
        assert!(Writer::new(&[]).write(&range).is_err());
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&u64::MAX.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        assert!(Reader::new(&bytes).read::<XPathSourceRange>().is_err());
    }

    #[test]
    fn noncanonical_maps_and_deep_wire_programs_are_rejected() {
        let mut writer = Writer::new(&[]);
        writer.write(&2usize).unwrap();
        for _ in 0..2 {
            writer.write(&"same".to_owned()).unwrap();
            writer.write(&"value".to_owned()).unwrap();
        }
        assert!(Reader::new(&writer.finish())
            .read::<BTreeMap<String, String>>()
            .is_err());
        // Unary(Plus, Unary(Plus, ...)): decoder depth is checked before
        // walking an arbitrarily nested, not-yet-complete program.
        let deep = [1, 0].repeat(256);
        assert_eq!(
            Reader::new(&deep)
                .read::<XPathExpression>()
                .unwrap_err()
                .code,
            "cem.xpath.artifact_limit"
        );
    }
}

variants!(XPathLookupKey { 0 => Name(value), 1 => Integer(value), 2 => Expression(value), 3 => Wildcard });
