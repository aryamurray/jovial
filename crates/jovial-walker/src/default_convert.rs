use std::collections::HashSet;

use jovial_ast::go::GoNode;
use jovial_ast::java::JavaNode;

use crate::convert_decl;
use crate::convert_expr;
use crate::convert_stmt;
use crate::walker::WalkError;

/// Default mechanical converter for Java nodes that no plugin claims.
pub struct DefaultConverter;

impl DefaultConverter {
    pub fn new() -> Self {
        Self
    }

    /// Walk a child node within a class context, threading `current_class`
    /// and the class's field names through all descendants so `this` resolves
    /// to the correct receiver and bare field names get prefixed.
    pub(crate) fn walk_in_class<'a>(
        &'a self,
        node: &JavaNode,
        walk_child: &'a dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
        class_name: &'a str,
        class_fields: &'a HashSet<String>,
    ) -> Result<Vec<GoNode>, WalkError> {
        let class_walk = |child: &JavaNode| -> Result<Vec<GoNode>, WalkError> {
            self.walk_in_class(child, walk_child, class_name, class_fields)
        };
        self.convert(node, &class_walk, Some(class_name), Some(class_fields))
    }

    /// Convert a Java AST node to Go AST node(s) using basic mechanical translation.
    pub fn convert(
        &self,
        node: &JavaNode,
        walk_child: &dyn Fn(&JavaNode) -> Result<Vec<GoNode>, WalkError>,
        current_class: Option<&str>,
        class_fields: Option<&HashSet<String>>,
    ) -> Result<Vec<GoNode>, WalkError> {
        match node {
            // ── Declarations ────────────────────────────────────────

            JavaNode::ClassDecl {
                name,
                members,
                span,
                ..
            } => convert_decl::convert_class_decl(self, name, members, span, walk_child),

            JavaNode::InterfaceDecl {
                name,
                members,
                span,
                ..
            } => convert_decl::convert_interface_decl(name, members, span),

            JavaNode::EnumDecl {
                name,
                constants,
                span,
                ..
            } => convert_decl::convert_enum_decl(name, constants, span),

            JavaNode::MethodDecl {
                name,
                modifiers,
                return_type,
                parameters,
                body,
                span,
                ..
            } => convert_decl::convert_method_decl(
                name,
                modifiers,
                return_type.as_ref(),
                parameters,
                body.as_deref(),
                span,
                walk_child,
                current_class,
            ),

            JavaNode::ConstructorDecl {
                name,
                parameters,
                body,
                span,
                ..
            } => convert_decl::convert_constructor_decl(name, parameters, body, span, walk_child),

            JavaNode::FieldDecl {
                name,
                field_type,
                initializer,
                span,
                ..
            } => convert_decl::convert_field_decl(
                name,
                field_type,
                initializer.as_deref(),
                span,
                walk_child,
                current_class,
            ),

            JavaNode::Parameter { name, span, .. } => Ok(vec![GoNode::Ident {
                name: name.clone(),
                span: span.clone(),
            }]),

            JavaNode::AnnotationExpr { .. } => Ok(vec![]),

            // ── Statements ──────────────────────────────────────────

            JavaNode::BlockStmt { statements, span } => {
                convert_stmt::convert_block_stmt(statements, span, walk_child)
            }

            JavaNode::ReturnStmt { value, span } => {
                convert_stmt::convert_return_stmt(value.as_deref(), span, walk_child)
            }

            JavaNode::IfStmt {
                condition,
                then_branch,
                else_branch,
                span,
            } => convert_stmt::convert_if_stmt(
                condition,
                then_branch,
                else_branch.as_deref(),
                span,
                walk_child,
            ),

            JavaNode::ForStmt {
                init,
                condition,
                update,
                body,
                span,
            } => convert_stmt::convert_for_stmt(
                init.as_deref(),
                condition.as_deref(),
                update,
                body,
                span,
                walk_child,
            ),

            JavaNode::ForEachStmt {
                variable,
                iterable,
                body,
                span,
                ..
            } => convert_stmt::convert_foreach_stmt(variable, iterable, body, span, walk_child),

            JavaNode::WhileStmt {
                condition,
                body,
                span,
            } => convert_stmt::convert_while_stmt(condition, body, span, walk_child),

            JavaNode::TryCatchStmt {
                try_block,
                catches,
                finally_block,
                span,
            } => convert_stmt::convert_try_catch_stmt(
                try_block,
                catches,
                finally_block.as_deref(),
                span,
                walk_child,
            ),

            JavaNode::CatchClause {
                parameter,
                exception_types,
                body,
                span,
            } => convert_stmt::convert_catch_clause(
                parameter,
                exception_types,
                body,
                span,
                walk_child,
            ),

            JavaNode::ThrowStmt { expression, span } => {
                convert_stmt::convert_throw_stmt(expression, span, walk_child)
            }

            // ── Expressions ─────────────────────────────────────────

            JavaNode::MethodCallExpr {
                object,
                name,
                arguments,
                span,
                ..
            } => convert_expr::convert_method_call_expr(
                object.as_deref(),
                name,
                arguments,
                span,
                walk_child,
            ),

            JavaNode::FieldAccessExpr {
                object,
                field,
                span,
            } => convert_expr::convert_field_access_expr(object, field, span, walk_child),

            JavaNode::NameExpr { name, span } => {
                convert_expr::convert_name_expr(name, span, current_class, class_fields)
            }

            JavaNode::LiteralExpr { value, span } => {
                convert_expr::convert_literal_expr(value, span)
            }

            JavaNode::BinaryExpr {
                left,
                op,
                right,
                span,
            } => convert_expr::convert_binary_expr(left, op, right, span, walk_child),

            JavaNode::UnaryExpr { op, operand, span } => {
                convert_expr::convert_unary_expr(op, operand, span, walk_child)
            }

            JavaNode::AssignExpr {
                target,
                value,
                span,
            } => convert_expr::convert_assign_expr(target, value, span, walk_child),

            JavaNode::VariableDecl {
                name,
                var_type,
                initializer,
                span,
                ..
            } => convert_expr::convert_variable_decl(
                name,
                var_type.as_ref(),
                initializer.as_deref(),
                span,
                walk_child,
            ),

            JavaNode::NewExpr {
                class_type,
                arguments,
                span,
            } => convert_expr::convert_new_expr(class_type, arguments, span, walk_child),

            JavaNode::TernaryExpr {
                condition,
                then_expr,
                else_expr,
                span,
            } => convert_expr::convert_ternary_expr(
                condition, then_expr, else_expr, span, walk_child,
            ),

            JavaNode::CastExpr {
                target_type,
                expression,
                span,
            } => convert_expr::convert_cast_expr(target_type, expression, span, walk_child),

            JavaNode::LambdaExpr {
                parameters,
                body,
                span,
            } => convert_expr::convert_lambda_expr(parameters, body, span, walk_child),

            JavaNode::TypeRef { java_type, span } => {
                convert_expr::convert_type_ref(java_type, span)
            }
        }
    }
}

impl Default for DefaultConverter {
    fn default() -> Self {
        Self::new()
    }
}
