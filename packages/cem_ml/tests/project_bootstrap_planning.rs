use cem_ml::project_bootstrap::project_bootstrap_json;
use serde_json::{json, Value};

fn plan(preset: &str, answers: Value) -> Value {
    serde_json::from_str(&project_bootstrap_json(&json!({"action":"plan", "preset":preset,
        "projectId":"planned", "name":"My project", "createdAt":"2026-11-13T00:00:00Z", "answers":answers}).to_string())).unwrap()
}
fn paths(tree: &Value) -> Vec<String> {
    let mut result = vec![];
    if tree["kind"] == "file" {
        result.push(tree["path"].as_str().unwrap().into());
    }
    for child in tree["children"].as_array().unwrap() {
        result.extend(paths(child));
    }
    result
}

#[test]
fn site_plan_has_editable_nested_pages_shared_layout_navigation_css_and_static() {
    let result = plan(
        "markdown-site",
        json!({"sourceFiles":"index.md\nabout.md\nguides/start.md", "sourceFolder":"content", "outputFolder":"public"}),
    );
    assert!(result.get("error").is_none(), "{result}");
    let sources = paths(&result["review"]["sources"]);
    for expected in [
        "content/index.md",
        "content/about.md",
        "content/guides/start.md",
        "layouts/page.cemt",
        "layouts/chrome.cemt",
        "assets/css/site.css",
        "static/logo.svg",
    ] {
        assert!(
            sources.contains(&expected.into()),
            "{expected}: {sources:?}"
        );
    }
    let targets = paths(&result["review"]["outputs"]);
    for expected in [
        "public/index.html",
        "public/about.html",
        "public/guides/start.html",
        "public/assets/css/site.css",
        "public/static/logo.svg",
    ] {
        assert!(
            targets.contains(&expected.into()),
            "{expected}: {targets:?}"
        );
    }
    let config = &result["configuration"];
    let nav = config["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "site")
        .unwrap();
    let data: Value = serde_json::from_str(nav["text"].as_str().unwrap()).unwrap();
    assert_eq!(data["navigation"][2]["href"], "guides/start.html");
    let page = config["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == "content/guides/start.md")
        .unwrap();
    assert!(page["text"].as_str().unwrap().contains("base: \"..\""));
    assert_eq!(config["entries"][0]["entryArtifact"], "public/index.html");
}

#[test]
fn pipeline_plan_pairs_source_and_target_trees_with_a_shared_template() {
    let result = plan(
        "transformation-pipeline",
        json!({"sourceFiles":"sources/a.xml\nsources/nested/b.xml", "targetFiles":"dist/a.html\ndist/nested/b.html"}),
    );
    assert!(result.get("error").is_none(), "{result}");
    assert!(paths(&result["review"]["sources"]).contains(&"sources/nested/b.xml".into()));
    assert_eq!(
        paths(&result["review"]["outputs"]),
        ["dist/a.html", "dist/nested/b.html"]
    );
    assert_eq!(
        result["review"]["pipelines"][0]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        6
    );
}

#[test]
fn simple_validation_and_custom_plans_keep_distinct_structures() {
    let simple = plan(
        "simple-conversion",
        json!({"sourceFiles":"in/example.json", "targetFiles":"out/example.cem", "inputFormat":"json", "outputFormat":"cem"}),
    );
    assert!(simple.get("error").is_none(), "{simple}");
    assert_eq!(paths(&simple["review"]["outputs"]), ["out/example.cem"]);
    let validation = plan(
        "schema-validation",
        json!({"sourceFiles":"samples/a.xhtml\nsamples/b.xhtml"}),
    );
    assert!(validation.get("error").is_none(), "{validation}");
    assert!(paths(&validation["review"]["outputs"]).is_empty());
    let custom = plan(
        "custom",
        json!({"sourceFiles":"README.md\nsrc/data.json\nstyles/site.css\nstatic/logo.svg"}),
    );
    assert!(custom.get("error").is_none(), "{custom}");
    assert_eq!(
        custom["configuration"]["resources"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert!(custom["configuration"]["entries"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e["runConfigResourceId"].is_null()));
    let typed = plan(
        "custom",
        json!({"sourceFiles":"data/input.cem\ntemplates/main.cemt\nnotes.txt"}),
    );
    assert!(typed.get("error").is_none(), "{typed}");
    assert_eq!(typed["configuration"]["resources"][1]["role"], "template");
    assert_eq!(
        typed["configuration"]["resources"][2]["contentType"],
        "text/plain"
    );
    assert!(typed["configuration"]["resources"][2]
        .get("schema")
        .is_none());
}

#[test]
fn invalid_questionnaire_does_not_return_review_or_partial_configuration() {
    for (preset, answers) in [
        ("markdown-site", json!({"sourceFiles":"../escape.md"})),
        (
            "markdown-site",
            json!({"sourceFolder":"layouts", "sourceFiles":"page.cemt"}),
        ),
        (
            "markdown-site",
            json!({"outputFolder":"content", "sourceFiles":"index.md", "cssFolder":"../outside"}),
        ),
        (
            "transformation-pipeline",
            json!({"sourceFiles":"a.xml\nb.xml", "targetFiles":"a.html"}),
        ),
        (
            "transformation-pipeline",
            json!({"sourceFiles":"a.xml", "targetFiles":"a.xml"}),
        ),
        (
            "simple-conversion",
            json!({"sourceFiles":"a.yaml\nb.yaml", "targetFiles":"a.cem\nb.cem"}),
        ),
        ("simple-conversion", json!({"inputFormat":"unknown"})),
        ("custom", json!({"sourceFiles":"a.json\na.json"})),
    ] {
        let result = plan(preset, answers);
        assert!(result.get("error").is_some(), "{preset}: {result}");
        assert!(result.get("configuration").is_none());
        assert!(result.get("review").is_none());
    }
}
