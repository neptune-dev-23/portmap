use std::path::PathBuf;

pub fn infer_project_name(cwd: &str) -> Option<String> {
    let manifests = ["package.json", "Cargo.toml", "pyproject.toml"];
    let mut current = PathBuf::from(cwd);

    loop {
        for manifest in &manifests {
            let candidate = current.join(manifest);
            if candidate.exists() {
                if let Some(name) = extract_name(&candidate, manifest) {
                    return Some(name);
                }
            }
        }
        if !current.pop() {
            break;
        }
    }
    None
}

fn extract_name(path: &PathBuf, manifest: &str) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;

    match manifest {
        "package.json" => {
            let v: serde_json::Value = serde_json::from_str(&content).ok()?;
            v["name"].as_str().map(|s| s.to_string())
        }
        "Cargo.toml" => {
            let v: toml::Value = toml::from_str(&content).ok()?;
            v.get("package")?
                .get("name")?
                .as_str()
                .map(|s| s.to_string())
        }
        "pyproject.toml" => {
            let v: toml::Value = toml::from_str(&content).ok()?;
            // PEP 621 [project] or Poetry [tool.poetry]
            v.get("project")
                .and_then(|p| p.get("name"))
                .or_else(|| {
                    v.get("tool")
                        .and_then(|t| t.get("poetry"))
                        .and_then(|p| p.get("name"))
                })
                .and_then(|n| n.as_str())
                .map(|s| s.to_string())
        }
        _ => None,
    }
}
