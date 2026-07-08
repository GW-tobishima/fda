use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn canonicalize_existing(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|e| e.to_string())
}

pub(crate) fn canonicalize_existing_or_parent(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return fs::canonicalize(path).map_err(|e| e.to_string());
    }

    let mut missing_components = Vec::new();
    let mut ancestor = path;
    while !ancestor.exists() {
        let Some(name) = ancestor.file_name() else {
            break;
        };
        missing_components.push(name.to_os_string());
        ancestor = ancestor
            .parent()
            .ok_or_else(|| format!("no existing parent for {}", path.display()))?;
    }

    let mut canonical =
        fs::canonicalize(ancestor).map_err(|e| format!("{}: {e}", ancestor.display()))?;
    for component in missing_components.iter().rev() {
        canonical.push(component);
    }
    Ok(canonical)
}
