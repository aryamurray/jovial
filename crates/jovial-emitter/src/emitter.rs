use thiserror::Error;

use jovial_ast::go::{
    GoBinaryOp, GoFile, GoImport, GoLiteralValue, GoNode, GoParam, GoType, GoUnaryOp,
};

use crate::formatter::Formatter;

/// Errors during Go code emission.
#[derive(Debug, Error)]
pub enum EmitError {
    #[error("emit failed for node: {0}")]
    EmitFailed(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Emits Go source code from Go AST nodes.
pub struct GoEmitter {
    fmt: Formatter,
}

impl GoEmitter {
    pub fn new() -> Self {
        Self {
            fmt: Formatter::new(),
        }
    }

    // ── Type + literal + operator helpers ──────────────────────────

    fn emit_type(&self, t: &GoType) -> String {
        if t.is_map {
            let key = t
                .key_type
                .as_ref()
                .map(|k| self.emit_type(k))
                .unwrap_or_else(|| "string".into());
            let val = t
                .value_type
                .as_ref()
                .map(|v| self.emit_type(v))
                .unwrap_or_else(|| "interface{}".into());
            return format!("map[{key}]{val}");
        }

        let mut out = String::new();
        if t.is_pointer {
            out.push('*');
        }
        if t.is_slice {
            out.push_str("[]");
        }
        if let Some(ref pkg) = t.package {
            out.push_str(pkg);
            out.push('.');
        }
        out.push_str(&t.name);
        out
    }

    fn emit_literal(&self, value: &GoLiteralValue) -> String {
        match value {
            GoLiteralValue::Int(v) => v.to_string(),
            GoLiteralValue::Float(v) => {
                let s = v.to_string();
                if s.contains('.') {
                    s
                } else {
                    format!("{s}.0")
                }
            }
            GoLiteralValue::String(s) => format!("\"{s}\""),
            GoLiteralValue::Rune(c) => format!("'{c}'"),
            GoLiteralValue::Bool(b) => b.to_string(),
            GoLiteralValue::Nil => "nil".into(),
        }
    }

    fn emit_binary_op(&self, op: &GoBinaryOp) -> &'static str {
        match op {
            GoBinaryOp::Add => "+",
            GoBinaryOp::Sub => "-",
            GoBinaryOp::Mul => "*",
            GoBinaryOp::Div => "/",
            GoBinaryOp::Mod => "%",
            GoBinaryOp::And => "&&",
            GoBinaryOp::Or => "||",
            GoBinaryOp::BitwiseAnd => "&",
            GoBinaryOp::BitwiseOr => "|",
            GoBinaryOp::BitwiseXor => "^",
            GoBinaryOp::ShiftLeft => "<<",
            GoBinaryOp::ShiftRight => ">>",
            GoBinaryOp::Eq => "==",
            GoBinaryOp::Ne => "!=",
            GoBinaryOp::Lt => "<",
            GoBinaryOp::Gt => ">",
            GoBinaryOp::Le => "<=",
            GoBinaryOp::Ge => ">=",
        }
    }

    fn emit_unary_op(&self, op: &GoUnaryOp) -> &'static str {
        match op {
            GoUnaryOp::Negate => "-",
            GoUnaryOp::Not => "!",
            GoUnaryOp::BitwiseNot => "^",
            GoUnaryOp::Deref => "*",
            GoUnaryOp::Addr => "&",
        }
    }

    // ── Expression emission ────────────────────────────────────────

