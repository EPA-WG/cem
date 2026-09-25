//! Explicit WPT JSON fixture boundary; isolated dependency research only.
#[unsafe(no_mangle)]
pub extern "C" fn parser_probe() -> u32 {
    let mut counts = 0;
    let fixtures: serde_json::Value =
        serde_json::from_str(include_str!("../../wpt-url-parse.json")).unwrap();
    assert_eq!(fixtures.as_array().unwrap().len(), 51);
    for name in ["upstream", "ada"] {
        let mut failed = 0;
        for row in fixtures.as_array().unwrap() {
            let c = &row["case"];
            let input = c["input"].as_str().unwrap();
            let base = c["base"].as_str();
            let result: Option<Vec<String>> = if name == "upstream" {
                let base = base.map(url::Url::parse).transpose().unwrap();
                url::Url::options()
                    .base_url(base.as_ref())
                    .parse(input)
                    .ok()
                    .map(|u| {
                        vec![
                            url::quirks::href(&u).to_string(),
                            url::quirks::origin(&u).to_string(),
                            url::quirks::protocol(&u).to_string(),
                            url::quirks::username(&u).to_string(),
                            url::quirks::password(&u).to_string(),
                            url::quirks::host(&u).to_string(),
                            url::quirks::hostname(&u).to_string(),
                            url::quirks::port(&u).to_string(),
                            url::quirks::pathname(&u).to_string(),
                            url::quirks::search(&u).to_string(),
                            url::quirks::hash(&u).to_string(),
                        ]
                    })
            } else {
                ada_url::Url::parse(input, base).ok().map(|u| {
                    vec![
                        u.href().to_string(),
                        u.origin().to_string(),
                        u.protocol().to_string(),
                        u.username().to_string(),
                        u.password().to_string(),
                        u.host().to_string(),
                        u.hostname().to_string(),
                        u.port().to_string(),
                        u.pathname().to_string(),
                        u.search().to_string(),
                        u.hash().to_string(),
                    ]
                })
            };
            let mut differences = Vec::new();
            match result {
                None => {
                    if c["failure"] != true {
                        differences.push("unexpected rejection".to_string());
                    }
                }
                Some(values) => {
                    if c["failure"] == true {
                        differences.push("unexpected acceptance".to_string());
                    }
                    for (field, actual) in [
                        "href", "origin", "protocol", "username", "password", "host", "hostname",
                        "port", "pathname", "search", "hash",
                    ]
                    .iter()
                    .zip(values)
                    {
                        if let Some(expected) = c[field].as_str() {
                            if actual != expected {
                                differences.push(format!("{field}: {expected:?} != {actual:?}"));
                            }
                        }
                    }
                }
            }
            if !differences.is_empty() {
                failed += 1;
                #[cfg(not(target_arch = "wasm32"))]
                println!(
                    "{name} {}: {}",
                    row["upstream_index"],
                    differences.join("; ")
                );
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        println!("{name}: {failed}/51 cases differ");
        counts = (counts << 16) | failed;
    }
    counts
}
