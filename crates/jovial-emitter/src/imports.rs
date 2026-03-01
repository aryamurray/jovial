/// Manages Go import blocks with stdlib/external grouping.
pub struct ImportBlock {
    stdlib: Vec<String>,
    external: Vec<String>,
}

impl ImportBlock {
    pub fn new() -> Self {
        Self {
            stdlib: Vec::new(),
            external: Vec::new(),
        }
    }

    /// Add an import path. Automatically categorizes as stdlib or external.
    pub fn add(&mut self, path: impl Into<String>) {
        let path = path.into();
        if Self::is_stdlib(&path) {
            if !self.stdlib.contains(&path) {
                self.stdlib.push(path);
            }
        } else if !self.external.contains(&path) {
            self.external.push(path);
        }
    }

    /// Render the import block as Go source.
    pub fn render(&self) -> String {
        if self.is_empty() {
            return String::new();
        }

        let mut stdlib = self.stdlib.clone();
        stdlib.sort();
        let mut external = self.external.clone();
        external.sort();

        let total = stdlib.len() + external.len();

        if total == 1 {
            let path = if !stdlib.is_empty() {
                &stdlib[0]
            } else {
                &external[0]
            };
            return format!("import \"{path}\"");
        }

        let mut lines = vec!["import (".to_string()];
        for path in &stdlib {
            lines.push(format!("\t\"{path}\""));
        }
        if !stdlib.is_empty() && !external.is_empty() {
            lines.push(String::new());
        }
        for path in &external {
            lines.push(format!("\t\"{path}\""));
        }
        lines.push(")".to_string());
        lines.join("\n")
    }

    /// Check if an import path is from the Go standard library.
    fn is_stdlib(path: &str) -> bool {
        // Stdlib packages don't contain dots in their path
        !path.contains('.')
    }

    /// Whether the import block is empty.
    pub fn is_empty(&self) -> bool {
        self.stdlib.is_empty() && self.external.is_empty()
    }
}

impl Default for ImportBlock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_block() {
        let block = ImportBlock::new();
        insta::assert_snapshot!(block.render(), @"");
    }

    #[test]
    fn single_stdlib() {
        let mut block = ImportBlock::new();
        block.add("fmt");
        insta::assert_snapshot!(block.render(), @r#"import "fmt""#);
    }

    #[test]
    fn single_external() {
        let mut block = ImportBlock::new();
        block.add("github.com/gin-gonic/gin");
        insta::assert_snapshot!(block.render(), @r#"import "github.com/gin-gonic/gin""#);
    }

    #[test]
    fn multiple_stdlib_only() {
        let mut block = ImportBlock::new();
        block.add("strings");
        block.add("fmt");
        block.add("os");
        insta::assert_snapshot!(block.render(), @r#"
        import (
        	"fmt"
        	"os"
        	"strings"
        )
        "#);
    }

    #[test]
    fn mixed_grouped() {
        let mut block = ImportBlock::new();
        block.add("fmt");
        block.add("github.com/gin-gonic/gin");
        block.add("strings");
        block.add("github.com/lib/pq");
        insta::assert_snapshot!(block.render(), @r#"
        import (
        	"fmt"
        	"strings"

        	"github.com/gin-gonic/gin"
        	"github.com/lib/pq"
        )
        "#);
    }

    #[test]
    fn deduplication() {
        let mut block = ImportBlock::new();
        block.add("fmt");
        block.add("fmt");
        block.add("github.com/gin-gonic/gin");
        block.add("github.com/gin-gonic/gin");
        insta::assert_snapshot!(block.render(), @r#"
        import (
        	"fmt"

        	"github.com/gin-gonic/gin"
        )
        "#);
    }
}
