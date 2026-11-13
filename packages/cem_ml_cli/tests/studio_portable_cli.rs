//! Materialized project builds retain original identities and exact source bytes.
use cem_ml::command_service::sha256_hex;
use cem_ml::studio_project::{
    parse_studio_project, StudioProjectProjection, STUDIO_PROJECT_JSON_CONTENT_TYPE,
    STUDIO_PROJECT_SCHEMA_URI,
};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Project {
    root: PathBuf,
    model: Value,
    sources: Vec<(String, Vec<u8>)>,
}
impl Project {
    fn login() -> Self {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let definition: Value = serde_json::from_slice(
            &fs::read(workspace.join("examples/projects/login/example.json")).unwrap(),
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!(
            "cem-portable-cli-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut resources = vec![];
        let mut sources = vec![];
        for source in definition["sources"].as_array().unwrap() {
            let id = source["id"].as_str().unwrap();
            let path = source["path"].as_str().unwrap();
            let bytes = fs::read(workspace.join(source["source"].as_str().unwrap())).unwrap();
            let output = root.join(path);
            fs::create_dir_all(output.parent().unwrap()).unwrap();
            fs::write(output, &bytes).unwrap();
            resources.push(json!({"id":id,"role":"data","sourceKind":"project-file","path":path,"referencePath":path,
                "contentType":source["contentType"],"schema":source["schema"],"revision":1,"sha256":sha256_hex(&bytes)}));
            sources.push((id.to_owned(), bytes));
        }
        let model = json!({"$schema":STUDIO_PROJECT_SCHEMA_URI,"schemaVersion":1,"id":"portable-login","name":"Portable Login",
            "rootUri":"studio://portable-login/","revision":1,"createdAt":"2026-11-13T00:00:00Z","updatedAt":"2026-11-13T00:00:00Z","entries":[],"resources":resources});
        Self {
            root,
            model,
            sources,
        }
    }
    fn run(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .args(args)
            .current_dir(&self.root)
            .output()
            .unwrap()
    }
    fn add(&mut self, id: &str, path: &str, bytes: &[u8], content_type: &str, schema: &str) {
        let output = self.root.join(path);
        fs::create_dir_all(output.parent().unwrap()).unwrap();
        fs::write(output, bytes).unwrap();
        self.model["resources"].as_array_mut().unwrap().push(json!({"id":id,"role":"data","sourceKind":"project-file","path":path,"referencePath":path,
            "contentType":content_type,"schema":schema,"revision":1,"sha256":sha256_hex(bytes)}));
        self.sources.push((id.into(), bytes.into()));
    }
    fn move_and_export(&mut self) {
        for resource in self.model["resources"].as_array_mut().unwrap() {
            let path = format!(
                "moved/it's $(touch nope) `{}`.source",
                resource["id"].as_str().unwrap()
            );
            let output = self.root.join(&path);
            fs::create_dir_all(output.parent().unwrap()).unwrap();
            fs::rename(self.root.join(resource["path"].as_str().unwrap()), output).unwrap();
            resource["path"] = path.into();
        }
        let project = parse_studio_project(
            &serde_json::to_vec(&self.model).unwrap(),
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap();
        fs::write(
            self.root.join("project.cem"),
            project.serialize(StudioProjectProjection::Cem).unwrap(),
        )
        .unwrap();
    }
    fn assert_sources(&self) {
        for (id, bytes) in &self.sources {
            let resource = self.model["resources"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == *id)
                .unwrap();
            assert_eq!(
                &fs::read(self.root.join(resource["path"].as_str().unwrap())).unwrap(),
                bytes
            );
        }
        assert!(!self.root.join("nope").exists());
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn success(output: &Output) {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn renamed_nested_login_graph_matches_original_materialization() {
    let mut project = Project::login();
    let baseline = project.run(&["transform", "--config", "graphs/view.cem"]);
    success(&baseline);
    let expected = fs::read(project.root.join("generated/login.html")).unwrap();
    project.move_and_export();
    for config in ["graphs/view.cem", "studio://portable-login/graphs/view.cem"] {
        fs::remove_file(project.root.join("generated/login.html")).unwrap();
        let result = project.run(&[
            "transform",
            "--config",
            config,
            "--resolver-project",
            "project.cem",
        ]);
        success(&result);
        assert_eq!(
            fs::read(project.root.join("generated/login.html")).unwrap(),
            expected
        );
        project.assert_sources();
    }
}

#[test]
fn explicit_input_and_renamed_extension_keep_declared_identity_and_bytes() {
    let mut project = Project::login();
    let args = [
        "convert",
        "data/login.cem",
        "--to-content-type",
        "text/html",
        "--to-schema",
        "https://cem.dev/ns/data/html/1",
        "--output-color-type",
        "none",
    ];
    let baseline = project.run(&args);
    success(&baseline);
    project.move_and_export();
    let result = project.run(&[
        "convert",
        "studio://portable-login/data/login.cem",
        "--to-content-type",
        "text/html",
        "--to-schema",
        "https://cem.dev/ns/data/html/1",
        "--output-color-type",
        "none",
        "--resolver-project",
        "project.cem",
    ]);
    success(&result);
    assert_eq!(result.stdout, baseline.stdout);
    project.assert_sources();
}

#[test]
fn missing_or_wrong_project_addresses_do_not_publish_output() {
    let mut project = Project::login();
    project.move_and_export();
    for input in [
        "studio://other/data/login.cem",
        "studio://portable-login/missing.cem",
    ] {
        let result = project.run(&[
            "convert",
            input,
            "--out",
            "sentinel.cem",
            "--resolver-project",
            "project.cem",
        ]);
        assert!(!result.status.success());
        assert!(!project.root.join("sentinel.cem").exists());
        project.assert_sources();
    }
}

#[test]
fn publication_cannot_overwrite_a_source_through_its_display_path() {
    let mut project = Project::login();
    project.move_and_export();
    let path = project.model["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "login-cem")
        .unwrap()["path"]
        .as_str()
        .unwrap();
    let destination = format!("studio://portable-login/{path}");
    let result = project.run(&[
        "convert",
        "studio://portable-login/data/login.cem",
        "--out",
        &destination,
        "--to-content-type",
        "text/html",
        "--to-schema",
        "https://cem.dev/ns/data/html/1",
        "--resolver-project",
        "project.cem",
    ]);
    assert!(!result.status.success());
    project.assert_sources();
}

#[test]
fn input_and_output_records_keep_quoted_commas_and_native_output_bytes() {
    let mut project = Project::login();
    success(&project.run(&[
        "convert",
        "data/login.cem",
        "--out",
        "baseline.html",
        "--to-content-type",
        "text/html",
        "--to-schema",
        "https://cem.dev/ns/data/html/1",
    ]));
    let expected = fs::read(project.root.join("baseline.html")).unwrap();
    project.move_and_export();
    let result = project.run(&["convert", "--input-spec", "uri=data/login.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1",
        "--output-spec", "input=data/login.cem,dest=\"generated/a,b page.html\",contentType=text/html,schema=https://cem.dev/ns/data/html/1",
        "--resolver-project", "project.cem"]);
    success(&result);
    assert_eq!(
        fs::read(project.root.join("generated/a,b page.html")).unwrap(),
        expected
    );
    project.assert_sources();
}

#[test]
fn glob_reads_use_original_reference_topology_after_moves() {
    let mut project = Project::login();
    for id in ["a", "b"] {
        project.add(
            id,
            &format!("data/items/{id}.cem"),
            format!("{{p | {id}}}").as_bytes(),
            "application/cem",
            "https://cem.dev/ns/cem-ml/1",
        );
    }
    project.add("item-template", "templates/item.cemt", br#"@doc cem-ml 1
@ns template = "https://cem.dev/ns/template/cem-native/1"
@default template
{param @name=input @required=true}
{template @name=main @visibility=public | {element @name=p @namespace=http://www.w3.org/1999/xhtml | Item}}"#,
        "application/vnd.cem.template+cem", "https://cem.dev/ns/template/cem-native/1");
    project.add("glob", "graphs/glob.cem", br#"{run | {import @id=items @src="../data/items/*.cem" @content-type=application/cem @schema=https://cem.dev/ns/cem-ml/1 |
        {transform @id=view @src=../templates/item.cemt @entrypoint=main @template-content-type=application/vnd.cem.template+cem @template-schema=https://cem.dev/ns/template/cem-native/1 |
        {export @id=page @out="../generated/items/{page}.html" @content-type=text/html @schema=https://cem.dev/ns/data/html/1}}}}"#,
        "application/cem", "https://cem.dev/ns/cli/transform-config/1");
    success(&project.run(&["transform", "--config", "graphs/glob.cem"]));
    let expected = ["a", "b"]
        .map(|id| fs::read(project.root.join(format!("generated/items/{id}.html"))).unwrap());
    project.move_and_export();
    fs::remove_dir_all(project.root.join("generated/items")).unwrap();
    success(&project.run(&[
        "transform",
        "--config",
        "graphs/glob.cem",
        "--resolver-project",
        "project.cem",
    ]));
    for (index, id) in ["a", "b"].iter().enumerate() {
        assert_eq!(
            fs::read(project.root.join(format!("generated/items/{id}.html"))).unwrap(),
            expected[index]
        );
    }
    project.assert_sources();
}

#[test]
fn json_manifest_and_escaped_original_addresses_keep_exact_unicode_source() {
    let mut project = Project::login();
    project.add(
        "quoted",
        "data/a b, it's.cem",
        "{p | é😀}".as_bytes(),
        "application/cem",
        "https://cem.dev/ns/cem-ml/1",
    );
    let baseline = project.run(&[
        "convert",
        "data/a b, it's.cem",
        "--to-content-type",
        "text/html",
        "--to-schema",
        "https://cem.dev/ns/data/html/1",
        "--output-color-type",
        "none",
    ]);
    success(&baseline);
    project.move_and_export();
    fs::write(
        project.root.join("project.json"),
        serde_json::to_vec(&project.model).unwrap(),
    )
    .unwrap();
    let result = project.run(&[
        "convert",
        "data/a b, it's.cem",
        "--to-content-type",
        "text/html",
        "--to-schema",
        "https://cem.dev/ns/data/html/1",
        "--output-color-type",
        "none",
        "--resolver-project",
        "project.json",
    ]);
    success(&result);
    assert_eq!(result.stdout, baseline.stdout);
    project.assert_sources();
}

#[test]
fn changed_materialized_source_retains_the_last_successful_output() {
    let mut project = Project::login();
    project.move_and_export();
    fs::create_dir_all(project.root.join("generated")).unwrap();
    fs::write(
        project.root.join("generated/login.html"),
        b"last accepted output",
    )
    .unwrap();
    let path = project.model["resources"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == "login-cem")
        .unwrap()["path"]
        .as_str()
        .unwrap();
    fs::write(project.root.join(path), "{p | different saved revision}").unwrap();
    let result = project.run(&[
        "transform",
        "--config",
        "graphs/view.cem",
        "--resolver-project",
        "project.cem",
    ]);
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("source_changed"));
    assert_eq!(
        fs::read(project.root.join("generated/login.html")).unwrap(),
        b"last accepted output"
    );
}

#[test]
fn a_source_alias_in_a_graph_rolls_back_the_complete_publication_batch() {
    let mut project = Project::login();
    project.add("collision", "graphs/collision.cem", br#"{run | {import @id=source @src=../data/login.cem @content-type=application/cem @schema=https://cem.dev/ns/cem-ml/1 |
        {transform @id=view @src=../templates/view.cemt @entrypoint=main @template-content-type=application/vnd.cem.template+cem @template-schema=https://cem.dev/ns/template/cem-native/1 |
          {export @id=first @out=../generated/first.html @content-type=text/html @schema=https://cem.dev/ns/data/html/1}
          {export @id=conflict @out="studio://portable-login/moved/it's $(touch nope) `login-cem`.source" @content-type=text/html @schema=https://cem.dev/ns/data/html/1}
        }}}"#, "application/cem", "https://cem.dev/ns/cli/transform-config/1");
    project.move_and_export();
    fs::create_dir_all(project.root.join("generated")).unwrap();
    fs::write(
        project.root.join("generated/first.html"),
        b"previous complete publication",
    )
    .unwrap();
    let result = project.run(&[
        "transform",
        "--config",
        "graphs/collision.cem",
        "--resolver-project",
        "project.cem",
    ]);
    assert!(!result.status.success());
    assert_eq!(
        fs::read(project.root.join("generated/first.html")).unwrap(),
        b"previous complete publication"
    );
    project.assert_sources();
}

#[test]
fn explicit_other_scheme_output_maps_retain_their_authored_destination() {
    let mut project = Project::login();
    project.move_and_export();
    let result = project.run(&["convert", "--input-spec", "uri=data/login.cem,contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1",
        "--output-spec", "input=data/login.cem,dest=mirror://published/out.html,contentType=text/html,schema=https://cem.dev/ns/data/html/1",
        "--resolver-write-map", "mirror://published/=exports", "--resolver-project", "project.cem"]);
    success(&result);
    assert!(
        fs::metadata(project.root.join("exports/out.html"))
            .unwrap()
            .len()
            > 0
    );
    project.assert_sources();
}

#[test]
fn nested_json_run_configuration_keeps_input_and_output_bases_after_moves() {
    let mut project = Project::login();
    let config = json!({"inputs":[{"uri":"data/login.cem","rootScope":{"defaultContentType":"application/cem","schema":"https://cem.dev/ns/cem-ml/1"}}],
        "outputs":[{"inputRef":"data/login.cem","destination":"generated/config.html","rootScope":{"defaultContentType":"text/html","schema":"https://cem.dev/ns/data/html/1"}}]});
    project.add(
        "run-config",
        "config/convert.json",
        &serde_json::to_vec(&config).unwrap(),
        "application/json",
        "https://cem.dev/ns/cli/run-config/1",
    );
    success(&project.run(&["convert", "--config", "config/convert.json"]));
    let expected = fs::read(project.root.join("config/generated/config.html")).unwrap();
    project.move_and_export();
    fs::remove_file(project.root.join("config/generated/config.html")).unwrap();
    success(&project.run(&[
        "convert",
        "--config",
        "config/convert.json",
        "--resolver-project",
        "project.cem",
    ]));
    assert_eq!(
        fs::read(project.root.join("config/generated/config.html")).unwrap(),
        expected
    );
    project.assert_sources();
}
