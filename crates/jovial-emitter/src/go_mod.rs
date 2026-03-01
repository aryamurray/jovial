/// Generates go.mod file content.
///
/// Takes `&[(String, String)]` (module, version) tuples instead of `GoDependency`
/// to keep jovial-emitter free from jovial-plugin dependency.
pub struct GoModGenerator {
    module_path: String,
    go_version: String,
}

impl GoModGenerator {
    pub fn new(module_path: impl Into<String>, go_version: impl Into<String>) -> Self {
        Self {
            module_path: module_path.into(),
            go_version: go_version.into(),
        }
    }

    /// Generate go.mod content from a list of (module, version) dependencies.
    pub fn generate(&self, deps: &[(String, String)]) -> String {
        let mut out = format!("module {}\n\ngo {}\n", self.module_path, self.go_version);

        if !deps.is_empty() {
            out.push_str("\nrequire (\n");
            for (module, version) in deps {
                out.push_str(&format!("\t{module} {version}\n"));
            }
            out.push_str(")\n");
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_deps() {
        let gen = GoModGenerator::new("github.com/example/myapp", "1.21");
        let result = gen.generate(&[]);
        insta::assert_snapshot!(result, @r"
        module github.com/example/myapp

        go 1.21
        ");
    }

    #[test]
    fn with_deps() {
        let gen = GoModGenerator::new("github.com/example/myapp", "1.21");
        let deps = vec![
            ("github.com/gin-gonic/gin".into(), "v1.9.1".into()),
            ("github.com/lib/pq".into(), "v1.10.9".into()),
        ];
        let result = gen.generate(&deps);
        insta::assert_snapshot!(result, @r"
        module github.com/example/myapp

        go 1.21

        require (
        	github.com/gin-gonic/gin v1.9.1
        	github.com/lib/pq v1.10.9
        )
        ");
    }
}
