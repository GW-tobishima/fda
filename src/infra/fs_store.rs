use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use crate::application::ports::ArtifactStore;

pub(crate) struct FsArtifactStore;

pub(crate) fn list_file_names(path: &Path) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path)
        .map_err(|e| format!("failed to read artifact dir {}: {e}", path.display()))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_file() {
            files.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    files.sort();
    Ok(files)
}

pub(crate) fn list_dir_names(path: &Path) -> Result<Vec<String>, String> {
    let mut dirs = Vec::new();
    for entry in fs::read_dir(path)
        .map_err(|e| format!("failed to read runs dir {}: {e}", path.display()))?
    {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.path().is_dir() {
            dirs.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    dirs.sort();
    Ok(dirs)
}

impl ArtifactStore for FsArtifactStore {
    fn canonicalize(&self, path: &Path) -> Result<PathBuf, String> {
        fs::canonicalize(path).map_err(|e| e.to_string())
    }

    fn create_dir_all(&self, path: &Path) -> Result<(), String> {
        fs::create_dir_all(path).map_err(|e| e.to_string())
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn copy(&self, src: &Path, dst: &Path) -> Result<(), String> {
        fs::copy(src, dst)
            .map(|_| ())
            .map_err(|e| format!("failed to copy {} to {}: {e}", src.display(), dst.display()))
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))
    }

    fn write_text(&self, path: &Path, body: &str) -> Result<(), String> {
        fs::write(path, body)
            .map_err(|e| format!("failed to write text file {}: {e}", path.display()))
    }

    fn write_json(&self, path: &Path, value: &Value) -> Result<(), String> {
        let body = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
        fs::write(path, format!("{body}\n"))
            .map_err(|e| format!("failed to write JSON file {}: {e}", path.display()))
    }

    fn schema_files(&self, schema_dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();
        for entry in fs::read_dir(schema_dir)
            .map_err(|e| format!("failed to read schema dir {}: {e}", schema_dir.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".schema.json"))
            {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    fn yaml_files(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut files = Vec::new();
        for entry in fs::read_dir(dir)
            .map_err(|e| format!("failed to read YAML dir {}: {e}", dir.display()))?
        {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "yaml" | "yml"))
            {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }
}