    /// Render an expression as a single-line string fragment.
    fn emit_expr_inline(&self, node: &GoNode) -> Result<String, EmitError> {
        match node {
            GoNode::Ident { name, .. } => Ok(name.clone()),
            GoNode::Literal { value, .. } => Ok(self.emit_literal(value)),
            GoNode::TypeRef { go_type, .. } => Ok(self.emit_type(go_type)),
            GoNode::BinaryExpr {
                left, op, right, ..
            } => {
                let l = self.emit_expr_inline(left)?;
                let r = self.emit_expr_inline(right)?;
                let op_str = self.emit_binary_op(op);
                Ok(format!("{l} {op_str} {r}"))
            }
            GoNode::UnaryExpr { op, operand, .. } => {
                let expr = self.emit_expr_inline(operand)?;
                let op_str = self.emit_unary_op(op);
                Ok(format!("{op_str}{expr}"))
            }
            GoNode::CallExpr {
                function, args, ..
            } => {
                let func = self.emit_expr_inline(function)?;
                let arg_strs: Result<Vec<_>, _> =
                    args.iter().map(|a| self.emit_expr_inline(a)).collect();
                Ok(format!("{}({})", func, arg_strs?.join(", ")))
            }
            GoNode::SelectorExpr { object, field, .. } => {
                let obj = self.emit_expr_inline(object)?;
                Ok(format!("{obj}.{field}"))
            }
            GoNode::KeyValue { key, value, .. } => {
                let k = self.emit_expr_inline(key)?;
                let v = self.emit_expr_inline(value)?;
                Ok(format!("{k}: {v}"))
            }
            GoNode::CompositeLit {
                lit_type, elements, ..
            } => {
                let type_str = self.emit_type(lit_type);
                if elements.is_empty() {
                    Ok(format!("{type_str}{{}}"))
                } else if elements.len() <= 2 {
                    let elems: Result<Vec<_>, _> =
                        elements.iter().map(|e| self.emit_expr_inline(e)).collect();
                    Ok(format!("{type_str}{{{}}}", elems?.join(", ")))
                } else {
                    // Inline fallback for >2 elements (V1 limitation in composed positions)
                    let elems: Result<Vec<_>, _> =
                        elements.iter().map(|e| self.emit_expr_inline(e)).collect();
                    Ok(format!("{type_str}{{{}}}", elems?.join(", ")))
                }
            }
            GoNode::FuncDecl {
                params,
                returns,
                body,
                ..
            } => {
                // Anonymous function (lambda) inline rendering
                let params_str = self.emit_params(params);
                let ret_str = self.emit_return_types(returns);
                if body.len() <= 1 {
                    let mut out = format!("func({params_str}){ret_str} {{");
                    if let Some(stmt) = body.first() {
                        // Inline the single statement
                        let mut tmp = GoEmitter::new();
                        tmp.emit_node(stmt)?;
                        let inner = tmp.fmt.output();
                        let trimmed = inner.trim();
                        out.push_str(&format!(" {trimmed} "));
                    }
                    out.push('}');
                    Ok(out)
                } else {
                    // Multi-line fallback — flatten for V1
                    let mut tmp = GoEmitter::new();
                    for stmt in body {
                        tmp.emit_node(stmt)?;
                    }
                    let inner = tmp.fmt.output();
                    let trimmed = inner.trim();
                    Ok(format!(
                        "func({params_str}){ret_str} {{ {trimmed} }}"
                    ))
                }
            }
            GoNode::RawCode { code, .. } => Ok(code.clone()),
            _ => Err(EmitError::EmitFailed(format!(
                "cannot inline expression: {node:?}"
            ))),
        }
    }

    /// Write an expression to the Formatter, handling multi-line cases.
    fn emit_expr_to_fmt(&mut self, node: &GoNode) -> Result<(), EmitError> {
        match node {
            GoNode::CompositeLit {
                lit_type,
                elements,
                ..
            } if elements.len() > 2 => {
                let type_str = self.emit_type(lit_type);
                self.fmt.write(&format!("{type_str}{{"));
                self.fmt.flush_line();
                self.fmt.indent();
                for elem in elements {
                    let elem_str = self.emit_expr_inline(elem)?;
                    self.fmt.write_line(&format!("{elem_str},"));
                }
                self.fmt.dedent();
                self.fmt.write("}");
                Ok(())
            }
            GoNode::FuncDecl {
                params,
                returns,
                body,
                name,
                ..
            } if name.is_empty() && body.len() > 1 => {
                let params_str = self.emit_params(params);
                let ret_str = self.emit_return_types(returns);
                self.fmt.write(&format!("func({params_str}){ret_str} {{"));
                self.fmt.flush_line();
                self.fmt.indent();
                for stmt in body {
                    self.emit_node(stmt)?;
                }
                self.fmt.dedent();
                self.fmt.write("}");
                Ok(())
            }
            _ => {
                let s = self.emit_expr_inline(node)?;
                self.fmt.write(&s);
                Ok(())
            }
        }
    }

    // ── Parameter / return-type helpers ─────────────────────────────

