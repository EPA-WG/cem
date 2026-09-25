//! Opt-in source bases for ordinary HTML navigation links. No module-map lookup or fetch.

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum NavigationLinkBase {
    #[default]
    Document,
    Source,
}

pub fn resolve_navigation_links(
    policy: NavigationLinkBase,
    source_url: &str,
    hrefs: &[String],
) -> Result<Vec<String>, String> {
    if policy == NavigationLinkBase::Document {
        return Ok(hrefs.to_vec());
    }
    hrefs
        .iter()
        .map(|authored| {
            let href = authored.trim_matches(|c: char| c.is_ascii_whitespace());
            let has_scheme = href.split_once(':').is_some_and(|(scheme, _)| {
                scheme
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphabetic)
                    && scheme
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.'))
            });
            // Empty and fragment-only links navigate the live document. Explicit
            // schemes retain their authored spelling and existing browser behavior.
            if href.is_empty() || href.starts_with('#') || has_scheme {
                return Ok(authored.clone());
            }
            let base = url::Url::parse(source_url)
                .map_err(|error| format!("invalid navigation source base: {error}"))?;
            base.join(href)
                .map(|url| url.to_string())
                .map_err(|error| format!("source-relative navigation link cannot resolve: {error}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(
        policy: NavigationLinkBase,
        base: &str,
        values: &[&str],
    ) -> Result<Vec<String>, String> {
        resolve_navigation_links(
            policy,
            base,
            &values
                .iter()
                .map(|value| value.to_string())
                .collect::<Vec<_>>(),
        )
    }

    #[test]
    fn document_is_default_and_preserves_authored_links() {
        assert_eq!(NavigationLinkBase::default(), NavigationLinkBase::Document);
        let values = [
            "../index.html",
            "page.html",
            "?page=2",
            "/home",
            "#local",
            "",
            " https://example.test/A ",
        ];
        assert_eq!(
            resolve(NavigationLinkBase::default(), "invalid base", &values).unwrap(),
            values
        );
    }

    #[test]
    fn source_resolves_relative_forms_without_touching_current_page_or_absolute_links() {
        let actual = resolve(
            NavigationLinkBase::Source,
            "https://source.test/final/docs/page.html?old=1",
            &[
                "../index.html",
                "next.html?q=two words#part",
                "/root",
                "//cdn.test/file",
                "?page=2",
                "#local",
                " \n#local\t",
                "",
                " \t",
                "HTTPS://Other.test/A",
                "mailto:a@example.test",
                "data:text/plain,hello",
            ],
        )
        .unwrap();
        assert_eq!(
            actual,
            [
                "https://source.test/final/index.html",
                "https://source.test/final/docs/next.html?q=two%20words#part",
                "https://source.test/root",
                "https://cdn.test/file",
                "https://source.test/final/docs/page.html?page=2",
                "#local",
                " \n#local\t",
                "",
                " \t",
                "HTTPS://Other.test/A",
                "mailto:a@example.test",
                "data:text/plain,hello",
            ]
        );
    }

    #[test]
    fn source_preserves_url_escaping_and_uses_the_final_base() {
        assert_eq!(
            resolve(
                NavigationLinkBase::Source,
                "https://cdn.test/redirected/component.html",
                &[
                    "./café.html?x=%2F&y=1#part",
                    "\n ./next.html\t",
                    "../../outside.html",
                ]
            )
            .unwrap(),
            [
                "https://cdn.test/redirected/caf%C3%A9.html?x=%2F&y=1#part",
                "https://cdn.test/redirected/next.html",
                "https://cdn.test/outside.html",
            ]
        );
    }

    #[test]
    fn unresolvable_source_links_fail_without_publishing_a_host_relative_fallback() {
        assert!(resolve(NavigationLinkBase::Source, "not a URL", &["next.html"]).is_err());
        assert!(resolve(
            NavigationLinkBase::Source,
            "mailto:a@example.test",
            &["next.html"]
        )
        .is_err());
        assert!(resolve(
            NavigationLinkBase::Source,
            "https://source.test/",
            &["//[invalid"]
        )
        .is_err());
    }
}
