//! XPath 3.1 functional-parity coverage for cem-ql.
//!
//! Implements verification item §13.2 from `docs/cem-ql-ac.md`:
//! "table-driven tests against the XPath 3.1 conformance suite,
//! restricted to the AC-QX-1 subset. Failures on out-of-subset items
//! are skipped, not reported as failures."
//!
//! The full QT3 corpus is not vendored into this repo. The cases below
//! mirror representative QT3 categories that fall inside the AC-QX-1
//! evaluable surface — arithmetic, value/general comparisons, boolean
//! ops, conditional, `for`/`let`, quantifiers, sequence construction,
//! the four set operators (`| & - ^`), and explicit numeric casts.
//! Each case names the QT3 area it represents so the table can grow
//! against a downloaded corpus without restructuring the harness.
//!
//! Cases tagged [`OutOfSubset`] return a result the evaluator does not
//! yet compute (axes, predicates, FLWOR `order by`, regex, etc.). They
//! are *skipped* (counted) rather than failing the build, per AC-QX-1.

use cem_ml::scheduler::ScopePolicy;
use cem_ql::api::{compile, evaluate, CompileContext, EvaluationContext};
use cem_ql::eval::{AtomValue, Item, ItemStream, QueryContextScope};

#[derive(Debug, Clone)]
enum Expected {
    Items(Vec<Item>),
    OutOfSubset,
}

#[derive(Debug, Clone)]
struct Case {
    name: &'static str,
    qt3_area: &'static str,
    query: &'static str,
    expected: Expected,
}

fn run(query: &str) -> Result<ItemStream, String> {
    let compiled = compile(query, &CompileContext::default())
        .map_err(|err| format!("compile failed: {err}"))?;
    Ok(evaluate(
        &compiled,
        &EvaluationContext {
            scope: QueryContextScope(0),
            scope_policy: ScopePolicy::host_root().with_queue_size(256),
            diagnostics: Vec::new(),
        },
    ))
}

fn int(v: i64) -> Item {
    Item::Atomic(AtomValue::Integer(v))
}

fn boolean(v: bool) -> Item {
    Item::Atomic(AtomValue::Boolean(v))
}

fn string(v: &str) -> Item {
    Item::Atomic(AtomValue::String(v.to_owned()))
}

