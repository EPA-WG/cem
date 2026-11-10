#![cfg(any(target_os = "linux", windows))]
use std::{
    fs,
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn fixture() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "cem-ssr-cli-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("page.html"),r#"<!doctype html><html><head><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3"><template id="x-card" type="text/cem-ml">{article | Published}</template></cem-element><x-card></x-card></body></html>"#).unwrap();
    fs::write(
        root.join("ssr.cemt"),
        include_str!("../../cem_ml_transform_cem_ql/templates/ssr-page.cemt"),
    )
    .unwrap();
    root
}
fn params() -> [&'static str; 9] {
    [
        "page-url=https://ssr.test/guide/",
        "consumer-versions=x-card=1.9.0",
        "max-depth=8",
        "max-instances=8",
        "max-render-passes=8",
        "max-resources=16",
        "max-bytes=1048576",
        "wall-ms=60000",
        "cpu-ms=10000",
    ]
}
fn direct(root: &std::path::Path, target: &str, extra: &[&str]) -> std::process::Output {
    direct_page(root, "page.html", target, extra)
}

fn direct_page(
    root: &std::path::Path,
    source: &str,
    target: &str,
    extra: &[&str],
) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_cem-ml"));
    cmd.current_dir(root).args([
        "transform",
        source,
        "--template",
        "ssr.cemt",
        "--template-content-type",
        "text/cem-ml",
        "--template-entrypoint",
        "ssr:render-page",
        "--to-content-type",
        target,
        "--out",
        "out.html",
    ]);
    for param in &params() {
        cmd.args(["--param", param]);
    }
    cmd.args(extra).output().unwrap()
}
#[test]
fn direct_cli_lowers_to_native_proof_and_publishes_uncolored_html() {
    let root = fixture();
    let out = direct(&root, "text/html", &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(html.contains("<article>Published</article>"), "{html}");
    assert!(html.contains("data-cem-island="));
    assert!(!html.contains('\u{1b}'));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_accepts_explicit_pinned_ssr_network_grants() {
    let root = fixture();
    let output = direct(
        &root,
        "text/html",
        &[
            "--ssr-network-read",
            "https://ssr.test/styles.css=192.0.2.10",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::remove_dir_all(root).unwrap();
}

struct SsrHttpFixture {
    origin: String,
    requests: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    stop: std::sync::mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl SsrHttpFixture {
    fn start(response: impl Fn(&str) -> String + Send + 'static) -> Self {
        use std::{
            io::{BufRead, BufReader, Write},
            net::TcpListener,
            sync::{mpsc, Arc, Mutex},
            thread,
            time::Duration,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let seen = requests.clone();
        let (stop, stopped) = mpsc::channel();
        let thread = thread::spawn(move || loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let target = {
                        let mut reader = BufReader::new(&stream);
                        let mut line = String::new();
                        if !reader.read_line(&mut line).is_ok_and(|bytes| bytes > 0) {
                            continue;
                        }
                        let mut words = line.split_ascii_whitespace();
                        assert_eq!(words.next(), Some("GET"));
                        let target = words.next().unwrap().to_owned();
                        loop {
                            line.clear();
                            if !reader.read_line(&mut line).is_ok_and(|bytes| bytes > 0)
                                || line == "\r\n"
                            {
                                break;
                            }
                        }
                        target
                    };
                    seen.lock().unwrap().push(target.clone());
                    let _ = stream.write_all(response(&target).as_bytes());
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if stopped.recv_timeout(Duration::from_millis(5)).is_ok() {
                        break;
                    }
                }
                Err(error) => panic!("{error}"),
            }
        });
        Self {
            origin,
            requests,
            stop,
            thread: Some(thread),
        }
    }

    fn grant(&self, path: &str) -> String {
        format!("{}{path}=127.0.0.1", self.origin)
    }

    fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for SsrHttpFixture {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        self.thread.take().unwrap().join().unwrap();
    }
}

fn http_ok(content_type: &str, body: &str) -> String {
    format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
}

#[test]
fn config_cli_proves_granted_network_declaration_css_and_payload_reads() {
    const XHTML: &str = r#"<cem-element xmlns="http://www.w3.org/1999/xhtml" tag="x-card" version="1.2.3"><template id="x-card" type="text/cem-ml">{style |```@import './style.css';```}{article | Network}</template></cem-element>"#;
    const CSS: &str = "article { color: rgb(1, 2, 3) }";
    const PAYLOAD: &str = "{item | External payload}";
    let new_style = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let change = new_style.clone();
    let server = SsrHttpFixture::start(move |path| match path {
        "/card.xhtml" => http_ok("application/xhtml+xml; charset=utf-8", XHTML),
        "/style.css" if change.load(std::sync::atomic::Ordering::Relaxed) => {
            http_ok("text/css", "article { color: rgb(2, 3, 4) }")
        }
        "/style.css" => http_ok("text/css", CSS),
        "/payload.cem" => http_ok("text/cem-ml", PAYLOAD),
        _ => "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
    });
    let root = fixture();
    let page = format!(
        r#"<!doctype html><html><head><base href="{}/"><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3" src="./card.xhtml#x-card"></cem-element><x-card><template type="text/cem-ml" data-cem-payload-src="./payload.cem"></template></x-card></body></html>"#,
        server.origin,
    );
    fs::write(root.join("page.html"), &page).unwrap();
    let declaration = server.grant("/card.xhtml");
    let css = server.grant("/style.css");
    let payload = server.grant("/payload.cem");
    let publish = |grants: &[&str]| {
        let args = grants
            .iter()
            .flat_map(|grant| ["--ssr-network-read", *grant])
            .collect::<Vec<_>>();
        let output = config_cli(&root, &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("page-out.json")).unwrap())
            .unwrap()
    };
    let first = publish(&[&declaration, &css, &payload]);
    assert_eq!(first["usage"]["resources"], 4);
    assert_eq!(
        first["usage"]["totalBytes"],
        page.len() + XHTML.len() + CSS.len() + PAYLOAD.len()
    );
    assert_eq!(
        server.requests(),
        ["/card.xhtml", "/payload.cem", "/style.css"]
    );
    let html = fs::read_to_string(root.join("page-out.html")).unwrap();
    assert!(html.contains("<article>Network</article>"), "{html}");
    assert!(html.contains(PAYLOAD), "{html}");
    assert!(html.contains("color:rgb(1, 2, 3)"), "{html}");
    let reordered = publish(&[&payload, &declaration, &css]);
    assert_eq!(
        first["renderInputIdentity"],
        reordered["renderInputIdentity"]
    );
    let changed_policy = publish(&[&payload, &declaration, &css, &server.grant("/unused.css")]);
    assert_ne!(
        first["renderInputIdentity"],
        changed_policy["renderInputIdentity"]
    );
    new_style.store(true, std::sync::atomic::Ordering::Relaxed);
    let changed_response = publish(&[&declaration, &css, &payload]);
    assert_ne!(first["renderInputIdentity"], changed_response["renderInputIdentity"]);
    assert_eq!(first["usage"]["totalBytes"], changed_response["usage"]["totalBytes"]);
    assert_eq!(server.requests().len(), 12);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_cli_refuses_unsafe_network_responses_without_replacing_outputs() {
    let server = SsrHttpFixture::start(|path| {
        match path {
        "/redirect.cem" => "HTTP/1.1 302 Found\r\nLocation: /target.cem\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        "/mime.cem" => http_ok("text/plain", "{item | Wrong MIME}"),
        "/encoded.cem" => "HTTP/1.1 200 OK\r\nContent-Type: text/cem-ml\r\nContent-Encoding: gzip\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        "/large.cem" => http_ok("text/cem-ml", &" ".repeat(32_769)),
        "/chunked.cem" => format!("HTTP/1.1 200 OK\r\nContent-Type: text/cem-ml\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n8001\r\n{}\r\n0\r\n\r\n", " ".repeat(32_769)),
        "/slow.cem" => {
            std::thread::sleep(std::time::Duration::from_millis(2500));
            http_ok("text/cem-ml", "{item | Too late}")
        }
        _ => http_ok("text/cem-ml", "{item | Fixture}"),
    }
    });
    let root = fixture();
    assert!(config_cli(&root, &[]).status.success());
    let names = [
        "page-out.html",
        "page-out.html.map",
        "page-out.json",
        "page-out.json.map",
    ];
    let previous = names.map(|name| fs::read(root.join(name)).unwrap());
    let publish_input = |path| {
        fs::write(root.join("page.html"), format!(
            r#"<!doctype html><html><head><base href="{}/"><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3"><template id="x-card" type="text/cem-ml">{{article | Network}}</template></cem-element><x-card><template type="text/cem-ml" data-cem-payload-src=".{path}"></template></x-card></body></html>"#,
            server.origin
        )).unwrap();
    };
    let refuse = |args: &[&str], message: &str| {
        let output = config_cli(&root, args);
        assert!(!output.status.success());
        let diagnostics = String::from_utf8_lossy(&output.stderr);
        assert!(diagnostics.contains(message), "{diagnostics}");
        assert_eq!(
            names.map(|name| fs::read(root.join(name)).unwrap()),
            previous
        );
    };

    publish_input("/target.cem");
    refuse(&[], "cem.ssr.resource_read");
    let target = server.grant("/target.cem");
    let redirect = server.grant("/redirect.cem");
    refuse(&["--ssr-network-read", &redirect], "exact URL grant");
    assert!(server.requests().is_empty());
    fs::create_dir(root.join("mirror")).unwrap();
    let mapping = format!("{}/={}", server.origin, root.join("mirror").display());
    refuse(
        &[
            "--resolver-read-map",
            &mapping,
            "--ssr-network-read",
            &target,
        ],
        "Cannot open mapped SSR resource",
    );
    assert!(server.requests().is_empty());

    for (path, diagnostic) in [
        ("/redirect.cem", "HTTP 200"),
        ("/mime.cem", "requested Content-Type"),
        ("/encoded.cem", "identity content encoding"),
        ("/large.cem", "byte budget"),
        ("/chunked.cem", "byte budget"),
    ] {
        publish_input(path);
        refuse(
            &[
                "--ssr-network-read",
                &server.grant(path),
                "--ssr-network-read",
                &target,
            ],
            diagnostic,
        );
    }
    publish_input("/slow.cem");
    let limits = params().map(|param| {
        if param.starts_with("wall-ms=") {
            "wall-ms=1800"
        } else {
            param
        }
    });
    let slow = config_page_with_params(
        &root,
        "page.html",
        r#"@content-type="text/html""#,
        &limits,
        &["--ssr-network-read", &server.grant("/slow.cem")],
    );
    assert!(!slow.status.success());
    let diagnostics = String::from_utf8_lossy(&slow.stderr);
    assert!(
        diagnostics.contains("Original SSR resource wall-time budget exhausted"),
        "{diagnostics}"
    );
    assert_eq!(
        names.map(|name| fs::read(root.join(name)).unwrap()),
        previous
    );
    assert_eq!(
        server.requests(),
        [
            "/redirect.cem",
            "/mime.cem",
            "/encoded.cem",
            "/large.cem",
            "/chunked.cem",
            "/slow.cem"
        ]
    );
    fs::remove_dir_all(root).unwrap();
}

fn external_declaration_fixture() -> (PathBuf, &'static str, &'static str) {
    let root = fixture();
    let page = r#"<!doctype html><html><head><base href="/assets/"><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3" src="./card.html#x-card"></cem-element><x-card></x-card></body></html>"#;
    let declaration = r#"<cem-element tag="x-card" version="1.2.3"><template id="x-card" type="text/cem-ml">{article | External}</template></cem-element>"#;
    fs::write(root.join("page.html"), page).unwrap();
    fs::create_dir(root.join("mirror")).unwrap();
    fs::write(root.join("mirror/card.html"), declaration).unwrap();
    (root, page, declaration)
}

#[test]
fn direct_cli_loads_mapped_external_declaration() {
    let (root, _, _) = external_declaration_fixture();
    let read_map = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let out = direct(&root, "text/html", &["--resolver-read-map", &read_map]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(html.contains("<article>External</article>"), "{html}");
    assert!(html.contains(r#"src="./card.html#x-card""#), "{html}");
    assert!(html.contains("data-cem-ssr-declaration="), "{html}");
    assert!(html.contains("data-cem-island="));
    assert!(!html.contains(root.join("mirror").to_str().unwrap()));

    let map: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("out.html.map")).unwrap()).unwrap();
    let spans = map["outputSpans"].as_array().unwrap();
    let text_start = html.find("External</article>").unwrap() as u64;
    let span = spans.iter().find(|span| {
        let range = &span["outputRange"];
        let start = range["start"].as_u64().unwrap_or(u64::MAX);
        let len = range["len"].as_u64().unwrap_or(0);
        start <= text_start && text_start - start < len
    });
    assert!(
        span.is_some_and(|span| span
            .to_string()
            .contains("https://ssr.test/assets/card.html")),
        "{span:?}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_cli_proves_declaration_bytes_and_preserves_exports_on_refusal() {
    let (root, page, declaration) = external_declaration_fixture();
    let read_map = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let read_args = ["--resolver-read-map", &read_map];
    let remote = root.join("mirror/card.html");
    let publish = || {
        let output = config_cli(&root, &read_args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("page-out.json")).unwrap())
            .unwrap()
    };
    let first = publish();
    assert_eq!(first["usage"]["resources"], 1);
    assert_eq!(first["usage"]["totalBytes"], page.len() + declaration.len());
    fs::write(&remote, declaration.replace("External", "New text")).unwrap();
    let second = publish();
    assert_ne!(first["renderInputIdentity"], second["renderInputIdentity"]);
    assert_eq!(first["usage"]["totalBytes"], second["usage"]["totalBytes"]);

    let paths = ["page-out.html", "page-out.html.map", "page-out.json"];
    let published = paths.map(|path| fs::read(root.join(path)).unwrap());
    let assert_refused = |output: std::process::Output, code: &str| {
        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(code), "{stderr}");
    };
    assert_refused(config_cli(&root, &[]), "cem.ssr.resource_read");
    for (bytes, code) in [
        (
            declaration.replace("1.2.3", "1.2.4"),
            "cem.ssr.external_declaration_identity",
        ),
        (
            declaration.replace("{article", "{style |```b {color:red}```}{article"),
            "cem.ssr.inline_template_unsupported",
        ),
        (
            format!("{declaration}<img src=\"./tracking.png\">"),
            "cem.html.external_resource_rejected",
        ),
        (" ".repeat(65_537), "cem.ssr.resource_read"),
    ] {
        fs::write(&remote, bytes).unwrap();
        assert_refused(config_cli(&root, &read_args), code);
    }
    fs::write(&remote, declaration).unwrap();
    fs::write(
        root.join("page.html"),
        page.replace("#x-card", "#unrelated"),
    )
    .unwrap();
    assert_refused(
        config_cli(&root, &read_args),
        "cem.ssr.external_declaration_unsupported",
    );
    fs::write(
        root.join("page.html"),
        page.replace("<x-card>", r#"<img src="./outside.png"><x-card>"#),
    )
    .unwrap();
    assert_refused(
        config_cli(&root, &read_args),
        "cem.html.external_resource_rejected",
    );
    assert_eq!(
        paths.map(|path| fs::read(root.join(path)).unwrap()),
        published
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_cli_admits_canonical_xhtml_and_resolves_declaration_styles_from_its_url() {
    let root = fixture();
    let page = r#"<!doctype html><html><head><base href="/assets/"><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3" scope="card" src="./components/card.xhtml#x-card"></cem-element><x-card><template><style>@import './instance.css';</style></template></x-card><x-card></x-card></body></html>"#;
    let xhtml = r#"<?xml version="1.0" encoding="UTF-8"?><cem-element xmlns="http://www.w3.org/1999/xhtml" tag="x-card" version="1.2.3" scope="card"><template id="x-card" type="text/cem-ml">{style @scope=card |```@import './styles/theme.css';```}{article | XHTML &amp; CSS}</template></cem-element>"#;
    let files = [
        ("components/card.xhtml", xhtml),
        (
            "components/styles/theme.css",
            "@import './nested/color.css'; article { padding: 3px }",
        ),
        (
            "components/styles/nested/color.css",
            "article { color: rgb(17, 34, 51) }",
        ),
        (
            "instance.css",
            "article { background-color: rgb(29, 47, 71) }",
        ),
    ];
    fs::write(root.join("page.html"), page).unwrap();
    fs::create_dir_all(root.join("mirror/components/styles/nested")).unwrap();
    for (path, bytes) in files {
        fs::write(root.join("mirror").join(path), bytes).unwrap();
    }
    let mapping = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let out = config_cli(&root, &["--resolver-read-map", &mapping]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("page-out.html")).unwrap();
    assert_eq!(
        html.matches("<article>XHTML &amp; CSS</article>").count(),
        2,
        "{html}"
    );
    assert_eq!(
        html.matches(r#"data-cem-declaration-style="shared""#)
            .count(),
        1,
        "{html}"
    );
    assert_eq!(
        html.matches(r#"data-cem-instance-style="0""#).count(),
        1,
        "{html}"
    );
    let hosts: Vec<_> = html
        .match_indices("<x-card ")
        .map(|(start, _)| html[start..].split_once('>').unwrap().0)
        .collect();
    assert_eq!(hosts.len(), 2, "{html}");
    assert!(
        hosts.iter().all(|tag| tag.contains(r#" scope="card""#)),
        "{html}"
    );
    for style in html.split("<style data-cem-").skip(1) {
        let css = style.split_once("</style>").unwrap().0;
        assert!(!css.contains("@import"), "{css}");
    }
    assert!(html.contains("@import './instance.css'"));
    assert_eq!(html.matches("data-cem-ssr-stylesheets=").count(), 2);
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("page-out.json")).unwrap()).unwrap();
    assert_eq!(report["usage"]["resources"], 6);
    assert_eq!(
        report["usage"]["totalBytes"],
        page.len() + files.iter().map(|(_, bytes)| bytes.len()).sum::<usize>()
    );
    let paths = [
        "page-out.html",
        "page-out.html.map",
        "page-out.json",
        "page-out.json.map",
    ];
    let previous = paths.map(|path| fs::read(root.join(path)).unwrap());
    let map: serde_json::Value = serde_json::from_slice(&previous[1]).unwrap();
    let offset = html.find("XHTML &amp; CSS</article>").unwrap() as u64;
    let source_offset = xhtml.find("XHTML").unwrap() as u64;
    assert!(
        map["outputSpans"].as_array().unwrap().iter().any(|span| {
            let range = &span["outputRange"];
            let start = range["start"].as_u64().unwrap_or(u64::MAX);
            let length = range["len"].as_u64().unwrap_or(0);
            start <= offset
                && offset - start < length
                && span["origin"]["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|frame| {
                        let range = &frame["span"]["ranges"];
                        frame["transform"]["uri"] == "https://ssr.test/assets/components/card.xhtml"
                            && range["start"] == source_offset
                            && range["len"] == "XHTML ".len()
                    })
        }),
        "{map}"
    );
    let reject = |code: &str| {
        let result = config_cli(&root, &["--resolver-read-map", &mapping]);
        assert!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains(code), "{error}");
        assert_eq!(
            paths.map(|path| fs::read(root.join(path)).unwrap()),
            previous
        );
    };
    let declaration = root.join("mirror/components/card.xhtml");
    for (source, code) in [
        (
            xhtml.replace("</template>", "</wrong>"),
            "cem.ssr.external_declaration_response",
        ),
        (
            xhtml.replace("1.2.3", "1.2.4"),
            "cem.ssr.external_declaration_identity",
        ),
        (
            xhtml.replace(r#"scope="card""#, r#"scope="other""#),
            "cem.ssr.external_declaration_identity",
        ),
        (
            xhtml.replace("<?xml", r#"<?xml-stylesheet href="external.css"?><?xml"#),
            "cem.ssr.external_declaration_response",
        ),
        (
            xhtml.replace(
                "<cem-element",
                r#"<!DOCTYPE cem-element SYSTEM "external.dtd"><cem-element"#,
            ),
            "cem.ssr.external_declaration_response",
        ),
    ] {
        fs::write(&declaration, source).unwrap();
        reject(code);
    }
    fs::write(&declaration, xhtml).unwrap();
    fs::remove_file(root.join("mirror/components/styles/nested/color.css")).unwrap();
    reject("cem.css.import");
    fs::remove_dir_all(root).unwrap();
}

fn external_payload_fixture() -> (PathBuf, &'static str, &'static str) {
    let root = fixture();
    let page = r#"<!doctype html><html><head><base href="/assets/"><title>SSR</title></head><body><cem-element tag="x-card" version="1.2.3"><template id="x-card" type="text/cem-ml">{article | {$ datadom.payload.children.targets.children}}</template></cem-element><x-card><template type="text/cem-ml" data-cem-payload-src="./payload.cem"></template></x-card></body></html>"#;
    let payload = "{item | External {$ 1 div 0}}";
    fs::write(root.join("page.html"), page).unwrap();
    fs::create_dir(root.join("mirror")).unwrap();
    fs::write(root.join("mirror/payload.cem"), payload).unwrap();
    (root, page, payload)
}

#[test]
fn direct_cli_loads_mapped_external_cem_payload() {
    let (root, _, payload) = external_payload_fixture();
    let read_map = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let out = direct(&root, "text/html", &["--resolver-read-map", &read_map]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(
        html.contains(&format!("<article>{payload}</article>")),
        "{html}"
    );
    assert!(html.contains("data-cem-ssr-payload="), "{html}");
    assert!(
        html.contains(r#"data-cem-external-payload="true""#),
        "{html}"
    );
    assert!(!html.contains("data-cem-payload-src="));
    assert!(!html.contains(root.join("mirror").to_str().unwrap()));
    let map: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("out.html.map")).unwrap()).unwrap();
    let spans = map["outputSpans"].as_array().unwrap();
    let occurrences = html
        .match_indices(payload)
        .map(|(offset, _)| offset as u64)
        .collect::<Vec<_>>();
    assert_eq!(occurrences.len(), 2, "{html}");
    for offset in occurrences {
        assert!(
            spans.iter().any(|span| {
                span["outputRange"]["start"] == offset
                    && span["outputRange"]["len"] == payload.len()
                    && span["origin"]["frames"].as_array().is_some_and(|frames| {
                        frames.iter().any(|frame| {
                            frame["transform"]["uri"] == "https://ssr.test/assets/payload.cem"
                                && frame["span"]["ranges"]["start"] == 0
                                && frame["span"]["ranges"]["len"] == payload.len()
                        })
                    })
            }),
            "{map}"
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_cli_accounts_for_each_payload_read_and_preserves_exports_on_late_refusal() {
    let (root, page, payload) = external_payload_fixture();
    let read_map = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let args = ["--resolver-read-map", &read_map];
    let host = r#"<x-card><template type="text/cem-ml" data-cem-payload-src="./payload.cem"></template></x-card>"#;
    let twice = page.replace(host, &format!("{host}{host}"));
    fs::write(root.join("page.html"), &twice).unwrap();
    let publish = || {
        let out = config_cli(&root, &args);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("page-out.json")).unwrap())
            .unwrap()
    };
    let first = publish();
    assert_eq!(first["usage"]["resources"], 2);
    assert_eq!(
        first["usage"]["totalBytes"],
        twice.len() + 2 * payload.len()
    );
    fs::write(
        root.join("mirror/payload.cem"),
        payload.replace("External", "Modified"),
    )
    .unwrap();
    let second = publish();
    assert_ne!(first["renderInputIdentity"], second["renderInputIdentity"]);

    let outputs = [
        "page-out.html",
        "page-out.html.map",
        "page-out.json",
        "page-out.json.map",
    ];
    let previous = outputs.map(|name| fs::read(root.join(name)).unwrap());
    let reject = |args: &[&str], code: &str| {
        let result = config_cli(&root, args);
        assert!(!result.status.success());
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(error.contains(code), "{error}");
        assert_eq!(
            outputs.map(|name| fs::read(root.join(name)).unwrap()),
            previous
        );
    };
    reject(&[], "cem.ssr.resource_read");
    let bad = root.join("mirror/bad.cem");
    fs::write(
        root.join("page.html"),
        page.replace(
            host,
            &format!("{host}{}", host.replace("payload.cem", "bad.cem")),
        ),
    )
    .unwrap();
    fs::write(&bad, "{item | Unclosed").unwrap();
    reject(&args, "cem.ssr.payload_source_invalid");
    fs::write(&bad, vec![b' '; 32_769]).unwrap();
    reject(&args, "cem.ssr.resource_read");
    fs::write(
        root.join("page.html"),
        page.replace("</template></x-card>", "{item | Mixed}</template></x-card>"),
    )
    .unwrap();
    reject(&args, "cem.ssr.payload_profile_unsupported");
    fs::write(
        root.join("page.html"),
        page.replace("payload.cem", "payload.cem?query"),
    )
    .unwrap();
    reject(&args, "cem.ssr.payload_url");
    fs::remove_dir_all(root).unwrap();
}

fn stylesheet_fixture() -> PathBuf {
    let root = fixture();
    let page = root.join("page.html");
    let text = fs::read_to_string(&page).unwrap();
    fs::write(
        page,
        text.replace(
            "{article | Published}",
            "{style |```@import './styles/theme.css';```}{article | Published}",
        ),
    )
    .unwrap();
    fs::create_dir_all(root.join("mirror/styles/nested")).unwrap();
    fs::write(
        root.join("mirror/styles/theme.css"),
        "@import './nested/color.css'; article { padding: 1px }",
    )
    .unwrap();
    fs::write(
        root.join("mirror/styles/nested/color.css"),
        "article { color: red }",
    )
    .unwrap();
    root
}

#[test]
fn direct_cli_loads_mapped_stylesheets_with_nested_public_bases() {
    let root = stylesheet_fixture();
    let read_map = format!("https://ssr.test/guide/={}", root.join("mirror").display());
    let out = direct(&root, "text/html", &["--resolver-read-map", &read_map]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(
        html.contains("color: red") || html.contains("color:red"),
        "{html}"
    );
    assert!(
        html.contains("padding: 1px") || html.contains("padding:1px"),
        "{html}"
    );
    assert!(
        !html.split("<style ").skip(1).any(|style| style
            .split("</style>")
            .next()
            .unwrap()
            .contains("@import")),
        "{html}"
    );
    assert!(
        !html.contains(root.join("mirror").to_str().unwrap()),
        "{html}"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn direct_cli_uses_authored_base_for_mapped_stylesheets() {
    let root = stylesheet_fixture();
    let page = root.join("page.html");
    let original = fs::read_to_string(&page).unwrap();
    fs::write(
        &page,
        original.replace("<head>", r#"<head><base href="/assets/">"#),
    )
    .unwrap();
    let read_map = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let out = direct(&root, "text/html", &["--resolver-read-map", &read_map]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("out.html")).unwrap();
    assert!(html.contains(r#"<base href="/assets/">"#), "{html}");
    assert!(
        html.contains("color: red") || html.contains("color:red"),
        "{html}"
    );

    for base in [
        r#"<base href="javascript:alert(1)">"#,
        r#"<base href="file:///assets/">"#,
        r#"<base href="https://user@ssr.test/">"#,
        r#"<base href="/assets/" target="_blank">"#,
        r#"<base href="/assets/"><base href="/other/">"#,
        r#"<base href="/assets/"><meta http-equiv="Content-Security-Policy" content="base-uri 'none'">"#,
        r#"<base href="/assets/"><script>throw 1</script>"#,
        r#"<base href="/assets/"><link rel="stylesheet" href="unadmitted.css">"#,
    ] {
        fs::write(&page, original.replace("<head>", &format!("<head>{base}"))).unwrap();
        let out = direct(&root, "text/html", &["--resolver-read-map", &read_map]);
        assert!(!out.status.success(), "{base}");
        assert_eq!(fs::read_to_string(root.join("out.html")).unwrap(), html);
    }
    fs::remove_dir_all(root).unwrap();
}
#[test]
fn direct_cli_refuses_coloring_without_replacing_existing_output() {
    let root = fixture();
    fs::write(root.join("out.html"), "existing").unwrap();
    let out = direct(&root, "text/html", &["--output-color-type", "html"]);
    assert!(!out.status.success());
    assert_eq!(
        fs::read_to_string(root.join("out.html")).unwrap(),
        "existing"
    );
    fs::remove_dir_all(root).unwrap();
}
fn config_cli(root: &std::path::Path, args: &[&str]) -> std::process::Output {
    config_page(root, "page.html", r#"@content-type="text/html""#, args)
}

fn config_page(
    root: &std::path::Path,
    source: &str,
    identity: &str,
    args: &[&str],
) -> std::process::Output {
    config_page_with_params(root, source, identity, &params(), args)
}

fn config_page_with_params(
    root: &std::path::Path,
    source: &str,
    identity: &str,
    params: &[&str],
    args: &[&str],
) -> std::process::Output {
    let mut bindings = String::new();
    for param in params {
        let (k, v) = param.split_once('=').unwrap();
        bindings.push_str(&format!("{{param @name=\"{k}\" @value='{v}'}}"));
    }
    let config = format!(
        r#"{{run | {{import @id=page @src="{source}" {identity} | {{transform @id=ready @src="ssr.cemt" @template-content-type="text/cem-ml" @entrypoint="ssr:render-page" | {bindings} {{export @id=html @out="page-out.html" @content-type="text/html"}} {{export @id=report @out="page-out.json" @content-type="application/vnd.cem.ssr-report+json"}} }} }} }}"#
    );
    fs::write(root.join("run.cem"), config).unwrap();
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .current_dir(root)
        .args(["transform", "--config", "run.cem"])
        .args(args)
        .output()
        .unwrap()
}

fn batch_fixture() -> PathBuf {
    let root = fixture();
    fs::create_dir(root.join("pages")).unwrap();
    for name in ["z", "a"] {
        fs::copy(
            root.join("page.html"),
            root.join(format!("pages/{name}.html")),
        )
        .unwrap();
    }
    root
}

fn batch_config(page_url: &str, report_path: &str) -> String {
    let bindings = params()
        .into_iter()
        .map(|param| {
            let (name, value) = param.split_once('=').unwrap();
            let value = if name == "page-url" { page_url } else { value };
            format!("{{param @name=\"{name}\" @value='{value}'}}")
        })
        .collect::<String>();
    format!(
        r#"{{run |
        {{import @id=pages @src="pages/*.html" @root="pages" |
          {{transform @id=ready @src="ssr.cemt" @template-content-type="text/cem-ml" @entrypoint="ssr:render-page" |
            {bindings}
            {{export @id=html @out="out/{page}.html" @content-type="text/html"}}
            {{export @id=report @out="{report_path}" @content-type="application/vnd.cem.ssr-report+json"}}
          }}
        }}
        }}"#,
        page = "{page}",
    )
}

fn batch_cli(root: &std::path::Path, page_url: &str, report_path: &str) -> std::process::Output {
    run_batch_config(root, batch_config(page_url, report_path), &[])
}

fn run_batch_config(
    root: &std::path::Path,
    config: String,
    extra: &[&str],
) -> std::process::Output {
    fs::write(root.join("batch.cem"), config).unwrap();
    Command::new(env!("CARGO_BIN_EXE_cem-ml"))
        .current_dir(root)
        .args(["transform", "--config", "batch.cem", "--source-map-summary"])
        .args(extra)
        .output()
        .unwrap()
}

#[test]
fn ssr_glob_fan_out_publishes_all_pages_and_bounds_each_read() {
    let root = batch_fixture();
    let page_url = "https://ssr.test/guide/{page}/";
    let report_path = "out/{page}.json";
    let run = || {
        let output = batch_cli(&root, page_url, report_path);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let summary = String::from_utf8_lossy(&output.stdout);
        assert!(summary.find("out/a.html.map").unwrap() < summary.find("out/z.html.map").unwrap());
        ["a", "z"].map(|page| {
            let html = fs::read(root.join(format!("out/{page}.html"))).unwrap();
            let report: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join(format!("out/{page}.json"))).unwrap())
                    .unwrap();
            assert_eq!(
                report["route"]["pageUrl"],
                format!("https://ssr.test/guide/{page}/")
            );
            assert!(report["route"]["sourceUri"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/pages/{page}.html")));
            (html, report["renderInputIdentity"].clone())
        })
    };
    assert_eq!(run(), run());
    let paths = [
        "a.html",
        "a.html.map",
        "a.json",
        "a.json.map",
        "z.html",
        "z.html.map",
        "z.json",
        "z.json.map",
    ];
    let outputs = || paths.map(|path| fs::read(root.join("out").join(path)).unwrap());
    let previous = outputs();

    let late = root.join("pages/z.html");
    let source = fs::read_to_string(&late).unwrap();
    fs::write(
        &late,
        source.replace(
            "{article | Published}",
            "{style |```@import 'missing.css';```}{article | Published}",
        ),
    )
    .unwrap();
    let failed = batch_cli(&root, page_url, report_path);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("missing.css"));
    assert_eq!(outputs(), previous);
    fs::File::create(late).unwrap().set_len(1_048_577).unwrap();
    let failed = batch_cli(&root, page_url, report_path);
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr)
        .contains("page byte budget exceeded before opening"));
    assert_eq!(outputs(), previous);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ssr_glob_bounds_total_acquisition_before_parsing_any_page() {
    let root = batch_fixture();
    for name in ["a", "b", "c", "d", "z"] {
        fs::File::create(root.join(format!("pages/{name}.html")))
            .unwrap()
            .set_len(16 * 1024 * 1024)
            .unwrap();
    }
    let config = batch_config("https://ssr.test/guide/{page}/", "out/{page}.json")
        .replace("1048576", "16777216");
    let output = run_batch_config(&root, config, &[]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\\', "/");
    assert!(
        stderr.contains("SSR batch source byte budget has 0 bytes left")
            && stderr.contains("pages/z.html"),
        "{stderr}"
    );
    assert!(!root.join("out").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn ssr_batch_rejects_routes_and_destinations_before_first_render() {
    let root = batch_fixture();
    let first = root.join("pages/a.html");
    let source = fs::read_to_string(&first).unwrap();
    fs::write(
        &first,
        source.replace(
            "{article | Published}",
            "{style |```@import 'must-not-read.css';```}{article | Published}",
        ),
    )
    .unwrap();
    fs::create_dir(root.join("out")).unwrap();
    fs::write(root.join("out/a.html"), "previous").unwrap();

    let duplicate = batch_cli(&root, "https://ssr.test/same/", "out/{page}.json");
    let stderr = String::from_utf8_lossy(&duplicate.stderr);
    assert!(
        !duplicate.status.success() && stderr.contains("cem.ssr.page_url_duplicate"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read.css"), "{stderr}");

    let late = root.join("pages/z.html");
    let page = fs::read_to_string(&late).unwrap();
    fs::write(
        &late,
        page.replace("<head>", r#"<head><base href="data:text/html,blocked">"#),
    )
    .unwrap();
    let invalid_base = batch_cli(&root, "https://ssr.test/guide/{page}/", "out/{page}.json");
    let stderr = String::from_utf8_lossy(&invalid_base.stderr);
    assert!(
        !invalid_base.status.success() && stderr.contains("cem.ssr.base_url_invalid"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read.css"), "{stderr}");
    fs::write(late, page).unwrap();

    let sidecar = batch_cli(
        &root,
        "https://ssr.test/guide/{page}/",
        "out/{page}.html.map",
    );
    let stderr = String::from_utf8_lossy(&sidecar.stderr);
    assert!(
        !sidecar.status.success() && stderr.contains("duplicated"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read.css"), "{stderr}");
    let report_collision = run_batch_config(
        &root,
        batch_config("https://ssr.test/guide/{page}/", "out/{page}.json"),
        &["--report-json", "out/a.json"],
    );
    let stderr = String::from_utf8_lossy(&report_collision.stderr);
    assert!(
        !report_collision.status.success() && stderr.contains("duplicated"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read.css"), "{stderr}");

    let nested = batch_cli(
        &root,
        "https://ssr.test/guide/{page}/",
        "out/{page}.html.map/ready.json",
    );
    let stderr = String::from_utf8_lossy(&nested.stderr);
    assert!(
        !nested.status.success() && stderr.contains("has another output as its parent"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read.css"), "{stderr}");

    let source_alias = batch_cli(&root, "https://ssr.test/guide/{page}/", "pages/{page}.html");
    let stderr = String::from_utf8_lossy(&source_alias.stderr);
    assert!(
        !source_alias.status.success() && stderr.contains("aliases a source file"),
        "{stderr}"
    );
    assert!(!stderr.contains("must-not-read.css"), "{stderr}");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("out", root.join("alias")).unwrap();
        let alias = batch_cli(&root, "https://ssr.test/guide/{page}/", "alias/{page}.html");
        let stderr = String::from_utf8_lossy(&alias.stderr);
        assert!(
            !alias.status.success() && stderr.contains("duplicated"),
            "{stderr}"
        );
        assert!(!stderr.contains("must-not-read.css"), "{stderr}");
    }
    assert_eq!(
        fs::read_to_string(root.join("out/a.html")).unwrap(),
        "previous"
    );
    assert_eq!(fs::read_dir(root.join("out")).unwrap().count(), 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn page_profiles_work_in_direct_and_config_cli() {
    let root = fixture();
    let declaration = r#"{cem-element @tag=x-card @version="1.2.3" | {template @id=x-card @type="text/cem-ml" |```{article | Published}```}}{x-card}"#;
    let cem = format!("{{html | {{head | {{title | CEM page}}}}{{body | {declaration}}}}}");
    let markdown = format!("# Markdown page\n\n```cem-ml\n{declaration}\n```\n");
    for (source, bytes, extra, config_identity) in [
        (
            "page.cem",
            cem.as_str(),
            &["--data-content-type", "application/vnd.cem.page+cem"][..],
            r#"@content-type="application/vnd.cem.page+cem""#,
        ),
        ("page.cem.md", markdown.as_str(), &[][..], ""),
    ] {
        fs::write(root.join(source), bytes).unwrap();
        for out in [
            direct_page(&root, source, "text/html", extra),
            config_page(&root, source, config_identity, &[]),
        ] {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        for dest in ["out.html", "page-out.html"] {
            let html = fs::read_to_string(root.join(dest)).unwrap();
            assert!(
                html.contains("<article>Published</article>") && html.contains("data-cem-island="),
                "{html}"
            );
            let map: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join(format!("{dest}.map"))).unwrap())
                    .unwrap();
            let spans = map["outputSpans"].as_array().expect("HTML output spans");
            assert!(spans.iter().any(|span| span.to_string().contains(source)));
            if source.ends_with(".cem.md") {
                assert!(spans.iter().any(|span| span.to_string().contains("#cem-ml-fence-")));
            }
        }
        let report: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join("page-out.json")).unwrap()).unwrap();
        assert_eq!(report["usage"]["totalBytes"], bytes.len());
        assert!(report["route"]["sourceUri"]
            .as_str()
            .unwrap()
            .ends_with(source));
    }
    let prior = fs::read(root.join("out.html")).unwrap();
    let prior_map = fs::read(root.join("out.html.map")).unwrap();
    fs::write(root.join("plain.md"), markdown).unwrap();
    let plain = direct_page(&root, "plain.md", "text/html", &[]);
    assert!(!plain.status.success());
    assert!(String::from_utf8_lossy(&plain.stderr).contains("opted-in CEM page profile"));
    let override_profile = direct_page(
        &root,
        "page.cem.md",
        "text/html",
        &["--data-content-type", "text/markdown"],
    );
    assert!(!override_profile.status.success());
    assert_eq!(fs::read(root.join("out.html")).unwrap(), prior);
    assert_eq!(fs::read(root.join("out.html.map")).unwrap(), prior_map);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_cli_shares_proof_for_named_html_and_report_exports_atomically() {
    let root = fixture();
    let out = config_cli(&root, &[]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(fs::read_to_string(root.join("page-out.html"))
        .unwrap()
        .contains("<article>Published</article>"));
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("page-out.json")).unwrap()).unwrap();
    assert_eq!(report["readiness"], "native-ready");
    assert_eq!(report["hydrationReady"], false);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_and_direct_cli_bound_page_acquisition_before_publication() {
    let root = stylesheet_fixture();
    let page = root.join("page.html");
    let text = fs::read_to_string(&page).unwrap();
    fs::write(
        &page,
        text.replace("<head>", r#"<head><base href="/assets/">"#),
    )
    .unwrap();
    let read_map = format!("https://ssr.test/assets/={}", root.join("mirror").display());
    let out = config_cli(&root, &["--resolver-read-map", &read_map]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(root.join("page-out.html")).unwrap();
    assert!(html.contains(r#"<base href="/assets/">"#), "{html}");
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("page-out.json")).unwrap()).unwrap();
    assert_eq!(
        report["route"]["documentBaseUrl"],
        "https://ssr.test/assets/"
    );
    assert!(report["route"]["baseSourceMap"].is_object());
    assert_eq!(report["usage"]["resources"], 3);

    fs::write(root.join("out.html"), "previous direct output").unwrap();
    let paths = ["page-out.html", "page-out.json", "out.html"];
    let previous = paths.map(|path| fs::read(root.join(path)).unwrap());
    fs::File::create(page).unwrap().set_len(1_048_577).unwrap();
    for out in [config_cli(&root, &[]), direct(&root, "text/html", &[])] {
        assert!(!out.status.success());
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(
            error.contains("Original SSR page byte budget exceeded before opening"),
            "{error}"
        );
    }
    assert_eq!(
        paths.map(|path| fs::read(root.join(path)).unwrap()),
        previous
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn config_cli_hashes_acquired_bytes_and_keeps_both_previous_exports_on_refusal() {
    let root = stylesheet_fixture();
    let report = |dir| {
        let mapping = format!("https://ssr.test/guide/={}", root.join(dir).display());
        let out = config_cli(&root, &["--resolver-read-map", &mapping]);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&fs::read(root.join("page-out.json")).unwrap())
            .unwrap()
    };
    let first = report("mirror");
    let input_bytes = [
        "page.html",
        "mirror/styles/theme.css",
        "mirror/styles/nested/color.css",
    ]
    .iter()
    .map(|path| fs::metadata(root.join(path)).unwrap().len())
    .sum::<u64>();
    assert_eq!(first["usage"]["totalBytes"], input_bytes);
    assert_eq!(first["usage"]["resources"], 3);
    fs::rename(root.join("mirror"), root.join("relocated")).unwrap();
    let moved = report("relocated");
    assert_eq!(first["renderInputIdentity"], moved["renderInputIdentity"]);
    let leaf = root.join("relocated/styles/nested/color.css");
    fs::write(&leaf, "article { color: blue }").unwrap();
    let changed = report("relocated");
    assert_ne!(first["renderInputIdentity"], changed["renderInputIdentity"]);
    let published =
        ["page-out.html", "page-out.json"].map(|path| fs::read(root.join(path)).unwrap());
    fs::write(leaf, b"\xff\xfe").unwrap();
    let mapping = format!(
        "https://ssr.test/guide/={}",
        root.join("relocated").display()
    );
    let failed = config_cli(&root, &["--resolver-read-map", &mapping]);
    assert!(!failed.status.success());
    let diagnostics = String::from_utf8_lossy(&failed.stderr);
    assert!(
        diagnostics.contains("cem.css.import_parse_failed"),
        "{diagnostics}"
    );
    assert_eq!(
        ["page-out.html", "page-out.json"].map(|path| fs::read(root.join(path)).unwrap()),
        published
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cli_refuses_unmapped_or_oversized_resources_without_replacing_output() {
    let root = stylesheet_fixture();
    fs::write(root.join("out.html"), "existing").unwrap();
    let unmapped = direct(&root, "text/html", &[]);
    assert!(!unmapped.status.success());
    let diagnostics = String::from_utf8_lossy(&unmapped.stderr);
    assert!(
        diagnostics.contains("unsupported resolver")
            && diagnostics.contains("https://ssr.test/guide/styles/theme.css"),
        "{diagnostics}"
    );
    let leaf = fs::File::create(root.join("mirror/styles/nested/color.css")).unwrap();
    leaf.set_len(1_048_576).unwrap();
    drop(leaf);
    let mapping = format!("https://ssr.test/guide/={}", root.join("mirror").display());
    let oversized = direct(&root, "text/html", &["--resolver-read-map", &mapping]);
    assert!(!oversized.status.success());
    let diagnostics = String::from_utf8_lossy(&oversized.stderr);
    assert!(
        diagnostics.contains("Original SSR resource byte budget exceeded"),
        "{diagnostics}"
    );
    let page = root.join("page.html");
    let text = fs::read_to_string(&page).unwrap();
    fs::write(
        page,
        text.replace("./styles/theme.css", "https://ssr.test/guide-lookalike.css"),
    )
    .unwrap();
    fs::write(root.join("mirror/-lookalike.css"), "article { color: red }").unwrap();
    let prefix = format!("https://ssr.test/guide={}", root.join("mirror").display());
    let lookalike = direct(&root, "text/html", &["--resolver-read-map", &prefix]);
    assert!(!lookalike.status.success());
    assert!(String::from_utf8_lossy(&lookalike.stderr).contains("unsupported resolver"));
    assert_eq!(
        fs::read_to_string(root.join("out.html")).unwrap(),
        "existing"
    );
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn cli_refuses_stylesheet_symlinks_outside_the_read_map_root() {
    let root = stylesheet_fixture();
    let leaf = root.join("mirror/styles/nested/color.css");
    let outside = root.join("outside.css");
    fs::rename(&leaf, &outside).unwrap();
    std::os::unix::fs::symlink(outside, leaf).unwrap();
    let mapping = format!("https://ssr.test/guide/={}", root.join("mirror").display());
    let refused = direct(&root, "text/html", &["--resolver-read-map", &mapping]);
    assert!(!refused.status.success());
    let diagnostics = String::from_utf8_lossy(&refused.stderr);
    assert!(
        diagnostics.contains("SSR resource escapes its read-map root"),
        "{diagnostics}"
    );
    assert!(!root.join("out.html").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_graph_export_leaves_all_destinations_unpublished() {
    for export in [
        r#"{export @id=bad @out="bad.json" @content-type="application/json"}"#,
        r#"{export @id=bad @out="bad.html" @content-type="text/html" @style-policy="omit"}"#,
    ] {
        let root = fixture();
        fs::write(root.join("page-out.html"), "existing").unwrap();
        let mut bindings = String::new();
        for param in params() {
            let (key, value) = param.split_once('=').unwrap();
            bindings.push_str(&format!("{{param @name='{key}' @value='{value}'}}"));
        }
        fs::write(root.join("run.cem"),format!(r#"{{run | {{import @id=page @src="page.html" @content-type="text/html" | {{transform @id=ready @src="ssr.cemt" @template-content-type="text/cem-ml" @entrypoint="ssr:render-page" | {bindings} {{export @id=html @out="page-out.html" @content-type="text/html"}} {export} }} }} }}"#)).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_cem-ml"))
            .current_dir(&root)
            .args(["transform", "--config", "run.cem"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert_eq!(
            fs::read_to_string(root.join("page-out.html")).unwrap(),
            "existing"
        );
        assert!(!root.join("bad.json").exists());
        assert!(!root.join("bad.html").exists());
        fs::remove_dir_all(root).unwrap();
    }
}
