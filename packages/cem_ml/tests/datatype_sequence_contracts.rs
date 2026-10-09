use cem_ml::{
    operation_control::{OperationControl, ROOT_EXECUTION_SCOPE_ID},
    scheduler::AbortSignal,
    schema::datatype_contracts::{
        ItemBounds, LexicalInput, RegisteredTokenizer, TokenizationError, TokenizationLimits,
        TokenizationRequest, Tokenizer,
    },
};
use std::{ops::Range, sync::Arc};
#[test]
fn bounds_intersect_without_widening_and_empty_differs_from_absent() {
    assert!(ItemBounds::default().admits(0));
    let nonempty = ItemBounds::new(1, None).unwrap();
    assert!(!nonempty.intersect(ItemBounds::default()).unwrap().admits(0));
    assert!(nonempty
        .intersect(ItemBounds::new(0, Some(0)).unwrap())
        .is_err());
    let control = OperationControl::default();
    let tokenizer = RegisteredTokenizer::whitespace();
    let run = |input| {
        tokenizer.tokenize(
            input,
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            TokenizationLimits::default(),
        )
    };
    assert!(matches!(run(None), Err(TokenizationError::AbsentInput)));
    assert!(run(Some(LexicalInput::new(
        Arc::from(" \t\n"),
        Default::default()
    )))
    .unwrap()
    .tokens
    .is_empty());
}
#[test]
fn whitespace_preserves_order_duplicates_unicode_and_original_lexical_owner() {
    let original: Arc<str> = Arc::from(" α\tβ α ");
    let input = LexicalInput::new(original.clone(), Default::default());
    let result = RegisteredTokenizer::whitespace()
        .tokenize(
            Some(input),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            Default::default(),
        )
        .unwrap();
    assert!(Arc::ptr_eq(&original, &result.input.text));
    assert_eq!(
        result
            .tokens
            .iter()
            .map(|r| &result.input.text[r.clone()])
            .collect::<Vec<_>>(),
        vec!["α", "β", "α"]
    );
}
#[derive(Debug)]
struct Invalid(Vec<Range<usize>>);
impl Tokenizer for Invalid {
    fn tokenize(&self, _: TokenizationRequest<'_>) -> Result<Vec<Range<usize>>, TokenizationError> {
        Ok(self.0.clone())
    }
}
#[test]
fn custom_tokenizers_cannot_drop_lexical_content_or_return_invalid_spans() {
    for spans in [
        vec![0..1],
        vec![1..2],
        vec![0..2, 0..2],
        vec![0..99],
        vec![],
    ] {
        let t = RegisteredTokenizer::new("custom", Invalid(spans)).unwrap();
        assert!(t
            .tokenize(
                Some(LexicalInput::new(Arc::from("α x"), Default::default())),
                &OperationControl::default(),
                ROOT_EXECUTION_SCOPE_ID,
                Default::default()
            )
            .is_err());
    }
}
#[test]
fn tokenization_is_bounded_and_cooperates_with_operation_control() {
    let input = || Some(LexicalInput::new(Arc::from("a b"), Default::default()));
    let t = RegisteredTokenizer::whitespace();
    let abort = AbortSignal::new();
    let control = OperationControl::new(abort.clone());
    assert!(t
        .tokenize(
            input(),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            TokenizationLimits {
                max_tokens: 1,
                ..Default::default()
            }
        )
        .is_err());
    abort.abort();
    assert!(t
        .tokenize(
            input(),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default()
        )
        .is_err());
}

#[test]
fn custom_delimiters_require_the_registered_separator_policy() {
    #[derive(Debug)]
    struct Commas;
    impl Tokenizer for Commas {
        fn tokenize(
            &self,
            _: TokenizationRequest<'_>,
        ) -> Result<Vec<std::ops::Range<usize>>, TokenizationError> {
            Ok(vec![0..1, 2..3])
        }
        fn accepts_separator(&self, text: &str) -> bool {
            text.chars().all(|c| c == ',')
        }
    }
    let tokenizer = RegisteredTokenizer::new("comma-v1", Commas).unwrap();
    let control = OperationControl::default();
    let result = tokenizer
        .tokenize(
            Some(LexicalInput::new(Arc::from("a,b"), Default::default())),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default(),
        )
        .unwrap();
    assert_eq!(result.tokens, vec![0..1, 2..3]);
    assert!(matches!(
        tokenizer.tokenize(
            Some(LexicalInput::new(Arc::from("a b"), Default::default())),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default()
        ),
        Err(TokenizationError::UnaccountedInput)
    ));
}
