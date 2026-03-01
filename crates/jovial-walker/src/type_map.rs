/// Map a Java type name to its Go equivalent.
pub fn java_to_go_type(java_type: &str) -> &str {
    match java_type {
        // Primitives
        "boolean" | "Boolean" | "java.lang.Boolean" => "bool",
        "byte" | "Byte" | "java.lang.Byte" => "byte",
        "short" | "Short" | "java.lang.Short" => "int16",
        "int" | "Integer" | "java.lang.Integer" => "int",
        "long" | "Long" | "java.lang.Long" => "int64",
        "float" | "Float" | "java.lang.Float" => "float32",
        "double" | "Double" | "java.lang.Double" => "float64",
        "char" | "Character" | "java.lang.Character" => "rune",
        "void" | "Void" | "java.lang.Void" => "",

        // Strings
        "String" | "java.lang.String" => "string",

        // Collections
        "List" | "java.util.List" | "ArrayList" | "java.util.ArrayList"
        | "LinkedList" | "java.util.LinkedList" => "[]",
        "Set" | "java.util.Set" | "HashSet" | "java.util.HashSet"
        | "TreeSet" | "java.util.TreeSet" => "[]",
        "Map" | "java.util.Map" | "HashMap" | "java.util.HashMap"
        | "TreeMap" | "java.util.TreeMap" | "LinkedHashMap" | "java.util.LinkedHashMap" => "map",

        // Common types
        "Object" | "java.lang.Object" => "interface{}",
        "Optional" | "java.util.Optional" => "*",
        "BigDecimal" | "java.math.BigDecimal" => "float64",
        "BigInteger" | "java.math.BigInteger" => "int64",
        "Date" | "java.util.Date" | "LocalDate" | "java.time.LocalDate"
        | "LocalDateTime" | "java.time.LocalDateTime"
        | "Instant" | "java.time.Instant" => "time.Time",
        "Duration" | "java.time.Duration" => "time.Duration",
        "UUID" | "java.util.UUID" => "string",

        // Fallback: return as-is
        other => other,
    }
}

/// Map a Java binary operator to its Go equivalent string.
pub fn java_to_go_operator(op: &str) -> &str {
    match op {
        "+" => "+",
        "-" => "-",
        "*" => "*",
        "/" => "/",
        "%" => "%",
        "&&" => "&&",
        "||" => "||",
        "&" => "&",
        "|" => "|",
        "^" => "^",
        "<<" => "<<",
        ">>" => ">>",
        ">>>" => ">>", // Go doesn't have unsigned shift right; caller may need special handling
        "==" => "==",
        "!=" => "!=",
        "<" => "<",
        ">" => ">",
        "<=" => "<=",
        ">=" => ">=",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_mapping() {
        assert_eq!(java_to_go_type("int"), "int");
        assert_eq!(java_to_go_type("boolean"), "bool");
        assert_eq!(java_to_go_type("double"), "float64");
        assert_eq!(java_to_go_type("String"), "string");
    }

    #[test]
    fn test_collection_mapping() {
        assert_eq!(java_to_go_type("List"), "[]");
        assert_eq!(java_to_go_type("Map"), "map");
        assert_eq!(java_to_go_type("HashMap"), "map");
    }

    #[test]
    fn test_operator_mapping() {
        assert_eq!(java_to_go_operator("&&"), "&&");
        assert_eq!(java_to_go_operator(">>>"), ">>");
        assert_eq!(java_to_go_operator("=="), "==");
    }

    #[test]
    fn test_unknown_type_passthrough() {
        assert_eq!(java_to_go_type("MyCustomClass"), "MyCustomClass");
    }
}
