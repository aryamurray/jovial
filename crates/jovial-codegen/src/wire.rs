use std::collections::{HashMap, HashSet, VecDeque};

use jovial_manifest::beans::Bean;
use jovial_manifest::Manifest;

/// Generates dependency injection wiring code (InitializeApp function).
pub struct WireGenerator {
    module_path: String,
}

impl WireGenerator {
    pub fn new(module_path: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
        }
    }

    /// Generate the InitializeApp() function that wires all dependencies.
    pub fn generate_initialize_app(
        &self,
        manifest: &Manifest,
    ) -> Result<String, std::io::Error> {
        let mut out = String::new();

        out.push_str("package wire\n\n");

        // Collect imports
        let mut imports = Vec::new();
        if !manifest.endpoints.is_empty() {
            imports.push("\"github.com/gin-gonic/gin\"".to_string());
        }

        // Add imports for each package we'll reference
        let packages = collect_packages(&manifest.beans, &self.module_path);
        for pkg in &packages {
            imports.push(format!("\"{}\"", pkg));
        }

        if !imports.is_empty() {
            out.push_str("import (\n");
            for imp in &imports {
                out.push_str(&format!("\t{}\n", imp));
            }
            out.push_str(")\n\n");
        }

        // App struct
        out.push_str("// App holds the initialized application components.\n");
        out.push_str("type App struct {\n");
        if !manifest.endpoints.is_empty() {
            out.push_str("\tRouter *gin.Engine\n");
        }
        out.push_str("}\n\n");

        // InitializeApp function
        out.push_str(
            "// InitializeApp wires all dependencies and returns the application.\n",
        );
        out.push_str("func InitializeApp() (*App, error) {\n");

        // Sort beans topologically
        let sorted = topological_sort(&manifest.beans);

        // Categorize beans for readability
        let (repos, services, controllers, others) = categorize_beans(&sorted);

        // Emit bean instantiations by category
        emit_bean_group(&mut out, &repos, "repositories", &manifest.beans);
        emit_bean_group(&mut out, &services, "services", &manifest.beans);
        emit_bean_group(&mut out, &controllers, "controllers", &manifest.beans);
        emit_bean_group(&mut out, &others, "other", &manifest.beans);

        // Router setup if we have endpoints
        if !manifest.endpoints.is_empty() {
            out.push_str("\trouter := gin.Default()\n\n");

            for endpoint in &manifest.endpoints {
                let method = format!("{:?}", endpoint.method).to_uppercase();
                let handler_var = to_var_name(&simple_name(&endpoint.handler_class));
                let handler_method = to_exported_name(&endpoint.handler_method);
                out.push_str(&format!(
                    "\trouter.{}(\"{}\", {}.{})\n",
                    method, endpoint.path, handler_var, handler_method,
                ));
            }

            out.push('\n');
            out.push_str("\treturn &App{Router: router}, nil\n");
        } else {
            out.push_str("\treturn &App{}, nil\n");
        }

        out.push_str("}\n");

        Ok(out)
    }
}

/// Topological sort of beans by dependency graph (Kahn's algorithm).
fn topological_sort(beans: &[Bean]) -> Vec<&Bean> {
    if beans.is_empty() {
        return Vec::new();
    }

    let bean_names: HashMap<&str, usize> = beans
        .iter()
        .enumerate()
        .map(|(i, b)| (b.name.as_str(), i))
        .collect();

    // Build adjacency and in-degree
    let n = beans.len();
    let mut in_degree = vec![0usize; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, bean) in beans.iter().enumerate() {
        for dep in &bean.dependencies {
            if let Some(&j) = bean_names.get(dep.bean_name.as_str()) {
                adj[j].push(i); // j must come before i
                in_degree[i] += 1;
            }
        }
    }

    // Kahn's algorithm
    let mut queue: VecDeque<usize> = VecDeque::new();
    for i in 0..n {
        if in_degree[i] == 0 {
            queue.push_back(i);
        }
    }

    let mut sorted = Vec::new();
    while let Some(idx) = queue.pop_front() {
        sorted.push(&beans[idx]);
        for &next in &adj[idx] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push_back(next);
            }
        }
    }

    // If there are cycles, append remaining beans with TODO comments
    if sorted.len() < n {
        let sorted_names: HashSet<&str> = sorted.iter().map(|b| b.name.as_str()).collect();
        for bean in beans {
            if !sorted_names.contains(bean.name.as_str()) {
                sorted.push(bean);
            }
        }
    }

    sorted
}

