use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Login {
    root: PathBuf,
    directory: PathBuf,
    definition: Value,
}
impl Login {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let definition: Value = serde_json::from_slice(
            &fs::read(root.join("examples/projects/login/example.json")).unwrap(),
        )
        .unwrap();
        assert!(
            definition["sources"]
                .as_array()
                .unwrap()
                .iter()
                .any(|source| source["id"] == "login-expression"),
            "Login must package a standalone expression"
        );
        let directory = std::env::temp_dir().join(format!(
            "cem-login-expression-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for source in definition["sources"].as_array().unwrap() {
            let path = directory.join(source["path"].as_str().unwrap());
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::copy(root.join(source["source"].as_str().unwrap()), path).unwrap();
        }
        Self {
            root,
            directory,
            definition,
        }
    }
    fn source(&self, id: &str) -> &Value {
        self.definition["sources"]
            .as_array()
            .unwrap()
            .iter()
            .find(|source| source["id"] == id)
            .unwrap()
    }
    fn path(&self, id: &str) -> PathBuf {
        self.directory
            .join(self.source(id)["path"].as_str().unwrap())
    }
    fn run(&self, id: &str) -> Output {
        let configuration = self.definition["configurations"]
            .as_array()
            .unwrap()
            .iter()
            .find(|configuration| configuration["id"] == id)
            .expect("packaged expression configuration");
        let args: Vec<String> = configuration["argv"]
            .as_array()
            .unwrap()
            .iter()
            .map(|arg| {
                let value = arg.as_str().unwrap();
                if value == "$source" {
                    self.path(configuration["source"].as_str().unwrap())
                        .to_string_lossy()
                        .into_owned()
                } else if let Some(id) = value.strip_prefix('$') {
                    self.path(id).to_string_lossy().into_owned()
                } else {
                    value.into()
                }
            })
            .collect();
        Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(&self.root)
            .args(args)
            .output()
            .unwrap()
    }
    fn text(&self, id: &str) -> String {
        let output = self.run(id);
        assert!(
            output.status.success(),
            "{id}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
    fn values(&self, id: &str) -> Vec<String> {
        let report: Value = serde_json::from_str(&self.text(id)).unwrap();
        assert_eq!(report["language"], "cem-ql");
        report["result"]["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["value"].as_str().unwrap().into())
            .collect()
    }
}
impl Drop for Login {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[test]
fn packaged_login_expression_evaluates_saved_sources_and_dependencies() {
    let login = Login::new();
    let expression_path = login.path("login-expression");
    let original = fs::read_to_string(&expression_path).unwrap();
    for (id, source_id) in [
        ("expression-html", "login-html"),
        ("expression-cem", "login-cem"),
    ] {
        assert_eq!(login.values(id), ["title=Sign in", "required=2"]);
        fs::write(&expression_path, original.replace("title=", "titre 🍒=")).unwrap();
        assert_eq!(login.values(id), ["titre 🍒=Sign in", "required=2"]);
        fs::write(&expression_path, &original).unwrap();
        let source = fs::read_to_string(login.path(source_id)).unwrap();
        let changed = source.replacen("Sign in", "Bienvenue 🍒", 1).replacen(
            if source_id == "login-cem" {
                " @required"
            } else {
                " required"
            },
            "",
            1,
        );
        fs::write(login.path(source_id), changed).unwrap();
        assert_eq!(login.values(id), ["title=Bienvenue 🍒", "required=1"]);
        fs::write(login.path(source_id), source).unwrap();
    }
}

#[test]
fn packaged_login_expression_formats_without_a_module_and_repairs_invalid_source() {
    let login = Login::new();
    let path = login.path("login-expression");
    let original = fs::read_to_string(&path).unwrap();
    for profile in ["pretty", "tabular"] {
        let id = format!("expression-format-{profile}");
        let formatted = login.text(&id);
        assert!(!formatted.contains("module "));
        assert!(!formatted.contains('\u{1b}'));
        assert!(formatted.contains("// Login résumé"));
        assert_eq!(fs::read_to_string(&path).unwrap(), original);
        fs::write(&path, &formatted).unwrap();
        assert_eq!(login.text(&id), formatted);
        assert_eq!(
            login.values("expression-cem"),
            ["title=Sign in", "required=2"]
        );
        fs::write(&path, format!("{original})")).unwrap();
        assert!(
            !login.run(&id).status.success(),
            "malformed expression must fail"
        );
        fs::write(&path, &original).unwrap();
        assert_eq!(login.text(&id), formatted);
    }
}
