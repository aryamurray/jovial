use std::collections::HashMap;
use std::path::{Path, PathBuf};

use clap::Parser;

use jovial_ast::go::{GoFile, GoNode};
use jovial_ast::java::JavaCompilationUnit;
use jovial_codegen::project::ProjectGenerator;
use jovial_codegen::wire::WireGenerator;
use jovial_emitter::emitter::GoEmitter;
use jovial_manifest::Manifest;
use jovial_parser::type_resolver::DefaultTypeResolver;
use jovial_plugin::types::ConfigValue;
use jovial_walker::walker::Walker;

use crate::config::JovialConfig;
use crate::discovery;
use crate::extractor;
use crate::loader;

/// Arguments for the transpile command.
#[derive(Parser)]
pub struct Args {
    /// Path to jovial.yaml config file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Output directory override
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Skip JVM extractor (use empty or file-based manifest)
    #[arg(long)]
    pub no_extract: bool,

    /// Verbose logging
    #[arg(short, long)]
    pub verbose: bool,
}

/// Run the transpile command.
pub fn run(args: Args) -> anyhow::Result<()> {
    // 1. Load config
    let mut config = match &args.config {
        Some(path) => JovialConfig::load(path)?,
        None => JovialConfig::load_or_default()?,
    };

    // Apply CLI overrides
    if let Some(ref output) = args.output {
        config.target.output_dir = output.to_string_lossy().into_owned();
    }
    if args.no_extract {
        config.extractor.skip = true;
    }
    if args.verbose {
        config.options.verbose = true;
    }

    let project_root = resolve_project_root(&args)?;

    if config.options.verbose {
        eprintln!("project root: {}", project_root.display());
        eprintln!("output dir:   {}", config.target.output_dir);
        eprintln!("go module:    {}", config.target.module);
    }

    // 2. Discover sources
    let source_set = discovery::discover_sources(&project_root, &config.source.roots)?;
    eprintln!("found {} Java source files", source_set.files.len());

    // 3. Run extractor
    let extractor_result = extractor::run_extractor(&config, &project_root)?;
    let manifest = extractor_result.manifest;

    if config.options.verbose {
        eprintln!(
            "manifest: {} beans, {} endpoints, {} entities (in {:.1}s)",
            manifest.beans.len(),
            manifest.endpoints.len(),
            manifest.entities.len(),
            extractor_result.duration.as_secs_f64(),
        );
    }

    // 4. Load plugins
    let plugin_registry = loader::load_plugins(&config.plugins);
    let plugin_config = build_plugin_config(&config);

    // 5-6-7. Parse, walk, emit
    let mut emitted_files: Vec<(PathBuf, String)> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    let mut todo_count = 0u32;

    for java_path in &source_set.files {
        let source_root = source_set
            .root_for_file
            .get(java_path)
            .cloned()
            .unwrap_or_else(|| project_root.clone());

        match process_file(
            java_path,
            &source_root,
            &manifest,
            &plugin_registry,
            &plugin_config,
        ) {
            Ok(result) => {
                todo_count += result.go_source.matches("// TODO(jovial)").count() as u32;
                emitted_files.push((result.output_path, result.go_source));
            }
            Err(e) => {
                let msg = format!(
                    "{}: {}",
                    java_path.file_name().unwrap_or_default().to_string_lossy(),
                    e
                );
                eprintln!("error: {}", msg);
                errors.push(msg);
                if config.options.error_handling == "fail" {
                    anyhow::bail!("transpilation failed (error_handling=fail)");
                }
            }
        }
    }

    // 8. Scaffold output
    let output_dir = PathBuf::from(&config.target.output_dir);
    let generator = ProjectGenerator::new(
        config.target.output_dir.clone(),
        config.target.module.clone(),
    );
    generator.scaffold(&manifest)?;

    // Write emitted Go files
    generator.write_emitted_files(&emitted_files)?;

    // Generate wire/DI code
    let wire_gen = WireGenerator::new(config.target.module.clone());
    let wire_code = wire_gen.generate_initialize_app(&manifest)?;
    let wire_path = output_dir.join("internal").join("wire").join("wire.go");
    std::fs::create_dir_all(wire_path.parent().unwrap())?;
    std::fs::write(&wire_path, &wire_code)?;

    // 9. Report
    eprintln!();
    eprintln!("--- jovial transpile complete ---");
    eprintln!(
        "  {} Go files generated",
        emitted_files.len()
    );
    if !errors.is_empty() {
        eprintln!("  {} errors", errors.len());
    }
    if todo_count > 0 {
        eprintln!("  {} TODO(jovial) markers", todo_count);
    }
    eprintln!("  output: {}", output_dir.display());

    Ok(())
}