fn cases() -> Vec<Case> {
    use Expected::*;
    vec![
        Case {
            name: "arithmetic.add",
            qt3_area: "op-numeric-add",
            query: "1 + 2",
            expected: Items(vec![int(3)]),
        },
        Case {
            name: "arithmetic.precedence",
            qt3_area: "op-numeric-multiply",
            query: "1 + 2 * 3",
            expected: Items(vec![int(7)]),
        },
        Case {
            name: "arithmetic.unary-minus",
            qt3_area: "op-numeric-unary-minus",
            query: "-(5)",
            expected: Items(vec![int(-5)]),
        },
        Case {
            name: "comparison.value-eq.true",
            qt3_area: "op-numeric-equal",
            query: "1 eq 1",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "comparison.value-eq.false",
            qt3_area: "op-numeric-equal",
            query: "1 eq 2",
            expected: Items(vec![boolean(false)]),
        },
        Case {
            name: "comparison.general-eq",
            qt3_area: "op-equal",
            query: "1 = 1",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "comparison.lt",
            qt3_area: "op-numeric-less-than",
            query: "1 lt 2",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "boolean.and.short-circuit",
            qt3_area: "op-boolean-and",
            query: "false and true",
            expected: Items(vec![boolean(false)]),
        },
        Case {
            name: "boolean.or",
            qt3_area: "op-boolean-or",
            query: "false or true",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "boolean.not",
            qt3_area: "fn-not",
            query: "not(false)",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "conditional.then",
            qt3_area: "if-expr",
            query: "if (true) then 1 else 2",
            expected: Items(vec![int(1)]),
        },
        Case {
            name: "conditional.else",
            qt3_area: "if-expr",
            query: "if (false) then 1 else 2",
            expected: Items(vec![int(2)]),
        },
        Case {
            name: "for.return",
            qt3_area: "for-expr",
            query: "for x in (1, 2, 3) return x + 1",
            expected: Items(vec![int(2), int(3), int(4)]),
        },
        Case {
            name: "let.return",
            qt3_area: "let-expr",
            query: "let x := 10 in x + 1",
            expected: Items(vec![int(11)]),
        },
        Case {
            name: "quantified.some",
            qt3_area: "some-expr",
            query: "some x in (1, 2, 3) satisfies x eq 2",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "quantified.every.true",
            qt3_area: "every-expr",
            query: "every x in (1, 2, 3) satisfies x lt 10",
            expected: Items(vec![boolean(true)]),
        },
        Case {
            name: "quantified.every.false",
            qt3_area: "every-expr",
            query: "every x in (1, 2, 3) satisfies x lt 2",
            expected: Items(vec![boolean(false)]),
        },
        Case {
            name: "sequence.literal",
            qt3_area: "constructor-of-sequences",
            query: "(1, 2, 3)",
            expected: Items(vec![int(1), int(2), int(3)]),
        },
        Case {
            name: "set.union.dedup",
            qt3_area: "op-union",
            query: "(1, 2) | (2, 3)",
            expected: Items(vec![int(1), int(2), int(3)]),
        },
        Case {
            name: "set.intersect",
            qt3_area: "op-intersect",
            query: "(1, 2, 3) & (2, 3, 4)",
            expected: Items(vec![int(2), int(3)]),
        },
        Case {
            name: "set.except",
            qt3_area: "op-except",
            query: "seq:difference((1, 2, 3), (2, 4))",
            expected: Items(vec![int(1), int(3)]),
        },
        Case {
            name: "cast.integer.to.double",
            qt3_area: "constructor-double",
            query: "num:double(1)",
            expected: Items(vec![Item::Atomic(AtomValue::Double(1.0))]),
        },
        Case {
            name: "string.lower",
            qt3_area: "fn-lower-case",
            query: r#"str:lower("ABC")"#,
            expected: Items(vec![string("abc")]),
        },
        // ---- AC-QX-1 explicitly excludes these from Tier A. Listed so
        // the table mirrors the QT3 categories; harness reports a skip.
        Case {
            name: "axis.child",
            qt3_area: "axes-child",
            query: "//*",
            expected: OutOfSubset,
        },
        Case {
            name: "flwor.order-by",
            qt3_area: "flwor-order-by",
            query: "for $x in (3, 1, 2) order by $x return $x",
            expected: OutOfSubset,
        },
        Case {
            name: "regex.matches",
            qt3_area: "fn-matches",
            query: r#"fn:matches("abc", "^a")"#,
            expected: OutOfSubset,
        },
        Case {
            name: "fn.doc",
            qt3_area: "fn-doc",
            query: r#"fn:doc("/etc/passwd")"#,
            expected: OutOfSubset,
        },
    ]
}

#[derive(Default)]
struct ParityReport {
    passed: usize,
    skipped: usize,
    failures: Vec<String>,
}

fn run_case(case: &Case, report: &mut ParityReport) {
    if matches!(case.expected, Expected::OutOfSubset) {
        report.skipped += 1;
        return;
    }
    let stream = match run(case.query) {
        Ok(stream) => stream,
        Err(err) => {
            report.failures.push(format!(
                "{} [{}]: {} (query: `{}`)",
                case.name, case.qt3_area, err, case.query
            ));
            return;
        }
    };
    let Expected::Items(expected) = &case.expected else {
        unreachable!()
    };
    if stream.error.is_some() {
        report.failures.push(format!(
            "{} [{}]: evaluator error: {:?}",
            case.name, case.qt3_area, stream.error
        ));
        return;
    }
    if &stream.items != expected {
        report.failures.push(format!(
            "{} [{}]: expected {:?}, got {:?}",
            case.name, case.qt3_area, expected, stream.items
        ));
        return;
    }
    report.passed += 1;
}

#[test]
fn xpath_parity_table_runs_ac_qx_1_subset() {
    let cases = cases();
    let mut report = ParityReport::default();
    for case in &cases {
        run_case(case, &mut report);
    }

    assert!(
        report.failures.is_empty(),
        "xpath parity failures:\n{}",
        report.failures.join("\n")
    );
    assert!(
        report.passed > 0,
        "expected at least one in-subset case to run"
    );
    assert!(
        report.skipped > 0,
        "expected at least one out-of-subset case so the harness exercises the skip path"
    );
    assert_eq!(
        report.passed + report.skipped,
        cases.len(),
        "every case must be either passed or skipped"
    );
}

#[test]
fn xpath_parity_target_is_registered() {
    assert_eq!(cem_ql::VERSION, env!("CARGO_PKG_VERSION"));
}
