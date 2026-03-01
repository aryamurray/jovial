use std::collections::HashMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// A set of discovered Java source files.
pub struct SourceSet {
    /// Absolute paths to .java files.
    pub files: Vec<PathBuf>,
    /// Maps each file to the source root it was found under.
    pub root_for_file: HashMap<PathBuf, PathBuf>,
}

/// Recursively discover Java source files under the given source roots.
///
/// Skips test directories, test files, package-info.java, and module-info.java.
pub fn discover_sources(
    project_root: &Path,
    source_roots: &[String],
) -> anyhow::Result<SourceSet> {
    let mut files = Vec::new();
    let mut root_for_file = HashMap::new();

    for root_rel in source_roots {
        let root = project_root.join(root_rel);
        if !root.exists() {
            log::warn!("source root does not exist: {}", root.display());
            continue;
        }

        for entry in WalkDir::new(&root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }

            let path = entry.path();

            // Only .java files
            let ext = path.extension().and_then(|e| e.to_str());
            if ext != Some("java") {
                continue;
            }

            // Skip test directories
            let path_str = path.to_string_lossy();
            let normalized = path_str.replace('\\', "/");
            if normalized.contains("/src/test/") || normalized.contains("/test/") {
                continue;
            }

            // Skip test files and meta files
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with("Test.java")
                    || name.ends_with("Tests.java")
                    || name == "package-info.java"
                    || name == "module-info.java"
                {
                    continue;
                }
            }

            let abs = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
            let abs_root = root
                .canonicalize()
                .unwrap_or_else(|_| root.clone());

            root_for_file.insert(abs.clone(), abs_root);
            files.push(abs);
        }
    }

    if files.is_empty() {
        anyhow::bail!(
            "no Java source files found under source roots: {:?}",
            source_roots
        );
    }

    files.sort();
    log::info!("discovered {} Java source files", files.len());

    Ok(SourceSet {
        files,
        root_for_file,
    })
}
