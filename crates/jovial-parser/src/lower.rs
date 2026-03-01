use jovial_ast::java::{
    JavaCompilationUnit, JavaNode, JavaType, Modifier,
};
use jovial_ast::span::Span;
use tree_sitter::{Node, Parser, Tree};

use crate::error::ParseError;
use crate::lower_expr::lower_expr;
use crate::lower_stmt::lower_stmt;
use crate::lower_type::lower_type;

/// Parse Java source into a `JavaCompilationUnit`.
pub fn parse_java(source: &str, filename: &str) -> Result<JavaCompilationUnit, Vec<ParseError>> {
    let mut parser = Parser::new();
    let language = tree_sitter_java::LANGUAGE;
    parser
        .set_language(&language.into())
        .expect("failed to load tree-sitter-java grammar");

    let tree: Tree = parser
        .parse(source, None)
        .expect("tree-sitter parse returned None");

    let lowerer = Lowerer::new(source, filename);
    lowerer.lower_program(tree.root_node())
}

/// Holds source text and filename for CST→AST lowering.
pub(crate) struct Lowerer<'a> {
    pub source: &'a str,
    pub filename: &'a str,
}

impl<'a> Lowerer<'a> {
    pub fn new(source: &'a str, filename: &'a str) -> Self {
        Self { source, filename }
    }

    // ── Helpers ──────────────────────────────────────────────────────

    /// Extract the text corresponding to a tree-sitter node.
    pub fn node_text(&self, node: Node) -> &str {
        node.utf8_text(self.source.as_bytes())
            .unwrap_or("<invalid utf8>")
    }

    /// Convert a tree-sitter node's range into our 1-based Span.
    pub fn span(&self, node: Node) -> Span {
        let start = node.start_position();
        let end = node.end_position();
        Span::new(
            self.filename,
            start.row + 1,
            start.column + 1,
            end.row + 1,
            end.column + 1,
        )
    }

    /// Build a ParseError for the given node.
    pub fn err(&self, node: Node, message: impl Into<String>) -> ParseError {
        ParseError::new(
            self.source,
            self.filename,
            node.start_byte(),
            node.end_byte().saturating_sub(node.start_byte()).max(1),
            message,
        )
    }

