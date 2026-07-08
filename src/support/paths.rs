use std::path::{Path, PathBuf};

pub(crate) fn resolve_path(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

pub(crate) fn display_path(repo_root: &Path, path: &Path) -> String {
    let display = path
        .strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    if display.is_empty() {
        ".".to_string()
    } else {
        display
    }
}
