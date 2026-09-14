use std::{
    fs,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn cli_recovers_across_same_module_and_imported_calls() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("cem-cli-recovery-{}-{stamp}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let input = root.join("data.cem");
    fs::write(&input, "{main}").unwrap();
    let module = root.join("helper.cemt");
    fs::write(&module, r#"{module | {template @name=fail @visibility=public | {body | {b | partial}{$report:raise("sample.import", "bad")}}}}"#).unwrap();
    for (index, source, expected) in [
        (
            0,
            r#"{module | {body | {try | {$1 / 0}{catch @as=e | {p | {$e.code}}}}}}"#,
            "<p>cem.ql.type_error</p>",
        ),
        (
            1,
            r#"{module | {template @name=fail | {body | {b | partial}{$1 / 0}}}
            {body | {try | {call @template=fail}{catch @as=e | {p | {$e.code}}}}}}"#,
            "<p>cem.ql.type_error</p>",
        ),
        (
            2,
            r#"{module | {import @as=base @src="./helper.cemt"}
            {body | {try | {call @from=base @template=fail}{catch @as=e | {p | {$e.code}}}}}}"#,
            "<p>sample.import</p>",
        ),
    ] {
        let template = root.join(format!("case-{index}.cemt"));
        fs::write(&template, source).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args([
                "transform",
                input.to_str().unwrap(),
                "--data-content-type",
                "text/cem-ml",
                "--template",
                template.to_str().unwrap(),
                "--template-content-type",
                "text/cem-ml",
                "--to-content-type",
                "text/html",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "case {index}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            expected,
            "case {index}"
        );
    }
}
