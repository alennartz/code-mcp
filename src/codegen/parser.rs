use std::path::Path;

use anyhow::{Context, Result};
use openapiv3::OpenAPI;

/// Load an OpenAPI spec from a local YAML or JSON file.
pub fn load_spec_from_file(path: &Path) -> Result<OpenAPI> {
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read {}", path.display()))?;

    // Try YAML first (which is a superset of JSON), then fall back to JSON
    let spec: OpenAPI = serde_yaml::from_str(&content)
        .or_else(|_| serde_json::from_str(&content))
        .with_context(|| format!("Failed to parse OpenAPI spec from {}", path.display()))?;

    Ok(spec)
}

/// Fetch and parse an OpenAPI spec from a URL.
pub async fn load_spec_from_url(url: &str) -> Result<OpenAPI> {
    let response = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to fetch spec from {url}"))?;

    let content = response
        .text()
        .await
        .with_context(|| format!("Failed to read response body from {url}"))?;

    let spec: OpenAPI = serde_yaml::from_str(&content)
        .or_else(|_| serde_json::from_str(&content))
        .with_context(|| format!("Failed to parse OpenAPI spec from {url}"))?;

    Ok(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_spec_from_file() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        assert_eq!(spec.info.title, "Petstore");
        assert!(!spec.paths.paths.is_empty());
    }

    #[test]
    fn test_load_spec_from_file_info() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        assert_eq!(spec.info.version, "1.0.0");
        assert!(spec.info.description.is_some());
    }

    #[test]
    fn test_load_spec_from_file_servers() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        assert_eq!(spec.servers.len(), 1);
        assert_eq!(spec.servers[0].url, "https://petstore.example.com/v1");
    }

    #[test]
    fn test_load_spec_from_file_paths() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        assert!(spec.paths.paths.contains_key("/pets"));
        assert!(spec.paths.paths.contains_key("/pets/{petId}"));
    }

    #[test]
    fn test_load_spec_from_file_schemas() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        let components = spec.components.as_ref().unwrap();
        assert!(components.schemas.contains_key("Pet"));
        assert!(components.schemas.contains_key("NewPet"));
    }

    #[test]
    fn test_load_spec_from_file_security() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        let components = spec.components.as_ref().unwrap();
        assert!(components.security_schemes.contains_key("bearerAuth"));
    }

    #[test]
    fn test_load_spec_from_file_tags() {
        let spec = load_spec_from_file(Path::new("testdata/petstore.yaml")).unwrap();
        assert_eq!(spec.tags.len(), 1);
        assert_eq!(spec.tags[0].name, "pets");
    }

    #[test]
    fn test_load_spec_nonexistent_file() {
        let result = load_spec_from_file(Path::new("testdata/nonexistent.yaml"));
        assert!(result.is_err());
    }
}
