use cem_ml::engine::{
    CemMlEngine, ConvertRequest, EngineContext, EngineInput, FormatIdentity, LayerFormat,
};
use cem_ml::legacy_custom_element::{convert_template_source, TEMPLATE_LANG};
use cem_ml::real::RealCemMlEngine;
use cem_ml::run_config::ScopeConfig;

fn cem_input(uri: &str, source: &str) -> EngineInput {
    EngineInput {
        uri: uri.to_owned(),
        bytes: source.as_bytes().to_vec(),
        from_format: None,
        identity: Some(FormatIdentity {
            content_type: Some("application/cem+xml".to_owned()),
            ..FormatIdentity::default()
        }),
        root_scope: ScopeConfig::default(),
    }
}

fn render_cem_html(uri: &str, source: &str) -> String {
    let response = RealCemMlEngine::new()
        .convert(ConvertRequest {
            input: cem_input(uri, source),
            to_format: LayerFormat::Html,
            preserve_source_offsets: false,
            context: EngineContext::default(),
            target: None,
            target_scope: ScopeConfig::default(),
            scheduler_scope_id: 0,
        })
        .expect("render CEM as HTML");
    assert!(
        response.diagnostics.is_empty(),
        "unexpected diagnostics rendering {uri}: {:?}",
        response.diagnostics
    );
    response.primary["content"]
        .as_str()
        .expect("HTML content string")
        .to_owned()
}

fn assert_xslt_output_parity(name: &str, xslt: &str, equivalent_cem: &str) {
    let lowered = convert_template_source(xslt);
    assert!(
        lowered.diagnostics.is_empty(),
        "unexpected XSLT lowering diagnostics for {name}: {:?}",
        lowered.diagnostics
    );
    assert_eq!(lowered.source, equivalent_cem);

    let lowered_html = render_cem_html(&format!("{name}.lowered.cem"), &lowered.source);
    let equivalent_html = render_cem_html(&format!("{name}.equivalent.cem"), equivalent_cem);
    assert_eq!(lowered_html, equivalent_html);
}

#[test]
fn xslt_adapter_output_matches_cem_for_login_shell() {
    assert_xslt_output_parity(
        "login-shell",
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><main class="login"><h1>Sign in</h1><form><button type="submit">Continue</button></form></main></xsl:template></xsl:stylesheet>"#,
        r#"{main @class="login" | {h1 | Sign in}{form | {button @type="submit" | Continue}}}"#,
    );
}

#[test]
fn xslt_adapter_output_matches_cem_for_profile_named_template() {
    assert_xslt_output_parity(
        "profile-named-template",
        r#"<xsl:stylesheet version="1.0"><xsl:template match="/"><section class="profile"><xsl:call-template name="row"><xsl:with-param name="label" select="'Display name'"/></xsl:call-template></section></xsl:template><xsl:template name="row"><p><xsl:value-of select="$label"/></p></xsl:template></xsl:stylesheet>"#,
        r#"{section @class="profile" | {p | "Display name"}}"#,
    );
}

#[test]
fn xslt_adapter_output_matches_cem_for_asset_list_apply_templates() {
    assert_xslt_output_parity(
        "asset-list-apply-templates",
        r#"<xsl:stylesheet version="1.0"><xsl:variable name="assets"><asset>Logo</asset><asset>Hero</asset></xsl:variable><xsl:template match="/"><ul><xsl:apply-templates select="exsl:node-set($assets)/*"/></ul></xsl:template><xsl:template match="asset"><li><xsl:value-of select="."/></li></xsl:template></xsl:stylesheet>"#,
        "{ul | {li | Logo}{li | Hero}}",
    );
}

#[test]
fn xslt_content_type_is_selected_by_the_template_adapter_registry() {
    let registry =
        cem_ml::transform_template::TransformTemplateAdapterRegistry::with_builtin_adapters();
    let selected = registry.select(&FormatIdentity {
        content_type: Some(TEMPLATE_LANG.to_owned()),
        ..FormatIdentity::default()
    });

    assert_eq!(
        selected,
        cem_ml::transform_template::TransformTemplateAdapterResolution::Matched(
            cem_ml::transform_template::TransformTemplateAdapterSelection {
                adapter_id: "xslt-template",
                kind: cem_ml::engine::TransformTemplateKind::Xslt,
            },
        )
    );
}
