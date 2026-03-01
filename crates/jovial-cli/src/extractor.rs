use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use jovial_manifest::Manifest;

use crate::config::JovialConfig;

/// Result of running the JVM extractor.
pub struct ExtractorResult {
    pub manifest: Manifest,
    pub duration: Duration,
}

/// Run the JVM extractor to produce a manifest, or load one from disk.
pub fn run_extractor(
    config: &JovialConfig,
    project_root: &Path,
) -> anyhow::Result<ExtractorResult> {
    let start = Instant::now();

    // If extraction is skipped, return an empty manifest
    if config.extractor.skip {
        log::info!("extractor skipped (--no-extract or extractor.skip)");
        return Ok(ExtractorResult {
            manifest: Manifest::default(),
            duration: start.elapsed(),
        });
    }

    // If a manifest path is provided, read and deserialize it
    if let Some(ref manifest_path) = config.extractor.manifest_path {
        let path = project_root.join(manifest_path);
        log::info!("loading manifest from: {}", path.display());
        let content = std::fs::read_to_string(&path)?;
        let manifest = Manifest::from_json(&content)?;
        return Ok(ExtractorResult {
            manifest,
            duration: start.elapsed(),
        });
    }

    // Otherwise, shell out to the JVM extractor
    let java_bin = find_java()?;
    let classpath = build_classpath(config, project_root)?;

    log::info!("running JVM extractor with java: {}", java_bin.display());

    let mut cmd = Command::new(&java_bin);

    for opt in &config.extractor.jvm_opts {
        cmd.arg(opt);
    }

    cmd.arg("-cp")
        .arg(&classpath)
        .arg("com.jovial.extractor.ManifestExtractor")
        .current_dir(project_root);

    if let Some(ref profile) = config.source.profile {
        cmd.arg("--profile").arg(profile);
    }

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("JVM extractor failed (exit {}): {}", output.status, stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let manifest = Manifest::from_json(&stdout)?;

    Ok(ExtractorResult {
        manifest,
        duration: start.elapsed(),
    })
}

/// Find the `java` binary: check PATH first, then $JAVA_HOME/bin/java.
fn find_java() -> anyhow::Result<PathBuf> {
    if let Ok(java) = which::which("java") {
        return Ok(java);
    }

    if let Ok(java_home) = std::env::var("JAVA_HOME") {
        let bin = PathBuf::from(&java_home)
            .join("bin")
            .join(if cfg!(windows) { "java.exe" } else { "java" });
        if bin.exists() {
            return Ok(bin);
        }
    }

    anyhow::bail!(
        "java not found in PATH or JAVA_HOME. \
         Install a JDK or set JAVA_HOME, or use --no-extract to skip extraction."
    )
}

/// Build a classpath string for the JVM extractor.
fn build_classpath(config: &JovialConfig, project_root: &Path) -> anyhow::Result<String> {
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut entries = Vec::new();

    // classes_dir
    if let Some(ref classes_dir) = config.classpath.classes_dir {
        entries.push(project_root.join(classes_dir).to_string_lossy().into_owned());
    }

    // All JARs in lib_dir
    if let Some(ref lib_dir) = config.classpath.lib_dir {
        let dir = project_root.join(lib_dir);
        if dir.exists() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("jar") {
                    entries.push(path.to_string_lossy().into_owned());
                }
            }
        }
    }

    // Extra classpath entries
    for extra in &config.classpath.extra {
        entries.push(project_root.join(extra).to_string_lossy().into_owned());
    }

    // Extractor JAR itself
    let extractor_jar = find_extractor_jar()?;
    entries.push(extractor_jar.to_string_lossy().into_owned());

    Ok(entries.join(sep))
}

/// Locate the extractor JAR. Checks (in order):
/// 1. $JOVIAL_EXTRACTOR_JAR env var
/// 2. ~/.jovial/extractor/jovial-extractor.jar
/// 3. ./extractor/target/jovial-extractor.jar
fn find_extractor_jar() -> anyhow::Result<PathBuf> {
    if let Ok(path) = std::env::var("JOVIAL_EXTRACTOR_JAR") {
        let p = PathBuf::from(&path);
        if p.exists() {
            return Ok(p);
        }
        log::warn!("JOVIAL_EXTRACTOR_JAR set but not found: {}", path);
    }

    if let Some(home) = dirs_home() {
        let p = home
            .join(".jovial")
            .join("extractor")
            .join("jovial-extractor.jar");
        if p.exists() {
            return Ok(p);
        }
    }

    let local = PathBuf::from("extractor/target/jovial-extractor.jar");
    if local.exists() {
        return Ok(local);
    }

    anyhow::bail!(
        "jovial-extractor.jar not found. Set $JOVIAL_EXTRACTOR_JAR, \
         place it at ~/.jovial/extractor/jovial-extractor.jar, \
         or use --no-extract to skip extraction."
    )
}

/// Get the user's home directory.
fn dirs_home() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}
