//! AC-S-V-1 / AC-S-V-3 fixture: compile a TypeScript consumer of the
//! emitted `.d.ts` and prove structural interfaces remain ordinary DOM
//! shapes by default.

use std::fs;

use super::ts_fixture_support::{prepare_ts_project, resolve_tsc, run_tsc};

#[test]
fn cem_core_dts_structural_interfaces_compile_with_tsc() {
    let tsc = match resolve_tsc() {
        Some(path) => path,
        None => {
            eprintln!("info: `tsc` not available — skipping AC-S-V structural d.ts fixture");
            return;
        }
    };

    let project = prepare_ts_project("cem_ml_ts_dts_structural");
    fs::write(
        project.join("fixture.ts"),
        concat!(
            "import type { Badge } from \"@epa-wg/cem-ml/schema/core/1.0.0/cem-core\";\n",
            "\n",
            "declare const badge: Badge;\n",
            "function acceptsHTMLElement(el: HTMLElement): HTMLElement { return el; }\n",
            "const htmlElement: HTMLElement = acceptsHTMLElement(badge);\n",
            "const badgeValue: \"success\" | \"info\" | \"warning\" | \"error\" | undefined = badge.cemBadge;\n",
            "const badgeState: \"default\" | undefined = badge.cemState;\n",
            "void htmlElement;\n",
            "void badgeValue;\n",
            "void badgeState;\n",
        ),
    )
    .expect("write TypeScript fixture");

    let output = run_tsc(&tsc, &project);
    assert!(
        output.status.success(),
        "tsc rejected structural d.ts fixture:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}