struct ProcessResult {
    output_path: PathBuf,
    go_source: String,
}

fn process_file(
    java_path: &Path,
    source_root: &Path,
    manifest: &Manifest,
    plugin_registry: &jovial_plugin::registry::PluginRegistry,
    plugin_config: &HashMap<String, ConfigValue>,
) -> anyhow::Result<ProcessResult> {
    // 5. Parse
    let source = std::fs::read_to_string(java_path)?;
    let filename = java_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    let unit = jovial_parser::parse_java(&source, &filename)
        .map_err(|errs| {
            anyhow::anyhow!(
                "parse errors: {}",
                errs.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        })?;

    // 6. Build type resolver and walk
    let type_resolver = build_type_resolver(&unit, manifest);
    let walker = Walker::new(plugin_registry, &type_resolver, plugin_config);
    let go_nodes = walker.walk(&unit)?;

    // 7. Emit Go source
    let go_file = nodes_to_go_file(go_nodes);
    let mut emitter = GoEmitter::new();
    let go_source = emitter.emit_file(&go_file)?;

    // Compute output path
    let output_path = map_java_path_to_go(java_path, source_root, manifest);

    Ok(ProcessResult {
        output_path,
        go_source,
    })
}

/// Build a DefaultTypeResolver seeded from the compilation unit's imports
/// and the manifest's beans and entities.
fn build_type_resolver(
    unit: &JavaCompilationUnit,
    manifest: &Manifest,
) -> DefaultTypeResolver {
    let mut resolver = DefaultTypeResolver::new();

    // Seed from file's import statements
    for import in &unit.imports {
        if let Some(simple) = import.rsplit('.').next() {
            resolver.add_type(simple.to_string(), import.clone());
        }
    }

    // Seed from manifest beans (known types in the project)
    for bean in &manifest.beans {
        let simple = bean
            .class_name
            .rsplit('.')
            .next()
            .unwrap_or(&bean.class_name);
        resolver.add_type(simple.to_string(), bean.class_name.clone());

        // Interface info from proxy_info
        if let Some(ref proxy) = bean.proxy_info {
            resolver.add_interfaces(bean.class_name.clone(), proxy.interfaces.clone());
        }
    }

    // Seed from manifest entities
    for entity in &manifest.entities {
        let simple = entity
            .class_name
            .rsplit('.')
            .next()
            .unwrap_or(&entity.class_name);
        resolver.add_type(simple.to_string(), entity.class_name.clone());
    }

    resolver
}

/// Bridge Walker's Vec<GoNode> output to the GoFile struct the emitter expects.
fn nodes_to_go_file(nodes: Vec<GoNode>) -> GoFile {
    let mut package = "main".to_string();
    let mut file_nodes = Vec::new();

    for node in nodes {
        match node {
            GoNode::Package { name } => {
                package = name;
            }
            other => {
                file_nodes.push(other);
            }
        }
    }

    GoFile {
        package,
        imports: Vec::new(), // V1: user runs goimports
        nodes: file_nodes,
    }
}

/// Map a Java source path to a Go output path with package placement heuristics.
fn map_java_path_to_go(
    java_path: &Path,
    source_root: &Path,
    manifest: &Manifest,
) -> PathBuf {
    // Get relative path from source root
    let relative = java_path
        .strip_prefix(source_root)
        .unwrap_or(java_path);

    // Extract the class name (without .java)
    let stem = java_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();

    // Convert CamelCase to snake_case for file name
    let go_filename = format!("{}.go", camel_to_snake(&stem));

    // Determine the FQCN from the path
    let fqcn = path_to_fqcn(relative);

    // Use manifest heuristics for package placement
    let go_dir = determine_go_package(&stem, &fqcn, manifest, relative);

    PathBuf::from(go_dir).join(go_filename)
}

/// Determine which Go package directory a class should be placed in.
fn determine_go_package(
    class_name: &str,
    fqcn: &str,
    manifest: &Manifest,
    relative_path: &Path,
) -> String {
    // Check if class is an endpoint handler
    if manifest
        .endpoints
        .iter()
        .any(|e| e.handler_class == fqcn || e.handler_class.ends_with(class_name))
    {
        return "handlers".to_string();
    }

    // Check if class is an entity
    if manifest
        .entities
        .iter()
        .any(|e| e.class_name == fqcn || e.class_name.ends_with(class_name))
    {
        return "models".to_string();
    }

    // Name-based heuristics
    if class_name.ends_with("Repository") || class_name.ends_with("Repo") {
        return "repositories".to_string();
    }
    if class_name.ends_with("Service") || class_name.ends_with("ServiceImpl") {
        return "services".to_string();
    }
    if class_name.contains("Config") || class_name.contains("Configuration") {
        return "internal/config".to_string();
    }

    // Fall back to last Java package segment
    let parent = relative_path
        .parent()
        .unwrap_or(Path::new(""));
    let components: Vec<&str> = parent
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Drop common prefixes: com, org, net, and the next component (company name)
    let skip = if components.first().map(|&c| c == "com" || c == "org" || c == "net").unwrap_or(false) {
        2.min(components.len())
    } else {
        0
    };

    let remaining = &components[skip..];
    if remaining.is_empty() {
        "pkg".to_string()
    } else {
        format!("pkg/{}", remaining.join("/"))
    }
}

/// Convert a relative path to a FQCN-like string.
fn path_to_fqcn(relative: &Path) -> String {
    let without_ext = relative.with_extension("");
    without_ext
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join(".")
}

/// Convert CamelCase to snake_case.
fn camel_to_snake(name: &str) -> String {
    let mut result = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                // Don't add underscore if previous char is also uppercase
                // and next char is lowercase (e.g. "HTTPHandler" -> "http_handler")
                let prev_upper = name.chars().nth(i - 1).map(|c| c.is_uppercase()).unwrap_or(false);
                let next_lower = name.chars().nth(i + 1).map(|c| c.is_lowercase()).unwrap_or(false);
                if !prev_upper || next_lower {
                    result.push('_');
                }
            }
            result.push(ch.to_lowercase().next().unwrap());
        } else {
            result.push(ch);
        }
    }
    result
}

/// Resolve the project root directory. Uses the config file's parent directory,
/// or the current working directory if no config was specified.
fn resolve_project_root(args: &Args) -> anyhow::Result<PathBuf> {
    if let Some(ref config_path) = args.config {
        let abs = std::fs::canonicalize(config_path)?;
        Ok(abs
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")))
    } else {
        Ok(std::env::current_dir()?)
    }
}

/// Build plugin config map from the JovialConfig's plugin_config section.
fn build_plugin_config(config: &JovialConfig) -> HashMap<String, ConfigValue> {
    let mut map = HashMap::new();
    for (key, values) in &config.plugin_config {
        for (k, v) in values {
            let full_key = format!("{}.{}", key, k);
            map.insert(full_key, ConfigValue::String(v.clone()));
        }
    }
    map
}