/// Categorize beans into repos, services, controllers, and others based on naming.
fn categorize_beans<'a>(
    beans: &[&'a Bean],
) -> (Vec<&'a Bean>, Vec<&'a Bean>, Vec<&'a Bean>, Vec<&'a Bean>) {
    let mut repos = Vec::new();
    let mut services = Vec::new();
    let mut controllers = Vec::new();
    let mut others = Vec::new();

    for bean in beans {
        let name = simple_name(&bean.class_name);
        if name.ends_with("Repository") || name.ends_with("Repo") {
            repos.push(*bean);
        } else if name.ends_with("Service") || name.ends_with("ServiceImpl") {
            services.push(*bean);
        } else if name.ends_with("Controller") || name.ends_with("Handler") {
            controllers.push(*bean);
        } else {
            others.push(*bean);
        }
    }

    (repos, services, controllers, others)
}

/// Emit a group of bean instantiations with a comment header.
fn emit_bean_group(
    out: &mut String,
    beans: &[&Bean],
    label: &str,
    all_beans: &[Bean],
) {
    if beans.is_empty() {
        return;
    }

    out.push_str(&format!("\t// {}\n", label));

    // Build a name->bean lookup for resolving dependencies
    let bean_map: HashMap<&str, &Bean> =
        all_beans.iter().map(|b| (b.name.as_str(), b)).collect();

    for bean in beans {
        let type_name = simple_name(&bean.class_name);
        let var_name = to_var_name(&type_name);
        let constructor = format!("New{}", type_name);

        // Build dependency arguments
        let dep_args: Vec<String> = bean
            .dependencies
            .iter()
            .map(|dep| {
                if bean_map.contains_key(dep.bean_name.as_str()) {
                    to_var_name(&simple_name(&dep.type_name))
                } else {
                    format!(
                        "nil /* TODO(jovial): unresolved dependency '{}' */",
                        dep.bean_name
                    )
                }
            })
            .collect();

        if dep_args.is_empty() {
            out.push_str(&format!("\t{} := {}()\n", var_name, constructor));
        } else {
            out.push_str(&format!(
                "\t{} := {}({})\n",
                var_name,
                constructor,
                dep_args.join(", ")
            ));
        }
    }

    out.push('\n');
}

/// Collect Go package import paths from bean class names.
fn collect_packages(beans: &[Bean], module_path: &str) -> Vec<String> {
    let mut packages = HashSet::new();

    for bean in beans {
        let name = simple_name(&bean.class_name);
        let pkg = if name.ends_with("Repository") || name.ends_with("Repo") {
            "repositories"
        } else if name.ends_with("Service") || name.ends_with("ServiceImpl") {
            "services"
        } else if name.ends_with("Controller") || name.ends_with("Handler") {
            "handlers"
        } else {
            continue;
        };
        packages.insert(format!("{}/{}", module_path, pkg));
    }

    let mut sorted: Vec<String> = packages.into_iter().collect();
    sorted.sort();
    sorted
}

/// Extract the simple class name from a FQCN.
fn simple_name(fqcn: &str) -> String {
    fqcn.rsplit('.').next().unwrap_or(fqcn).to_string()
}

/// Convert a type name to a Go variable name (lowercase first letter).
fn to_var_name(type_name: &str) -> String {
    let mut chars = type_name.chars();
    match chars.next() {
        Some(first) => {
            let lower: String = first.to_lowercase().collect();
            format!("{}{}", lower, chars.as_str())
        }
        None => String::new(),
    }
}

/// Convert a method name to a Go exported name (uppercase first letter).
fn to_exported_name(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => {
            let upper: String = first.to_uppercase().collect();
            format!("{}{}", upper, chars.as_str())
        }
        None => String::new(),
    }
}
