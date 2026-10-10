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

#[derive(Debug)]
struct CountedTokens {
    calls: Arc<std::sync::atomic::AtomicUsize>,
    spans: Vec<Range<usize>>,
    reject: bool,
    cancel: bool,
}
impl Tokenizer for CountedTokens {
    fn tokenize(
        &self,
        request: TokenizationRequest<'_>,
    ) -> Result<Vec<Range<usize>>, TokenizationError> {
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.cancel {
            request.control.cancel_root(None, None).unwrap();
        }
        if self.reject {
            Err(TokenizationError::Rejected)
        } else {
            Ok(self.spans.clone())
        }
    }
}
#[test]
fn checked_tokenizers_preserve_boundaries_and_preflight_transitive_calls() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let calls = Arc::new(AtomicUsize::new(0));
    let original = LexicalInput::new(Arc::from(" a a "), Default::default());
    let tokenizer = |spans| {
        RegisteredTokenizer::new(
            "same-id",
            CountedTokens {
                calls: calls.clone(),
                spans,
                reject: false,
                cancel: false,
            },
        )
        .unwrap()
    };
    let base = tokenizer(vec![1..2, 3..4]);
    let child = tokenizer(vec![1..2, 3..4]).checked_replacement(&base);
    let leaf = tokenizer(vec![1..2, 3..4]).checked_replacement(&child);
    assert_eq!(leaf.invocations(), 3);
    let control = OperationControl::default();
    let result = leaf
        .tokenize(
            Some(original.clone()),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default(),
        )
        .unwrap();
    assert_eq!(result.tokens, vec![1..2, 3..4]);
    assert!(Arc::ptr_eq(&original.text, &result.input.text));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let limited = leaf.tokenize(
        Some(original.clone()),
        &control,
        ROOT_EXECUTION_SCOPE_ID,
        TokenizationLimits {
            max_tokenizers: 2,
            ..Default::default()
        },
    );
    assert!(matches!(limited, Err(TokenizationError::Limit)));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let changed = tokenizer(vec![1..4]).checked_replacement(&child);
    assert!(matches!(
        changed.tokenize(
            Some(original),
            &control,
            ROOT_EXECUTION_SCOPE_ID,
            Default::default()
        ),
        Err(TokenizationError::IncompatibleReplacement)
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 6);
}
#[test]
fn checked_tokenizers_stop_after_base_rejection_failure_or_cancellation() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    for (reject, cancel, spans) in [
        (true, false, vec![]),
        (true, true, vec![]),
        (false, true, vec![0..1]),
        (false, false, vec![1..2]),
    ] {
        let base_calls = Arc::new(AtomicUsize::new(0));
        let child_calls = Arc::new(AtomicUsize::new(0));
        let base = RegisteredTokenizer::new(
            "base",
            CountedTokens {
                calls: base_calls.clone(),
                spans,
                reject,
                cancel,
            },
        )
        .unwrap();
        let child = RegisteredTokenizer::new(
            "child",
            CountedTokens {
                calls: child_calls.clone(),
                spans: vec![0..1],
                reject: false,
                cancel: false,
            },
        )
        .unwrap()
        .checked_replacement(&base);
        let result = child.tokenize(
            Some(LexicalInput::new(Arc::from("a"), Default::default())),
            &OperationControl::default(),
            ROOT_EXECUTION_SCOPE_ID,
            Default::default(),
        );
        assert!(result.is_err());
        if cancel {
            assert!(matches!(result, Err(TokenizationError::Control(_))));
        }
        assert_eq!(base_calls.load(Ordering::SeqCst), 1);
        assert_eq!(child_calls.load(Ordering::SeqCst), 0);
    }
}
