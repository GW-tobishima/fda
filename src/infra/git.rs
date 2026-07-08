use std::fs;
use std::path::Path;

pub(crate) fn repo_project_name(repo_root: &Path) -> String {
    repo_project_name_from_git_worktree_file(repo_root).unwrap_or_else(|| {
        repo_root
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown-repo")
            .to_string()
    })
}

fn repo_project_name_from_git_worktree_file(repo_root: &Path) -> Option<String> {
    let content = fs::read_to_string(repo_root.join(".git")).ok()?;
    let gitdir = content.trim().strip_prefix("gitdir:")?.trim();
    let gitdir_path = Path::new(gitdir);
    let gitdir_path = if gitdir_path.is_absolute() {
        gitdir_path.to_path_buf()
    } else {
        repo_root.join(gitdir_path)
    };
    let worktrees_dir = gitdir_path.parent()?;
    if worktrees_dir.file_name().and_then(|value| value.to_str()) != Some("worktrees") {
        return None;
    }
    let dot_git_dir = worktrees_dir.parent()?;
    if dot_git_dir.file_name().and_then(|value| value.to_str()) != Some(".git") {
        return None;
    }
    dot_git_dir
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
}
