//! Closed browser output from the original native application and island.
use super::*;
use crate::{application_runtime, render::RenderPlanNode};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ApplicationBrowserBehavior {
    NavigationLandmark,
    FormControl,
}

impl ApplicationBrowserBehavior {
    fn capability(self) -> &'static str {
        match self {
            Self::NavigationLandmark => "navigation-landmark",
            Self::FormControl => "form-control",
        }
    }

    fn identity(self) -> &'static str {
        match self {
            Self::NavigationLandmark => "cem-elements-navigation-landmark-v1",
            Self::FormControl => "cem-elements-form-control-v4",
        }
    }
}

impl PreparedApplication {
    /// An original Rust bootstrap explicitly selects the behavior for this
    /// publication's verified declaration. The envelope only records demand.
    pub fn with_browser_behavior(
        mut self,
        tag: &str,
        behavior: ApplicationBrowserBehavior,
    ) -> Result<Self, String> {
        let demand = self
            .staged
            .as_ref()
            .ok_or(INVALID)?
            .envelope()
            .requirements()
            .declarations
            .get(tag)
            .ok_or(INVALID)?;
        if demand.capability.as_deref() != Some(behavior.capability())
            || !demand.reviewed_browser_behavior()
            || self
                .browser_behaviors
                .insert(tag.into(), behavior)
                .is_some()
        {
            return Err(INVALID.into());
        }
        Ok(self)
    }
}

impl ApplicationSourceSession {
    /// A separate browser route pairs the exact declaration with its compiled
    /// output. Ordinary render and projection still refuse browser demand.
    pub fn render_browser_island_json(
        &mut self,
        tag: &str,
        bytes: &[u8],
        instance: &str,
    ) -> Result<String, String> {
        let _entry = self.enter()?;
        let behavior = ApplicationBrowserBehavior::NavigationLandmark;
        let identity = self.browser_declaration(tag, behavior)?;
        let html = self.frame(
            tag,
            instance,
            Some(behavior.capability()),
            |control| application_runtime::island::bind(bytes, instance, control),
            |artifact, data, control| {
                let plan = crate::render::project_render_plan_with_control(
                    &application_runtime::render_plan_data(artifact, data, control)?,
                    crate::eval::QueryContextScope(0),
                    &application_runtime::inputs::LIMITS,
                    control,
                    ROOT_EXECUTION_SCOPE_ID,
                ).map_err(|_| INVALID)?;
                if !plan.host_attribute_updates.is_empty()
                    || !matches!(plan.nodes.as_slice(), [RenderPlanNode::Element { tag, attributes, .. }]
                        if tag == "nav" && attributes.iter().any(|a| a.name == "part" && a.value.split_ascii_whitespace().any(|part| part == "root")))
                    || !closed_navigation(&plan.nodes, 0, &mut 256)
                {
                    return Err(INVALID.into());
                }
                Ok(crate::render::render_plan_to_html_with_control(
                    &plan, control, ROOT_EXECUTION_SCOPE_ID,
                ).map_err(|_| INVALID)?.rendered)
            },
        )?;
        let receipt = serde_json::json!({
            "tag": tag, "instance": instance, "behaviorIdentity": behavior.identity(),
            "declarationIdentity": identity, "html": html,
        })
        .to_string();
        if receipt.len() > application_runtime::MAX_INPUT_BYTES {
            return Err(INVALID.into());
        }
        self.check()?;
        Ok(receipt)
    }

    /// Rust-only ingress for a text provider on this exact native publication.
    pub fn register_browser_receiver_input(
        &mut self,
        tag: &str,
        root: crate::eval::RetainedCemNode,
        instance: &str,
    ) -> Result<ApplicationReceiverInput, String> {
        if !super::frames::valid_instance(instance) {
            return Err(INVALID.into());
        }
        self.browser_declaration(tag, ApplicationBrowserBehavior::FormControl)?;
        let mut input = self.register_receiver_input(tag, root)?;
        input.browser_instance = Some(instance.into());
        Ok(input)
    }