    fn emit_params(&self, params: &[GoParam]) -> String {
        params
            .iter()
            .map(|p| {
                let t = self.emit_type(&p.param_type);
                format!("{} {t}", p.name)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn emit_return_types(&self, returns: &[GoType]) -> String {
        match returns.len() {
            0 => String::new(),
            1 => format!(" {}", self.emit_type(&returns[0])),
            _ => {
                let types: Vec<_> = returns.iter().map(|t| self.emit_type(t)).collect();
                format!(" ({})", types.join(", "))
            }
        }
    }

    // ── Simple statement emitters ──────────────────────────────────

    fn emit_assign_stmt(
        &mut self,
        lhs: &[GoNode],
        rhs: &[GoNode],
        define: bool,
    ) -> Result<(), EmitError> {
        let lhs_strs: Result<Vec<_>, _> = lhs.iter().map(|n| self.emit_expr_inline(n)).collect();
        let rhs_strs: Result<Vec<_>, _> = rhs.iter().map(|n| self.emit_expr_inline(n)).collect();
        let op = if define { ":=" } else { "=" };
        self.fmt.write_line(&format!(
            "{} {op} {}",
            lhs_strs?.join(", "),
            rhs_strs?.join(", ")
        ));
        Ok(())
    }

    fn emit_return_stmt(&mut self, values: &[GoNode]) -> Result<(), EmitError> {
        if values.is_empty() {
            self.fmt.write_line("return");
        } else {
            let val_strs: Result<Vec<_>, _> =
                values.iter().map(|v| self.emit_expr_inline(v)).collect();
            self.fmt.write_line(&format!("return {}", val_strs?.join(", ")));
        }
        Ok(())
    }

    fn emit_var_decl(
        &mut self,
        name: &str,
        var_type: &Option<GoType>,
        value: &Option<Box<GoNode>>,
    ) -> Result<(), EmitError> {
        let mut line = format!("var {name}");
        if let Some(t) = var_type {
            line.push_str(&format!(" {}", self.emit_type(t)));
        }
        if let Some(v) = value {
            let val = self.emit_expr_inline(v)?;
            line.push_str(&format!(" = {val}"));
        }
        self.fmt.write_line(&line);
        Ok(())
    }

    fn emit_const_decl(
        &mut self,
        name: &str,
        const_type: &Option<GoType>,
        value: &GoNode,
    ) -> Result<(), EmitError> {
        let mut line = format!("const {name}");
        if let Some(t) = const_type {
            line.push_str(&format!(" {}", self.emit_type(t)));
        }
        let val = self.emit_expr_inline(value)?;
        line.push_str(&format!(" = {val}"));
        self.fmt.write_line(&line);
        Ok(())
    }

    fn emit_field_decl(
        &mut self,
        name: &str,
        field_type: &GoType,
        tag: &Option<String>,
    ) -> Result<(), EmitError> {
        let type_str = self.emit_type(field_type);
        match tag {
            Some(t) => self.fmt.write_line(&format!("{name} {type_str} `{t}`")),
            None => self.fmt.write_line(&format!("{name} {type_str}")),
        }
        Ok(())
    }

    // ── Compound statement emitters ────────────────────────────────

    fn emit_if_stmt(
        &mut self,
        init: &Option<Box<GoNode>>,
        condition: &GoNode,
        body: &[GoNode],
        else_body: &Option<Vec<GoNode>>,
    ) -> Result<(), EmitError> {
        let cond = self.emit_expr_inline(condition)?;
        let init_str = match init {
            Some(init_node) => {
                let s = self.emit_init_inline(init_node)?;
                format!("{s}; ")
            }
            None => String::new(),
        };
        self.fmt.write_line(&format!("if {init_str}{cond} {{"));
        self.fmt.indent();
        for stmt in body {
            self.emit_node(stmt)?;
        }
        self.fmt.dedent();

        match else_body {
            Some(else_stmts) if else_stmts.len() == 1 => {
                // Check for else-if chain
                if let GoNode::IfStmt {
                    init: ei_init,
                    condition: ei_cond,
                    body: ei_body,
                    else_body: ei_else,
                    ..
                } = &else_stmts[0]
                {
                    let ei_cond_str = self.emit_expr_inline(ei_cond)?;
                    let ei_init_str = match ei_init {
                        Some(n) => {
                            let s = self.emit_init_inline(n)?;
                            format!("{s}; ")
                        }
                        None => String::new(),
                    };
                    self.fmt
                        .write_line(&format!("}} else if {ei_init_str}{ei_cond_str} {{"));
                    self.fmt.indent();
                    for stmt in ei_body {
                        self.emit_node(stmt)?;
                    }
                    self.fmt.dedent();
                    // Recurse for further else-if / else
                    if ei_else.is_some() {
                        self.emit_else_tail(ei_else)?;
                    } else {
                        self.fmt.write_line("}");
                    }
                } else {
                    // Single non-if statement in else
                    self.fmt.write_line("} else {");
                    self.fmt.indent();
                    for stmt in else_stmts {
                        self.emit_node(stmt)?;
                    }
                    self.fmt.dedent();
                    self.fmt.write_line("}");
                }
            }
            Some(else_stmts) if !else_stmts.is_empty() => {
                self.fmt.write_line("} else {");
                self.fmt.indent();
                for stmt in else_stmts {
                    self.emit_node(stmt)?;
                }
                self.fmt.dedent();
                self.fmt.write_line("}");
            }
            _ => {
                self.fmt.write_line("}");
            }
        }
        Ok(())
    }

    fn emit_else_tail(&mut self, else_body: &Option<Vec<GoNode>>) -> Result<(), EmitError> {
        match else_body {
            Some(stmts) if stmts.len() == 1 => {
                if let GoNode::IfStmt {
                    init,
                    condition,
                    body,
                    else_body: next_else,
                    ..
                } = &stmts[0]
                {
                    let cond = self.emit_expr_inline(condition)?;
                    let init_str = match init {
                        Some(n) => {
                            let s = self.emit_init_inline(n)?;
                            format!("{s}; ")
                        }
                        None => String::new(),
                    };
                    self.fmt
                        .write_line(&format!("}} else if {init_str}{cond} {{"));
                    self.fmt.indent();
                    for stmt in body {
                        self.emit_node(stmt)?;
                    }
                    self.fmt.dedent();
                    if next_else.is_some() {
                        self.emit_else_tail(next_else)?;
                    } else {
                        self.fmt.write_line("}");
                    }
                } else {
                    self.fmt.write_line("} else {");
                    self.fmt.indent();
                    for stmt in stmts {
                        self.emit_node(stmt)?;
                    }
                    self.fmt.dedent();
                    self.fmt.write_line("}");
                }
            }
            Some(stmts) if !stmts.is_empty() => {
                self.fmt.write_line("} else {");
                self.fmt.indent();
                for stmt in stmts {
                    self.emit_node(stmt)?;
                }
                self.fmt.dedent();
                self.fmt.write_line("}");
            }
            _ => {
                self.fmt.write_line("}");
            }
        }
        Ok(())
    }

    /// Emit an init statement inline (for if/for init clauses).
    fn emit_init_inline(&self, node: &GoNode) -> Result<String, EmitError> {
        match node {
            GoNode::AssignStmt {
                lhs, rhs, define, ..
            } => {
                let lhs_strs: Result<Vec<_>, _> =
                    lhs.iter().map(|n| self.emit_expr_inline(n)).collect();
                let rhs_strs: Result<Vec<_>, _> =
                    rhs.iter().map(|n| self.emit_expr_inline(n)).collect();
                let op = if *define { ":=" } else { "=" };
                Ok(format!("{} {op} {}", lhs_strs?.join(", "), rhs_strs?.join(", ")))
            }
            GoNode::VarDecl {
                name,
                var_type,
                value,
                ..
            } => {
                let mut s = format!("var {name}");
                if let Some(t) = var_type {
                    s.push_str(&format!(" {}", self.emit_type(t)));
                }
                if let Some(v) = value {
                    let val = self.emit_expr_inline(v)?;
                    s.push_str(&format!(" = {val}"));
                }
                Ok(s)
            }
            _ => self.emit_expr_inline(node),
        }
    }

    fn emit_for_stmt(
        &mut self,
        init: &Option<Box<GoNode>>,
        condition: &Option<Box<GoNode>>,
        post: &Option<Box<GoNode>>,
        body: &[GoNode],
    ) -> Result<(), EmitError> {
        let has_init = init.is_some();
        let has_post = post.is_some();

        if !has_init && !has_post {
            // Condition-only or infinite loop
            if let Some(cond) = condition {
                let cond_str = self.emit_expr_inline(cond)?;
                self.fmt.write_line(&format!("for {cond_str} {{"));
            } else {
                self.fmt.write_line("for {");
            }
        } else {
            // Three-clause for
            let init_str = match init {
                Some(n) => self.emit_init_inline(n)?,
                None => String::new(),
            };
            let cond_str = match condition {
                Some(n) => self.emit_expr_inline(n)?,
                None => String::new(),
            };
            let post_str = match post {
                Some(n) => self.emit_init_inline(n)?,
                None => String::new(),
            };
            self.fmt
                .write_line(&format!("for {init_str}; {cond_str}; {post_str} {{"));
        }

        self.fmt.indent();
        for stmt in body {
            self.emit_node(stmt)?;
        }
        self.fmt.dedent();
        self.fmt.write_line("}");
        Ok(())
    }

    fn emit_range_stmt(
        &mut self,
        key: &Option<String>,
        value: &Option<String>,
        iterable: &GoNode,
        body: &[GoNode],
    ) -> Result<(), EmitError> {
        let iter_str = self.emit_expr_inline(iterable)?;
        let vars = match (key, value) {
            (Some(k), Some(v)) => format!("{k}, {v}"),
            (Some(k), None) => k.clone(),
            (None, Some(v)) => format!("_, {v}"),
            (None, None) => String::new(),
        };

        if vars.is_empty() {
            self.fmt.write_line(&format!("for range {iter_str} {{"));
        } else {
            self.fmt
                .write_line(&format!("for {vars} := range {iter_str} {{"));
        }

        self.fmt.indent();
        for stmt in body {
            self.emit_node(stmt)?;
        }
        self.fmt.dedent();
        self.fmt.write_line("}");
        Ok(())
    }

    // ── Declaration emitters ───────────────────────────────────────

    fn emit_func_decl(
        &mut self,
        name: &str,
        receiver: &Option<jovial_ast::go::GoReceiver>,
        params: &[GoParam],
        returns: &[GoType],
        body: &[GoNode],
    ) -> Result<(), EmitError> {
        let params_str = self.emit_params(params);
        let ret_str = self.emit_return_types(returns);

        let recv_str = match receiver {
            Some(r) => {
                let type_str = self.emit_type(&r.receiver_type);
                if r.is_pointer {
                    format!("({} *{type_str}) ", r.name)
                } else {
                    format!("({} {type_str}) ", r.name)
                }
            }
            None => String::new(),
        };

        self.fmt
            .write_line(&format!("func {recv_str}{name}({params_str}){ret_str} {{"));
        self.fmt.indent();
        for stmt in body {
            self.emit_node(stmt)?;
        }
        self.fmt.dedent();
        self.fmt.write_line("}");
        Ok(())
    }

    fn emit_struct_decl(&mut self, name: &str, fields: &[GoNode]) -> Result<(), EmitError> {
        self.fmt.write_line(&format!("type {name} struct {{"));
        self.fmt.indent();
        for field in fields {
            self.emit_node(field)?;
        }
        self.fmt.dedent();
        self.fmt.write_line("}");
        Ok(())
    }

    fn emit_interface_decl(&mut self, name: &str, methods: &[GoNode]) -> Result<(), EmitError> {
        self.fmt.write_line(&format!("type {name} interface {{"));
        self.fmt.indent();
        for method in methods {
            match method {
                GoNode::FuncDecl {
                    name: mname,
                    params,
                    returns,
                    ..
                } => {
                    let params_str = self.emit_params(params);
                    let ret_str = self.emit_return_types(returns);
                    self.fmt
                        .write_line(&format!("{mname}({params_str}){ret_str}"));
                }
                _ => self.emit_node(method)?,
            }
        }
        self.fmt.dedent();
        self.fmt.write_line("}");
        Ok(())
    }

    // ── Import emission ────────────────────────────────────────────

    fn emit_imports(&mut self, imports: &[GoImport]) -> Result<(), EmitError> {
        if imports.is_empty() {
            return Ok(());
        }

        let mut stdlib: Vec<String> = Vec::new();
        let mut external: Vec<String> = Vec::new();

        for imp in imports {
            let rendered = match &imp.alias {
                Some(alias) => format!("{alias} \"{path}\"", path = imp.path),
                None => format!("\"{path}\"", path = imp.path),
            };
            if imp.path.contains('.') {
                if !external.contains(&rendered) {
                    external.push(rendered);
                }
            } else if !stdlib.contains(&rendered) {
                stdlib.push(rendered);
            }
        }

        stdlib.sort();
        external.sort();

        let total = stdlib.len() + external.len();
        if total == 1 {
            let path = if !stdlib.is_empty() {
                &stdlib[0]
            } else {
                &external[0]
            };
            self.fmt.write_line(&format!("import {path}"));
            return Ok(());
        }

        self.fmt.write_line("import (");
        self.fmt.indent();
        for path in &stdlib {
            self.fmt.write_line(path);
        }
        if !stdlib.is_empty() && !external.is_empty() {
            self.fmt.blank_line();
        }
        for path in &external {
            self.fmt.write_line(path);
        }
        self.fmt.dedent();
        self.fmt.write_line(")");
        Ok(())
    }

    // ── Node dispatch ──────────────────────────────────────────────

    /// Emit a single Go AST node.
    pub fn emit_node(&mut self, node: &GoNode) -> Result<(), EmitError> {
        match node {
            GoNode::Package { name } => {
                self.fmt.write_line(&format!("package {name}"));
            }
            GoNode::FuncDecl {
                name,
                receiver,
                params,
                returns,
                body,
                ..
            } => {
                if name.is_empty() {
                    // Anonymous function at statement level
                    self.emit_expr_to_fmt(node)?;
                    self.fmt.flush_line();
                } else {
                    self.emit_func_decl(name, receiver, params, returns, body)?;
                }
            }
            GoNode::StructDecl { name, fields, .. } => {
                self.emit_struct_decl(name, fields)?;
            }
            GoNode::InterfaceDecl { name, methods, .. } => {
                self.emit_interface_decl(name, methods)?;
            }
            GoNode::FieldDecl {
                name,
                field_type,
                tag,
                ..
            } => {
                self.emit_field_decl(name, field_type, tag)?;
            }
            GoNode::VarDecl {
                name,
                var_type,
                value,
                ..
            } => {
                self.emit_var_decl(name, var_type, value)?;
            }
            GoNode::ConstDecl {
                name,
                const_type,
                value,
                ..
            } => {
                self.emit_const_decl(name, const_type, value)?;
            }
            GoNode::ReturnStmt { values, .. } => {
                self.emit_return_stmt(values)?;
            }
            GoNode::IfStmt {
                init,
                condition,
                body,
                else_body,
                ..
            } => {
                self.emit_if_stmt(init, condition, body, else_body)?;
            }
            GoNode::ForStmt {
                init,
                condition,
                post,
                body,
                ..
            } => {
                self.emit_for_stmt(init, condition, post, body)?;
            }
            GoNode::RangeStmt {
                key,
                value,
                iterable,
                body,
                ..
            } => {
                self.emit_range_stmt(key, value, iterable, body)?;
            }
            GoNode::AssignStmt {
                lhs, rhs, define, ..
            } => {
                self.emit_assign_stmt(lhs, rhs, *define)?;
            }
            GoNode::DeferStmt { call, .. } => {
                let call_str = self.emit_expr_inline(call)?;
                self.fmt.write_line(&format!("defer {call_str}"));
            }
            GoNode::GoStmt { call, .. } => {
                let call_str = self.emit_expr_inline(call)?;
                self.fmt.write_line(&format!("go {call_str}"));
            }
            GoNode::BlockStmt { statements, .. } => {
                for stmt in statements {
                    self.emit_node(stmt)?;
                }
            }
            GoNode::RawCode { code, .. } => {
                for line in code.lines() {
                    self.fmt.write_line(line);
                }
            }
            // Expression-only nodes at statement level
            GoNode::CallExpr { .. }
            | GoNode::SelectorExpr { .. }
            | GoNode::CompositeLit { .. }
            | GoNode::Ident { .. }
            | GoNode::Literal { .. }
            | GoNode::BinaryExpr { .. }
            | GoNode::UnaryExpr { .. }
            | GoNode::KeyValue { .. }
            | GoNode::TypeRef { .. } => {
                self.emit_expr_to_fmt(node)?;
                self.fmt.flush_line();
            }
        }
        Ok(())
    }

    // ── File emission ──────────────────────────────────────────────

    /// Emit a complete Go source file.
    pub fn emit_file(&mut self, file: &GoFile) -> Result<String, EmitError> {
        self.fmt = Formatter::new();

        self.fmt.write_line(&format!("package {}", file.package));

        if !file.imports.is_empty() {
            self.fmt.blank_line();
            self.emit_imports(&file.imports)?;
        }

        for (i, node) in file.nodes.iter().enumerate() {
            if i == 0 || !file.imports.is_empty() || i > 0 {
                self.fmt.blank_line();
            }
            self.emit_node(node)?;
        }

        let mut output = self.fmt.output();
        if !output.ends_with('\n') {
            output.push('\n');
        }
        Ok(output)
    }
}

impl Default for GoEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jovial_ast::go::*;
    use jovial_ast::span::Span;

    fn span() -> Span {
        Span {
            file: String::new(),
            line_start: 0,
            col_start: 0,
            line_end: 0,
            col_end: 0,
        }
    }

    fn simple_type(name: &str) -> GoType {
        GoType {
            name: name.to_string(),
            package: None,
            is_pointer: false,
            is_slice: false,
            is_map: false,
            key_type: None,
            value_type: None,
        }
    }

    #[test]
    fn type_pointer() {
        let emitter = GoEmitter::new();
        let t = GoType {
            is_pointer: true,
            ..simple_type("Foo")
        };
        insta::assert_snapshot!(emitter.emit_type(&t), @"*Foo");
    }

    #[test]
    fn type_slice() {
        let emitter = GoEmitter::new();
        let t = GoType {
            is_slice: true,
            ..simple_type("string")
        };
        insta::assert_snapshot!(emitter.emit_type(&t), @"[]string");
    }

    #[test]
    fn type_map() {
        let emitter = GoEmitter::new();
        let t = GoType {
            name: String::new(),
            package: None,
            is_pointer: false,
            is_slice: false,
            is_map: true,
            key_type: Some(Box::new(simple_type("string"))),
            value_type: Some(Box::new(simple_type("int"))),
        };
        insta::assert_snapshot!(emitter.emit_type(&t), @"map[string]int");
    }

    #[test]
    fn type_package_qualified() {
        let emitter = GoEmitter::new();
        let t = GoType {
            package: Some("http".into()),
            ..simple_type("Request")
        };
        insta::assert_snapshot!(emitter.emit_type(&t), @"http.Request");
    }

    #[test]
    fn simple_struct_with_tags() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::StructDecl {
            name: "User".into(),
            fields: vec![
                GoNode::FieldDecl {
                    name: "Name".into(),
                    field_type: simple_type("string"),
                    tag: Some("json:\"name\"".into()),
                    span: span(),
                },
                GoNode::FieldDecl {
                    name: "Age".into(),
                    field_type: simple_type("int"),
                    tag: Some("json:\"age\"".into()),
                    span: span(),
                },
            ],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r#"
        type User struct {
        	Name string `json:"name"`
        	Age int `json:"age"`
        }
        "#);
    }

    #[test]
    fn function_with_receiver() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::FuncDecl {
            name: "GetName".into(),
            receiver: Some(GoReceiver {
                name: "u".into(),
                receiver_type: simple_type("User"),
                is_pointer: true,
            }),
            params: vec![],
            returns: vec![simple_type("string")],
            body: vec![GoNode::ReturnStmt {
                values: vec![GoNode::SelectorExpr {
                    object: Box::new(GoNode::Ident {
                        name: "u".into(),
                        span: span(),
                    }),
                    field: "Name".into(),
                    span: span(),
                }],
                span: span(),
            }],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        func (u *User) GetName() string {
        	return u.Name
        }
        ");
    }

    #[test]
    fn if_else_if_else_chain() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::IfStmt {
            init: None,
            condition: Box::new(GoNode::BinaryExpr {
                left: Box::new(GoNode::Ident {
                    name: "x".into(),
                    span: span(),
                }),
                op: GoBinaryOp::Gt,
                right: Box::new(GoNode::Literal {
                    value: GoLiteralValue::Int(10),
                    span: span(),
                }),
                span: span(),
            }),
            body: vec![GoNode::ReturnStmt {
                values: vec![GoNode::Literal {
                    value: GoLiteralValue::String("big".into()),
                    span: span(),
                }],
                span: span(),
            }],
            else_body: Some(vec![GoNode::IfStmt {
                init: None,
                condition: Box::new(GoNode::BinaryExpr {
                    left: Box::new(GoNode::Ident {
                        name: "x".into(),
                        span: span(),
                    }),
                    op: GoBinaryOp::Gt,
                    right: Box::new(GoNode::Literal {
                        value: GoLiteralValue::Int(5),
                        span: span(),
                    }),
                    span: span(),
                }),
                body: vec![GoNode::ReturnStmt {
                    values: vec![GoNode::Literal {
                        value: GoLiteralValue::String("medium".into()),
                        span: span(),
                    }],
                    span: span(),
                }],
                else_body: Some(vec![GoNode::ReturnStmt {
                    values: vec![GoNode::Literal {
                        value: GoLiteralValue::String("small".into()),
                        span: span(),
                    }],
                    span: span(),
                }]),
                span: span(),
            }]),
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r#"
        if x > 10 {
        	return "big"
        } else if x > 5 {
        	return "medium"
        } else {
        	return "small"
        }
        "#);
    }

    #[test]
    fn for_three_clause() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::ForStmt {
            init: Some(Box::new(GoNode::AssignStmt {
                lhs: vec![GoNode::Ident {
                    name: "i".into(),
                    span: span(),
                }],
                rhs: vec![GoNode::Literal {
                    value: GoLiteralValue::Int(0),
                    span: span(),
                }],
                define: true,
                span: span(),
            })),
            condition: Some(Box::new(GoNode::BinaryExpr {
                left: Box::new(GoNode::Ident {
                    name: "i".into(),
                    span: span(),
                }),
                op: GoBinaryOp::Lt,
                right: Box::new(GoNode::Literal {
                    value: GoLiteralValue::Int(10),
                    span: span(),
                }),
                span: span(),
            })),
            post: Some(Box::new(GoNode::AssignStmt {
                lhs: vec![GoNode::Ident {
                    name: "i".into(),
                    span: span(),
                }],
                rhs: vec![GoNode::BinaryExpr {
                    left: Box::new(GoNode::Ident {
                        name: "i".into(),
                        span: span(),
                    }),
                    op: GoBinaryOp::Add,
                    right: Box::new(GoNode::Literal {
                        value: GoLiteralValue::Int(1),
                        span: span(),
                    }),
                    span: span(),
                }],
                define: false,
                span: span(),
            })),
            body: vec![GoNode::CallExpr {
                function: Box::new(GoNode::SelectorExpr {
                    object: Box::new(GoNode::Ident {
                        name: "fmt".into(),
                        span: span(),
                    }),
                    field: "Println".into(),
                    span: span(),
                }),
                args: vec![GoNode::Ident {
                    name: "i".into(),
                    span: span(),
                }],
                span: span(),
            }],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        for i := 0; i < 10; i = i + 1 {
        	fmt.Println(i)
        }
        ");
    }

    #[test]
    fn for_condition_only() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::ForStmt {
            init: None,
            condition: Some(Box::new(GoNode::Ident {
                name: "running".into(),
                span: span(),
            })),
            post: None,
            body: vec![GoNode::CallExpr {
                function: Box::new(GoNode::Ident {
                    name: "tick".into(),
                    span: span(),
                }),
                args: vec![],
                span: span(),
            }],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        for running {
        	tick()
        }
        ");
    }

    #[test]
    fn for_infinite() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::ForStmt {
            init: None,
            condition: None,
            post: None,
            body: vec![GoNode::CallExpr {
                function: Box::new(GoNode::Ident {
                    name: "spin".into(),
                    span: span(),
                }),
                args: vec![],
                span: span(),
            }],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        for {
        	spin()
        }
        ");
    }

    #[test]
    fn range_loop() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::RangeStmt {
            key: Some("i".into()),
            value: Some("v".into()),
            iterable: Box::new(GoNode::Ident {
                name: "items".into(),
                span: span(),
            }),
            body: vec![GoNode::CallExpr {
                function: Box::new(GoNode::SelectorExpr {
                    object: Box::new(GoNode::Ident {
                        name: "fmt".into(),
                        span: span(),
                    }),
                    field: "Println".into(),
                    span: span(),
                }),
                args: vec![
                    GoNode::Ident {
                        name: "i".into(),
                        span: span(),
                    },
                    GoNode::Ident {
                        name: "v".into(),
                        span: span(),
                    },
                ],
                span: span(),
            }],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        for i, v := range items {
        	fmt.Println(i, v)
        }
        ");
    }

    #[test]
    fn assign_and_define() {
        let mut emitter = GoEmitter::new();
        emitter
            .emit_node(&GoNode::AssignStmt {
                lhs: vec![GoNode::Ident {
                    name: "x".into(),
                    span: span(),
                }],
                rhs: vec![GoNode::Literal {
                    value: GoLiteralValue::Int(42),
                    span: span(),
                }],
                define: true,
                span: span(),
            })
            .unwrap();
        emitter
            .emit_node(&GoNode::AssignStmt {
                lhs: vec![GoNode::Ident {
                    name: "x".into(),
                    span: span(),
                }],
                rhs: vec![GoNode::Literal {
                    value: GoLiteralValue::Int(100),
                    span: span(),
                }],
                define: false,
                span: span(),
            })
            .unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        x := 42
        x = 100
        ");
    }

    #[test]
    fn return_multiple_values() {
        let mut emitter = GoEmitter::new();
        emitter
            .emit_node(&GoNode::ReturnStmt {
                values: vec![
                    GoNode::Ident {
                        name: "result".into(),
                        span: span(),
                    },
                    GoNode::Ident {
                        name: "nil".into(),
                        span: span(),
                    },
                ],
                span: span(),
            })
            .unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @"return result, nil");
    }

    #[test]
    fn defer_and_go_stmts() {
        let mut emitter = GoEmitter::new();
        emitter
            .emit_node(&GoNode::DeferStmt {
                call: Box::new(GoNode::CallExpr {
                    function: Box::new(GoNode::SelectorExpr {
                        object: Box::new(GoNode::Ident {
                            name: "f".into(),
                            span: span(),
                        }),
                        field: "Close".into(),
                        span: span(),
                    }),
                    args: vec![],
                    span: span(),
                }),
                span: span(),
            })
            .unwrap();
        emitter
            .emit_node(&GoNode::GoStmt {
                call: Box::new(GoNode::CallExpr {
                    function: Box::new(GoNode::Ident {
                        name: "serve".into(),
                        span: span(),
                    }),
                    args: vec![],
                    span: span(),
                }),
                span: span(),
            })
            .unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        defer f.Close()
        go serve()
        ");
    }

    #[test]
    fn composite_literal() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::CompositeLit {
            lit_type: simple_type("Config"),
            elements: vec![
                GoNode::KeyValue {
                    key: Box::new(GoNode::Ident {
                        name: "Host".into(),
                        span: span(),
                    }),
                    value: Box::new(GoNode::Literal {
                        value: GoLiteralValue::String("localhost".into()),
                        span: span(),
                    }),
                    span: span(),
                },
                GoNode::KeyValue {
                    key: Box::new(GoNode::Ident {
                        name: "Port".into(),
                        span: span(),
                    }),
                    value: Box::new(GoNode::Literal {
                        value: GoLiteralValue::Int(8080),
                        span: span(),
                    }),
                    span: span(),
                },
                GoNode::KeyValue {
                    key: Box::new(GoNode::Ident {
                        name: "Debug".into(),
                        span: span(),
                    }),
                    value: Box::new(GoNode::Literal {
                        value: GoLiteralValue::Bool(true),
                        span: span(),
                    }),
                    span: span(),
                },
            ],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r#"
        Config{
        	Host: "localhost",
        	Port: 8080,
        	Debug: true,
        }
        "#);
    }

    #[test]
    fn interface_decl() {
        let mut emitter = GoEmitter::new();
        let node = GoNode::InterfaceDecl {
            name: "Reader".into(),
            methods: vec![GoNode::FuncDecl {
                name: "Read".into(),
                receiver: None,
                params: vec![GoParam {
                    name: "p".into(),
                    param_type: GoType {
                        is_slice: true,
                        ..simple_type("byte")
                    },
                }],
                returns: vec![simple_type("int"), simple_type("error")],
                body: vec![],
                span: span(),
            }],
            span: span(),
        };
        emitter.emit_node(&node).unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r"
        type Reader interface {
        	Read(p []byte) (int, error)
        }
        ");
    }

    #[test]
    fn const_and_var_decls() {
        let mut emitter = GoEmitter::new();
        emitter
            .emit_node(&GoNode::ConstDecl {
                name: "MaxRetries".into(),
                const_type: Some(simple_type("int")),
                value: Box::new(GoNode::Literal {
                    value: GoLiteralValue::Int(3),
                    span: span(),
                }),
                span: span(),
            })
            .unwrap();
        emitter
            .emit_node(&GoNode::VarDecl {
                name: "count".into(),
                var_type: Some(simple_type("int")),
                value: None,
                span: span(),
            })
            .unwrap();
        emitter
            .emit_node(&GoNode::VarDecl {
                name: "name".into(),
                var_type: None,
                value: Some(Box::new(GoNode::Literal {
                    value: GoLiteralValue::String("world".into()),
                    span: span(),
                })),
                span: span(),
            })
            .unwrap();
        insta::assert_snapshot!(emitter.fmt.output(), @r#"
        const MaxRetries int = 3
        var count int
        var name = "world"
        "#);
    }

    #[test]
    fn complete_file() {
        let mut emitter = GoEmitter::new();
        let file = GoFile {
            package: "main".into(),
            imports: vec![GoImport {
                path: "fmt".into(),
                alias: None,
            }],
            nodes: vec![GoNode::FuncDecl {
                name: "main".into(),
                receiver: None,
                params: vec![],
                returns: vec![],
                body: vec![GoNode::CallExpr {
                    function: Box::new(GoNode::SelectorExpr {
                        object: Box::new(GoNode::Ident {
                            name: "fmt".into(),
                            span: span(),
                        }),
                        field: "Println".into(),
                        span: span(),
                    }),
                    args: vec![GoNode::Literal {
                        value: GoLiteralValue::String("Hello, World!".into()),
                        span: span(),
                    }],
                    span: span(),
                }],
                span: span(),
            }],
        };
        let output = emitter.emit_file(&file).unwrap();
        insta::assert_snapshot!(output, @r#"
        package main

        import "fmt"

        func main() {
        	fmt.Println("Hello, World!")
        }
        "#);
    }
}
