//! AC-S-V-2 / AC-S-V-4 / AC-S-V-5 declaration-shape fixture:
//! `Validated<T>` flows as `T`, but values validated against one schema
//! version are not assignable to another version's validated type.

use std::fs;

use super::ts_fixture_support::{prepare_ts_project, resolve_tsc, run_tsc};

#[test]
fn cem_core_dts_validated_brand_is_schema_version_discriminated() {
    let tsc = match resolve_tsc() {
        Some(path) => path,
        None => {
            eprintln!("info: `tsc` not available — skipping AC-S-V validated-brand fixture");
            return;
        }
    };

    let project = prepare_ts_project("cem_ml_ts_dts_validated_brand");
    fs::write(
        project.join("fixture.ts"),
        concat!(
            "import { asValidated, tryValidated } from \"@epa-wg/cem-ml/schema/core/1.0.0/cem-core\";\n",
            "import type { Badge as Badge1, Validated as Validated1 } from \"@epa-wg/cem-ml/schema/core/1.0.0/cem-core\";\n",
            "import type { Badge as Badge2, Validated as Validated2 } from \"@epa-wg/cem-ml/schema/core/2.0.0/cem-core\";\n",
            "\n",
            "declare const validatedV1: Validated1<Badge1>;\n",
            "const structuralBadge: Badge1 = validatedV1;\n",
            "const structuralElement: HTMLElement = validatedV1;\n",
            "const runtimeValidated = asValidated<Badge1>(structuralBadge);\n",
            "const runtimeMaybe = tryValidated<Badge1>(structuralBadge);\n",
            "void structuralElement;\n",
            "void runtimeValidated;\n",
            "void runtimeMaybe;\n",
            "\n",
            "// @ts-expect-error schema-version brands must not be assignable across subpaths\n",
            "const wrongVersion: Validated2<Badge2> = validatedV1;\n",
            "void wrongVersion;\n",
        ),
    )
    .expect("write TypeScript fixture");

    let output = run_tsc(&tsc, &project);
    assert!(
        output.status.success(),
        "tsc rejected validated-brand d.ts fixture:\n--- stderr ---\n{}\n--- stdout ---\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout),
    );
}
