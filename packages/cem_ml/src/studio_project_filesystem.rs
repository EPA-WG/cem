//! Filesystem projection of immutable Studio resource addresses. Source bodies
//! and consumer references are never rewritten by this export/resolver plan.
use crate::studio_project::{validate_studio_project, StudioProject};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFile {
    pub resource_id: String,
    pub uri: String,
    pub path: String,
    pub content_type: String,
    pub schema: Option<String>,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectFilesystemPlan {
    pub root_uri: String,
    pub files: Vec<ProjectFile>,
}

pub fn plan_project_filesystem(project: &StudioProject) -> Result<ProjectFilesystemPlan, String> {
    validate_studio_project(project).map_err(|error| error.to_string())?;
    let root = url::Url::parse(&project.root_uri).map_err(|error| error.to_string())?;
    let mut identities = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for resource in &project.resources {
        let uri = root
            .join(resource.reference_path.as_deref().unwrap_or(&resource.path))
            .map_err(|error| error.to_string())?
            .to_string();
        if !uri.starts_with(&project.root_uri) || !identities.insert(uri.clone()) {
            return Err("cem.studio.filesystem.ambiguous_address".into());
        }
        if resource.path == "project.cem"
            || resource.path.starts_with("project.cem/")
            || project
                .resources
                .iter()
                .any(|other| other.path.starts_with(&format!("{}/", resource.path)))
        {
            return Err("cem.studio.filesystem.path_conflict".into());
        }
        files.push(ProjectFile {
            resource_id: resource.id.clone(),
            uri,
            path: resource.path.clone(),
            content_type: resource.content_type.clone(),
            schema: resource.schema.clone(),
            sha256: resource.sha256.clone(),
        });
    }
    Ok(ProjectFilesystemPlan {
        root_uri: project.root_uri.clone(),
        files,
    })
}

impl ProjectFilesystemPlan {
    pub fn read_uri(&self, value: &str) -> Result<String, String> {
        if value == "-"
            || std::path::Path::new(value).is_absolute()
            || crate::resolver::is_windows_drive_path(value)
        {
            return Ok(value.into());
        }
        if let Some(scheme) = crate::resolver::uri_scheme(value) {
            if !matches!(scheme, "studio" | "cem-studio") {
                return Ok(value.into());
            }
        }
        let uri = url::Url::parse(&self.root_uri)
            .and_then(|root| root.join(value))
            .map_err(|error| error.to_string())?
            .to_string();
        if self.files.iter().any(|file| file.uri == uri) {
            return Ok(uri);
        }
        Err(format!("cem.studio.filesystem.unresolved: `{value}` is not an original project resource address"))
    }

    pub fn file(&self, uri: &str) -> Option<&ProjectFile> {
        let uri = url::Url::parse(uri).ok()?.to_string();
        self.files.iter().find(|file| file.uri == uri)
    }

    pub fn output_uri(&self, value: &str) -> Result<String, String> {
        let uri = url::Url::parse(&self.root_uri)
            .and_then(|root| root.join(value))
            .map_err(|error| error.to_string())?
            .to_string();
        self.output_path(&uri)?;
        Ok(uri)
    }

    pub fn output_path(&self, uri: &str) -> Result<String, String> {
        let suffix = uri
            .strip_prefix(&self.root_uri)
            .ok_or("cem.studio.filesystem.output_outside_project")?;
        let path = crate::resolver::percent_decode_uri_path(suffix)
            .ok_or("cem.studio.filesystem.output_path_invalid")?;
        crate::studio_project::validate_resource_path(&path, "output")
            .map_err(|error| error.to_string())?;
        if path == "project.cem"
            || self
                .files
                .iter()
                .any(|file| file.uri == uri || file.path == path)
        {
            return Err("cem.studio.filesystem.output_source_conflict".into());
        }
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::studio_project::{
        parse_studio_project, STUDIO_PROJECT_JSON_CONTENT_TYPE, STUDIO_PROJECT_SCHEMA_URI,
    };
    fn project() -> StudioProject {
        parse_studio_project(
            include_bytes!(
                "../schema-packages/studio-project/v1/examples/feature-tour.project.json"
            ),
            STUDIO_PROJECT_JSON_CONTENT_TYPE,
            STUDIO_PROJECT_SCHEMA_URI,
        )
        .unwrap()
    }
    #[test]
    fn filesystem_plan_preserves_original_owners_after_moves() {
        let mut project = project();
        project.resources[0].path = "renamed/it's $(touch nope).cem".into();
        project.resources[1].path = "moved/command.json".into();
        let before = project.clone();
        let plan = plan_project_filesystem(&project).unwrap();
        assert_eq!(plan.files[0].resource_id, "tour-source");
        assert_eq!(plan.files[0].uri, "studio://feature-tour/data/tour.cem");
        assert_eq!(plan.files[0].path, project.resources[0].path);
        assert_eq!(plan.read_uri("data/tour.cem").unwrap(), plan.files[0].uri);
        assert_eq!(
            plan.read_uri("./config/validate.json").unwrap(),
            plan.files[1].uri
        );
        assert_eq!(
            plan.read_uri(&plan.files[1].uri).unwrap(),
            plan.files[1].uri
        );
        assert_eq!(project, before);
    }
    #[test]
    fn filesystem_plan_refuses_unresolved_and_cross_project_addresses() {
        let plan = plan_project_filesystem(&project()).unwrap();
        for value in [
            "missing.cem",
            "studio://other/data/tour.cem",
            "cem-studio://other/input.cem",
        ] {
            assert!(plan.read_uri(value).is_err(), "{value}");
        }
        assert_eq!(plan.read_uri("-").unwrap(), "-");
        assert_eq!(
            plan.read_uri("https://example.test/input.cem").unwrap(),
            "https://example.test/input.cem"
        );
    }
    #[test]
    fn filesystem_plan_uses_reference_addresses_when_display_paths_swap() {
        let mut project = project();
        project.resources.swap(0, 1);
        project.resources[0].path = "data/tour.cem".into();
        project.resources[1].path = "config/validate.json".into();
        let plan = plan_project_filesystem(&project).unwrap();
        assert_eq!(
            plan.read_uri("config/validate.json").unwrap(),
            "studio://feature-tour/config/validate.json"
        );
        assert_eq!(plan.files[0].path, "data/tour.cem");
        project.resources[1].path = project.resources[0].path.clone();
        assert!(plan_project_filesystem(&project).is_err());
    }
}
