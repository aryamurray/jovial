use serde::{Deserialize, Serialize};

use crate::span::Span;

/// A complete Java source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JavaCompilationUnit {
    pub package: Option<String>,
    pub imports: Vec<String>,
    pub types: Vec<JavaNode>,
    pub span: Span,
}

/// Java type reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JavaType {
    pub name: String,
    pub type_args: Vec<JavaType>,
    pub array_dimensions: usize,
    pub is_varargs: bool,
}

/// Access and other modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Modifier {
    Public,
    Private,
    Protected,
    Static,
    Final,
    Abstract,
    Synchronized,
    Native,
    Transient,
    Volatile,
    Default,
}

/// Literal value kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralValue {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    And,
    Or,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    InstanceOf,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Negate,
    Not,
    BitwiseNot,
    PreIncrement,
    PreDecrement,
    PostIncrement,
    PostDecrement,
}

/// Java AST node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JavaNode {
    ClassDecl {
        name: String,
        modifiers: Vec<Modifier>,
        superclass: Option<JavaType>,
        interfaces: Vec<JavaType>,
        annotations: Vec<Box<JavaNode>>,
        members: Vec<JavaNode>,
        span: Span,
    },
    InterfaceDecl {
        name: String,
        modifiers: Vec<Modifier>,
        extends: Vec<JavaType>,
        annotations: Vec<Box<JavaNode>>,
        members: Vec<JavaNode>,
        span: Span,
    },
    EnumDecl {
        name: String,
        modifiers: Vec<Modifier>,
        constants: Vec<String>,
        annotations: Vec<Box<JavaNode>>,
        members: Vec<JavaNode>,
        span: Span,
    },
    MethodDecl {
        name: String,
        modifiers: Vec<Modifier>,
        return_type: Option<JavaType>,
        parameters: Vec<JavaNode>,
        annotations: Vec<Box<JavaNode>>,
        body: Option<Box<JavaNode>>,
        throws: Vec<JavaType>,
        span: Span,
    },
    ConstructorDecl {
        name: String,
        modifiers: Vec<Modifier>,
        parameters: Vec<JavaNode>,
        annotations: Vec<Box<JavaNode>>,
        body: Box<JavaNode>,
        throws: Vec<JavaType>,
        span: Span,
    },
    FieldDecl {
        name: String,
        modifiers: Vec<Modifier>,
        field_type: JavaType,
        initializer: Option<Box<JavaNode>>,
        annotations: Vec<Box<JavaNode>>,
        span: Span,
    },
    Parameter {
        name: String,
        param_type: JavaType,
        annotations: Vec<Box<JavaNode>>,
        span: Span,
    },
    AnnotationExpr {
        name: String,
        arguments: Vec<(String, Box<JavaNode>)>,
        span: Span,
    },
    BlockStmt {
        statements: Vec<JavaNode>,
        span: Span,
    },
    ReturnStmt {
        value: Option<Box<JavaNode>>,
        span: Span,
    },
    IfStmt {
        condition: Box<JavaNode>,
        then_branch: Box<JavaNode>,
        else_branch: Option<Box<JavaNode>>,
        span: Span,
    },
    ForStmt {
        init: Option<Box<JavaNode>>,
        condition: Option<Box<JavaNode>>,
        update: Vec<JavaNode>,
        body: Box<JavaNode>,
        span: Span,
    },
    ForEachStmt {
        variable: String,
        variable_type: JavaType,
        iterable: Box<JavaNode>,
        body: Box<JavaNode>,
        span: Span,
    },
    WhileStmt {
        condition: Box<JavaNode>,
        body: Box<JavaNode>,
        span: Span,
    },
    TryCatchStmt {
        try_block: Box<JavaNode>,
        catches: Vec<JavaNode>,
        finally_block: Option<Box<JavaNode>>,
        span: Span,
    },
    CatchClause {
        parameter: String,
        exception_types: Vec<JavaType>,
        body: Box<JavaNode>,
        span: Span,
    },
    ThrowStmt {
        expression: Box<JavaNode>,
        span: Span,
    },
    MethodCallExpr {
        object: Option<Box<JavaNode>>,
        name: String,
        arguments: Vec<JavaNode>,
        type_args: Vec<JavaType>,
        span: Span,
    },
    FieldAccessExpr {
        object: Box<JavaNode>,
        field: String,
        span: Span,
    },
    NameExpr {
        name: String,
        span: Span,
    },
    LiteralExpr {
        value: LiteralValue,
        span: Span,
    },
    BinaryExpr {
        left: Box<JavaNode>,
        op: BinaryOp,
        right: Box<JavaNode>,
        span: Span,
    },
    UnaryExpr {
        op: UnaryOp,
        operand: Box<JavaNode>,
        span: Span,
    },
    AssignExpr {
        target: Box<JavaNode>,
        value: Box<JavaNode>,
        span: Span,
    },
    VariableDecl {
        name: String,
        var_type: Option<JavaType>,
        initializer: Option<Box<JavaNode>>,
        is_final: bool,
        span: Span,
    },
    NewExpr {
        class_type: JavaType,
        arguments: Vec<JavaNode>,
        span: Span,
    },
    CastExpr {
        target_type: JavaType,
        expression: Box<JavaNode>,
        span: Span,
    },
    LambdaExpr {
        parameters: Vec<JavaNode>,
        body: Box<JavaNode>,
        span: Span,
    },
    TypeRef {
        java_type: JavaType,
        span: Span,
    },
}