    /// Find the first named child with the given `kind`.
    pub fn child_by_kind<'t>(&self, node: Node<'t>, kind: &str) -> Option<Node<'t>> {
        let mut cursor = node.walk();
        let result = node.named_children(&mut cursor)
            .find(|c| c.kind() == kind);
        result
    }

    /// Collect all named children with the given `kind`.
    pub fn children_by_kind<'t>(&self, node: Node<'t>, kind: &str) -> Vec<Node<'t>> {
        let mut cursor = node.walk();
        node.named_children(&mut cursor)
            .filter(|c| c.kind() == kind)
            .collect()
    }

    // ── Program ─────────────────────────────────────────────────────

    fn lower_program(&self, node: Node) -> Result<JavaCompilationUnit, Vec<ParseError>> {
        let mut errors = Vec::new();
        let mut package = None;
        let mut imports = Vec::new();
        let mut types = Vec::new();

        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "package_declaration" => {
                    package = self.lower_package(child);
                }
                "import_declaration" => {
                    imports.push(self.lower_import(child));
                }
                "class_declaration" | "interface_declaration" | "enum_declaration" => {
                    match self.lower_type_decl(child) {
                        Ok(decl) => types.push(decl),
                        Err(e) => errors.push(e),
                    }
                }
                "line_comment" | "block_comment" => {}
                _ => {
                    log::debug!("skipping top-level node kind: {}", child.kind());
                }
            }
        }

        // Report tree-sitter ERROR nodes
        self.collect_errors(node, &mut errors);

        if !errors.is_empty() {
            return Err(errors);
        }

        Ok(JavaCompilationUnit {
            package,
            imports,
            types,
            span: self.span(node),
        })
    }

    fn collect_errors(&self, node: Node, errors: &mut Vec<ParseError>) {
        if node.is_error() {
            errors.push(self.err(node, "syntax error"));
        }
        if node.is_missing() {
            errors.push(self.err(node, format!("missing {}", node.kind())));
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_errors(child, errors);
        }
    }

    // ── Package & Import ────────────────────────────────────────────

    fn lower_package(&self, node: Node) -> Option<String> {
        // The package name is a scoped_identifier or identifier child
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            match child.kind() {
                "scoped_identifier" | "identifier" => {
                    return Some(self.node_text(child).to_string());
                }
                _ => {}
            }
        }
        None
    }

    fn lower_import(&self, node: Node) -> String {
        // Collect the full import path text, strip `import` keyword and `;`
        let text = self.node_text(node).trim().to_string();
        text.trim_start_matches("import")
            .trim_start_matches("static")
            .trim()
            .trim_end_matches(';')
            .trim()
            .to_string()
    }

    // ── Declarations ────────────────────────────────────────────────

    pub(crate) fn lower_type_decl(&self, node: Node) -> Result<JavaNode, ParseError> {
        match node.kind() {
            "class_declaration" => self.lower_class(node),
            "interface_declaration" => self.lower_interface(node),
            "enum_declaration" => self.lower_enum(node),
            other => Err(self.err(node, format!("unexpected type declaration: {other}"))),
        }
    }

    fn lower_class(&self, node: Node) -> Result<JavaNode, ParseError> {
        let name = self.child_by_kind(node, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let (modifiers, annotations) = self.lower_modifiers_and_annotations(node);

        let superclass = self.child_by_kind(node, "superclass")
            .and_then(|sc| {
                let mut cursor = sc.walk();
                let first = sc.named_children(&mut cursor).next();
                first
            })
            .map(|t| lower_type(self, t));

        let interfaces = self.child_by_kind(node, "super_interfaces")
            .map(|si| {
                self.child_by_kind(si, "type_list")
                    .map(|tl| {
                        let mut cursor = tl.walk();
                        tl.named_children(&mut cursor)
                            .map(|t| lower_type(self, t))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let members = self.lower_class_body(node);

        Ok(JavaNode::ClassDecl {
            name,
            modifiers,
            superclass,
            interfaces,
            annotations,
            members,
            span: self.span(node),
        })
    }

    fn lower_interface(&self, node: Node) -> Result<JavaNode, ParseError> {
        let name = self.child_by_kind(node, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let (modifiers, annotations) = self.lower_modifiers_and_annotations(node);

        let extends = self.child_by_kind(node, "extends_interfaces")
            .map(|ei| {
                self.child_by_kind(ei, "type_list")
                    .map(|tl| {
                        let mut cursor = tl.walk();
                        tl.named_children(&mut cursor)
                            .map(|t| lower_type(self, t))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        let members = self.lower_interface_body(node);

        Ok(JavaNode::InterfaceDecl {
            name,
            modifiers,
            extends,
            annotations,
            members,
            span: self.span(node),
        })
    }

    fn lower_enum(&self, node: Node) -> Result<JavaNode, ParseError> {
        let name = self.child_by_kind(node, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let (modifiers, annotations) = self.lower_modifiers_and_annotations(node);

        let body = self.child_by_kind(node, "enum_body");
        let mut constants = Vec::new();
        let mut members = Vec::new();

        if let Some(body) = body {
            let mut cursor = body.walk();
            for child in body.named_children(&mut cursor) {
                match child.kind() {
                    "enum_constant" => {
                        if let Some(id) = self.child_by_kind(child, "identifier") {
                            constants.push(self.node_text(id).to_string());
                        }
                    }
                    "enum_body_declarations" => {
                        let mut inner_cursor = child.walk();
                        for member in child.named_children(&mut inner_cursor) {
                            if let Some(m) = self.lower_member(member) {
                                members.push(m);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(JavaNode::EnumDecl {
            name,
            modifiers,
            constants,
            annotations,
            members,
            span: self.span(node),
        })
    }

    fn lower_class_body(&self, node: Node) -> Vec<JavaNode> {
        let body = match self.child_by_kind(node, "class_body") {
            Some(b) => b,
            None => return Vec::new(),
        };
        let mut members = Vec::new();
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            if let Some(m) = self.lower_member(child) {
                members.push(m);
            }
        }
        members
    }

    fn lower_interface_body(&self, node: Node) -> Vec<JavaNode> {
        let body = match self.child_by_kind(node, "interface_body") {
            Some(b) => b,
            None => return Vec::new(),
        };
        let mut members = Vec::new();
        let mut cursor = body.walk();
        for child in body.named_children(&mut cursor) {
            if let Some(m) = self.lower_member(child) {
                members.push(m);
            }
        }
        members
    }

    fn lower_member(&self, node: Node) -> Option<JavaNode> {
        match node.kind() {
            "method_declaration" => Some(self.lower_method(node)),
            "constructor_declaration" => Some(self.lower_constructor(node)),
            "field_declaration" => self.lower_field(node),
            "class_declaration" | "interface_declaration" | "enum_declaration" => {
                self.lower_type_decl(node).ok()
            }
            "static_initializer" | "constructor_body" => None,
            "block" => Some(lower_stmt(self, node)),
            _ => {
                log::debug!("skipping member kind: {}", node.kind());
                None
            }
        }
    }

    // ── Method & Constructor ────────────────────────────────────────

    fn lower_method(&self, node: Node) -> JavaNode {
        let name = self.child_by_kind(node, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let (modifiers, annotations) = self.lower_modifiers_and_annotations(node);

        let return_type = node.child_by_field_name("type")
            .map(|t| lower_type(self, t));

        // Handle void return type
        let return_type = return_type.or_else(|| {
            self.child_by_kind(node, "void_type")
                .map(|_| JavaType {
                    name: "void".to_string(),
                    type_args: Vec::new(),
                    array_dimensions: 0,
                    is_varargs: false,
                })
        });

        let parameters = self.lower_formal_parameters(node);

        let throws = self.lower_throws(node);

        let body = self.child_by_kind(node, "block")
            .map(|b| Box::new(lower_stmt(self, b)));

        JavaNode::MethodDecl {
            name,
            modifiers,
            return_type,
            parameters,
            annotations,
            body,
            throws,
            span: self.span(node),
        }
    }

    fn lower_constructor(&self, node: Node) -> JavaNode {
        let name = self.child_by_kind(node, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let (modifiers, annotations) = self.lower_modifiers_and_annotations(node);

        let parameters = self.lower_formal_parameters(node);
        let throws = self.lower_throws(node);

        let body = self.child_by_kind(node, "constructor_body")
            .map(|b| {
                let mut stmts = Vec::new();
                let mut cursor = b.walk();
                for child in b.named_children(&mut cursor) {
                    match child.kind() {
                        "explicit_constructor_invocation" => {
                            // Handle this(...) or super(...) calls
                            stmts.push(lower_stmt(self, child));
                        }
                        _ => {
                            stmts.push(lower_stmt(self, child));
                        }
                    }
                }
                Box::new(JavaNode::BlockStmt {
                    statements: stmts,
                    span: self.span(b),
                })
            })
            .unwrap_or_else(|| Box::new(JavaNode::BlockStmt {
                statements: Vec::new(),
                span: self.span(node),
            }));

        JavaNode::ConstructorDecl {
            name,
            modifiers,
            parameters,
            annotations,
            body,
            throws,
            span: self.span(node),
        }
    }

    // ── Field ───────────────────────────────────────────────────────

    fn lower_field(&self, node: Node) -> Option<JavaNode> {
        let (modifiers, annotations) = self.lower_modifiers_and_annotations(node);

        let field_type = node.child_by_field_name("type")
            .map(|t| lower_type(self, t))
            .unwrap_or_else(|| JavaType {
                name: "unknown".to_string(),
                type_args: Vec::new(),
                array_dimensions: 0,
                is_varargs: false,
            });

        // Get the declarator(s) — we take the first one
        let declarator = self.child_by_kind(node, "variable_declarator")?;

        let name = self.child_by_kind(declarator, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let initializer = declarator.child_by_field_name("value")
            .map(|v| Box::new(lower_expr(self, v)));

        Some(JavaNode::FieldDecl {
            name,
            modifiers,
            field_type,
            initializer,
            annotations,
            span: self.span(node),
        })
    }

    // ── Parameters ──────────────────────────────────────────────────

    fn lower_formal_parameters(&self, node: Node) -> Vec<JavaNode> {
        let params = match self.child_by_kind(node, "formal_parameters") {
            Some(p) => p,
            None => return Vec::new(),
        };
        let mut result = Vec::new();
        let mut cursor = params.walk();
        for child in params.named_children(&mut cursor) {
            match child.kind() {
                "formal_parameter" | "spread_parameter" => {
                    result.push(self.lower_parameter(child));
                }
                _ => {}
            }
        }
        result
    }

    pub(crate) fn lower_parameter(&self, node: Node) -> JavaNode {
        let (_, annotations) = self.lower_modifiers_and_annotations(node);

        let mut param_type = node.child_by_field_name("type")
            .map(|t| lower_type(self, t))
            .unwrap_or_else(|| JavaType {
                name: "unknown".to_string(),
                type_args: Vec::new(),
                array_dimensions: 0,
                is_varargs: false,
            });

        if node.kind() == "spread_parameter" {
            param_type.is_varargs = true;
        }

        let name = self.child_by_kind(node, "identifier")
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        JavaNode::Parameter {
            name,
            param_type,
            annotations,
            span: self.span(node),
        }
    }

    // ── Throws ──────────────────────────────────────────────────────

    fn lower_throws(&self, node: Node) -> Vec<JavaType> {
        let throws = match self.child_by_kind(node, "throws") {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut cursor = throws.walk();
        throws.named_children(&mut cursor)
            .map(|t| lower_type(self, t))
            .collect()
    }

    // ── Modifiers & Annotations ─────────────────────────────────────

    #[allow(clippy::vec_box)]
    fn lower_modifiers_and_annotations(&self, node: Node) -> (Vec<Modifier>, Vec<Box<JavaNode>>) {
        let mods_node = match self.child_by_kind(node, "modifiers") {
            Some(m) => m,
            None => return (Vec::new(), Vec::new()),
        };

        let mut modifiers = Vec::new();
        let mut annotations = Vec::new();

        let mut cursor = mods_node.walk();
        for child in mods_node.children(&mut cursor) {
            match child.kind() {
                "marker_annotation" | "annotation" => {
                    annotations.push(Box::new(self.lower_annotation(child)));
                }
                _ => {
                    if let Some(m) = self.text_to_modifier(self.node_text(child)) {
                        modifiers.push(m);
                    }
                }
            }
        }

        (modifiers, annotations)
    }

    fn text_to_modifier(&self, text: &str) -> Option<Modifier> {
        match text {
            "public" => Some(Modifier::Public),
            "private" => Some(Modifier::Private),
            "protected" => Some(Modifier::Protected),
            "static" => Some(Modifier::Static),
            "final" => Some(Modifier::Final),
            "abstract" => Some(Modifier::Abstract),
            "synchronized" => Some(Modifier::Synchronized),
            "native" => Some(Modifier::Native),
            "transient" => Some(Modifier::Transient),
            "volatile" => Some(Modifier::Volatile),
            "default" => Some(Modifier::Default),
            _ => None,
        }
    }

    fn lower_annotation(&self, node: Node) -> JavaNode {
        let name = self.child_by_kind(node, "identifier")
            .or_else(|| self.child_by_kind(node, "scoped_identifier"))
            .map(|n| self.node_text(n).to_string())
            .unwrap_or_default();

        let mut arguments = Vec::new();

        if let Some(args) = self.child_by_kind(node, "annotation_argument_list") {
            let mut cursor = args.walk();
            for child in args.named_children(&mut cursor) {
                match child.kind() {
                    "element_value_pair" => {
                        let key = child.child_by_field_name("key")
                            .map(|k| self.node_text(k).to_string())
                            .unwrap_or_else(|| "value".to_string());
                        let value = child.child_by_field_name("value")
                            .map(|v| Box::new(lower_expr(self, v)))
                            .unwrap_or_else(|| Box::new(JavaNode::LiteralExpr {
                                value: jovial_ast::java::LiteralValue::Null,
                                span: self.span(child),
                            }));
                        arguments.push((key, value));
                    }
                    _ => {
                        // Single value annotation like @Annotation("value")
                        arguments.push((
                            "value".to_string(),
                            Box::new(lower_expr(self, child)),
                        ));
                    }
                }
            }
        }

        JavaNode::AnnotationExpr {
            name,
            arguments,
            span: self.span(node),
        }
    }
}
