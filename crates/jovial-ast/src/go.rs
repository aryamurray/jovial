use serde::{Deserialize, Serialize};

use crate::span::Span;

/// A complete Go source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoFile {
    pub package: String,
    pub imports: Vec<GoImport>,
    pub nodes: Vec<GoNode>,
}

/// Go import declaration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoImport {
    pub path: String,
    pub alias: Option<String>,
}

/// Go type reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoType {
    pub name: String,
    pub package: Option<String>,
    pub is_pointer: bool,
    pub is_slice: bool,
    pub is_map: bool,
    pub key_type: Option<Box<GoType>>,
    pub value_type: Option<Box<GoType>>,
}

/// Go method receiver.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoReceiver {
    pub name: String,
    pub receiver_type: GoType,
    pub is_pointer: bool,
}

/// Go function/method parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoParam {
    pub name: String,
    pub param_type: GoType,
}

/// Go literal value kinds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GoLiteralValue {
    Int(i64),
    Float(f64),
    String(String),
    Rune(char),
    Bool(bool),
    Nil,
}

/// Go binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoBinaryOp {
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
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// Go unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoUnaryOp {
    Negate,
    Not,
    BitwiseNot,
    Deref,
    Addr,
}

/// Go AST node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GoNode {
    Package {
        name: String,
    },
    FuncDecl {
        name: String,
        receiver: Option<GoReceiver>,
        params: Vec<GoParam>,
        returns: Vec<GoType>,
        body: Vec<GoNode>,
        span: Span,
    },
    StructDecl {
        name: String,
        embedded: Vec<GoType>,
        fields: Vec<GoNode>,
        span: Span,
    },
    InterfaceDecl {
        name: String,
        methods: Vec<GoNode>,
        span: Span,
    },
    FieldDecl {
        name: String,
        field_type: GoType,
        tag: Option<String>,
        span: Span,
    },
    VarDecl {
        name: String,
        var_type: Option<GoType>,
        value: Option<Box<GoNode>>,
        span: Span,
    },
    ConstDecl {
        name: String,
        const_type: Option<GoType>,
        value: Box<GoNode>,
        span: Span,
    },
    ReturnStmt {
        values: Vec<GoNode>,
        span: Span,
    },
    IfStmt {
        init: Option<Box<GoNode>>,
        condition: Box<GoNode>,
        body: Vec<GoNode>,
        else_body: Option<Vec<GoNode>>,
        span: Span,
    },
    ForStmt {
        init: Option<Box<GoNode>>,
        condition: Option<Box<GoNode>>,
        post: Option<Box<GoNode>>,
        body: Vec<GoNode>,
        span: Span,
    },
    RangeStmt {
        key: Option<String>,
        value: Option<String>,
        iterable: Box<GoNode>,
        body: Vec<GoNode>,
        span: Span,
    },
    AssignStmt {
        lhs: Vec<GoNode>,
        rhs: Vec<GoNode>,
        define: bool,
        span: Span,
    },
    CallExpr {
        function: Box<GoNode>,
        args: Vec<GoNode>,
        span: Span,
    },
    SelectorExpr {
        object: Box<GoNode>,
        field: String,
        span: Span,
    },
    Ident {
        name: String,
        span: Span,
    },
    Literal {
        value: GoLiteralValue,
        span: Span,
    },
    BinaryExpr {
        left: Box<GoNode>,
        op: GoBinaryOp,
        right: Box<GoNode>,
        span: Span,
    },
    UnaryExpr {
        op: GoUnaryOp,
        operand: Box<GoNode>,
        span: Span,
    },
    CompositeLit {
        lit_type: GoType,
        elements: Vec<GoNode>,
        span: Span,
    },
    KeyValue {
        key: Box<GoNode>,
        value: Box<GoNode>,
        span: Span,
    },
    DeferStmt {
        call: Box<GoNode>,
        span: Span,
    },
    GoStmt {
        call: Box<GoNode>,
        span: Span,
    },
    BlockStmt {
        statements: Vec<GoNode>,
        span: Span,
    },
    TypeRef {
        go_type: GoType,
        span: Span,
    },
    RawCode {
        code: String,
        span: Span,
    },
    /// Type assertion expression: `expr.(Type)`
    TypeAssertExpr {
        expr: Box<GoNode>,
        assert_type: GoType,
        span: Span,
    },
    /// Grouped const block: `const ( ... )`
    ConstBlock {
        decls: Vec<GoNode>,
        span: Span,
    },
    /// Increment/decrement statement: `expr++` or `expr--`
    IncDecStmt {
        operand: Box<GoNode>,
        is_increment: bool,
        span: Span,
    },
}