    /// The provider consumes a retained receiver and its admitted reference
    /// scopes. A caller cannot replace that source with an island or JS data.
    pub fn render_browser_receiver_json(
        &mut self,
        input: &ApplicationReceiverInput,
        instance: &str,
    ) -> Result<String, String> {
        let _entry = self.enter()?;
        if input.browser_instance.as_deref() != Some(instance) {
            return Err(INVALID.into());
        }
        let behavior = ApplicationBrowserBehavior::FormControl;
        let identity = self.browser_declaration(&input.tag, behavior)?;
        let (plan, control) = self.receiver_plan(input, instance, Some(behavior.capability()))?;
        let plan = crate::render::project_render_plan_with_control(
            &plan,
            crate::eval::QueryContextScope(0),
            &application_runtime::inputs::LIMITS,
            &control,
            ROOT_EXECUTION_SCOPE_ID,
        )
        .map_err(|_| INVALID)?;
        let value = closed_text_control(&plan, instance)?;
        let html = crate::render::render_plan_to_html_with_control(
            &plan,
            &control,
            ROOT_EXECUTION_SCOPE_ID,
        )
        .map_err(|_| INVALID)?
        .rendered;
        let receipt = serde_json::json!({
            "tag": input.tag, "instance": instance, "behaviorIdentity": behavior.identity(),
            "declarationIdentity": identity, "html": html, "value": value,
        })
        .to_string();
        if receipt.len() > application_runtime::MAX_INPUT_BYTES {
            return Err(INVALID.into());
        }
        self.check()?;
        Ok(receipt)
    }

    fn browser_declaration(
        &self,
        tag: &str,
        behavior: ApplicationBrowserBehavior,
    ) -> Result<String, String> {
        if self.prepared.browser_behaviors.get(tag) != Some(&behavior) {
            return Err(INVALID.into());
        }
        Ok(self
            .prepared
            .current()?
            .envelope()
            .requirements()
            .declarations
            .get(tag)
            .ok_or(INVALID)?
            .metadata_hash
            .clone())
    }
}

fn closed_text_control<'a>(
    plan: &'a crate::render::RenderPlan,
    instance: &str,
) -> Result<&'a str, String> {
    let [RenderPlanNode::Element {
        tag,
        namespace,
        qualified_name,
        attributes,
        children,
        ..
    }] = plan.nodes.as_slice()
    else {
        return Err(INVALID.into());
    };
    if !plan.host_attribute_updates.is_empty()
        || tag != "input"
        || namespace
            .as_deref()
            .is_some_and(|ns| ns != "http://www.w3.org/1999/xhtml")
        || qualified_name.as_deref().is_some_and(|name| name != tag)
        || !children.is_empty()
        || attributes.iter().any(|a| {
            a.namespace.is_some()
                || a.qualified_name
                    .as_ref()
                    .is_some_and(|name| name != &a.name)
                || !matches!(
                    a.name.as_str(),
                    "part" | "type" | "id" | "value" | "aria-label"
                )
        })
    {
        return Err(INVALID.into());
    }
    let attribute = |name| {
        let mut matches = attributes.iter().filter(|a| a.name == name);
        let value = matches.next()?.value.as_str();
        matches.next().is_none().then_some(value)
    };
    if attribute("type") != Some("text")
        || attribute("part") != Some("control")
        || attribute("id") != Some(instance)
        || attribute("aria-label").is_none_or(|label| label.trim().is_empty())
    {
        return Err(INVALID.into());
    }
    attribute("value")
        .filter(|value| !value.contains(['\r', '\n', '\0']))
        .ok_or_else(|| INVALID.into())
}

// The first behavior has no provider, URL or action grant. Review the native
// output as well as its static demand: references from an island can otherwise
// introduce elements and attributes absent from the compiled declaration.
fn closed_navigation(nodes: &[RenderPlanNode], depth: usize, remaining: &mut usize) -> bool {
    if depth > 16 || nodes.len() > *remaining {
        return false;
    }
    *remaining -= nodes.len();
    nodes.iter().all(|node| match node {
        RenderPlanNode::Text { .. } => true,
        RenderPlanNode::Element {
            tag,
            namespace,
            qualified_name,
            attributes,
            children,
            ..
        } => {
            (if depth == 0 {
                tag == "nav"
            } else {
                matches!(tag.as_str(), "button" | "div" | "span")
            }) && namespace
                .as_deref()
                .is_none_or(|ns| ns == "http://www.w3.org/1999/xhtml")
                && qualified_name.as_ref().is_none_or(|name| name == tag)
                && attributes.iter().all(|a| {
                    a.namespace.is_none()
                        && a.qualified_name.as_ref().is_none_or(|name| name == &a.name)
                        && match a.name.as_str() {
                            "class" | "part" | "role" | "id" | "aria-label" | "aria-current"
                            | "aria-disabled" => true,
                            "disabled" => tag == "button",
                            "type" => tag == "button" && a.value == "button",
                            _ => false,
                        }
                })
                && (tag != "button" || attributes.iter().any(|a| a.name == "type"))
                && closed_navigation(children, depth + 1, remaining)
        }
        _ => false,
    })
}
