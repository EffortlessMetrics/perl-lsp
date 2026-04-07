use crate::pure_rust_parser::AstNode;
use crate::pure_rust_parser::Rule;
use pest::iterators::Pair;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Precedence(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Associativity {
    Left,
    Right,
    None,
}

pub struct OpInfo {
    pub precedence: Precedence,
    pub associativity: Associativity,
}

pub struct PrattParser {
    operators: HashMap<&'static str, OpInfo>,
}

impl Default for PrattParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PrattParser {
    pub fn new() -> Self {
        let mut operators = HashMap::new();

        // Perl operator precedence (from lowest to highest)
        // Level 1: List operators (rightward)
        operators
            .insert(",", OpInfo { precedence: Precedence(1), associativity: Associativity::Left });
        operators
            .insert("=>", OpInfo { precedence: Precedence(1), associativity: Associativity::Left });

        // Level 2: List operators (leftward)
        // These are handled specially in Perl

        // Level 3: Assignment operators
        operators
            .insert("=", OpInfo { precedence: Precedence(3), associativity: Associativity::Right });
        operators.insert(
            "+=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "-=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "*=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "/=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "%=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "**=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "&=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "|=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "^=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "&.=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "|.=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "^.=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "<<=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            ">>=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            ".=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "//=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "&&=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );
        operators.insert(
            "||=",
            OpInfo { precedence: Precedence(3), associativity: Associativity::Right },
        );

        // Level 4: Ternary conditional
        operators
            .insert("?", OpInfo { precedence: Precedence(4), associativity: Associativity::Right });
        operators
            .insert(":", OpInfo { precedence: Precedence(4), associativity: Associativity::Right });

        // Level 5: Range operators
        operators
            .insert("..", OpInfo { precedence: Precedence(5), associativity: Associativity::None });
        operators.insert(
            "...",
            OpInfo { precedence: Precedence(5), associativity: Associativity::None },
        );

        // Level 6: Logical or
        operators
            .insert("||", OpInfo { precedence: Precedence(6), associativity: Associativity::Left });

        // Level 7: Defined-or
        operators
            .insert("//", OpInfo { precedence: Precedence(7), associativity: Associativity::Left });

        // Level 8: Logical and
        operators
            .insert("&&", OpInfo { precedence: Precedence(8), associativity: Associativity::Left });

        // Level 9: Low precedence logical or/xor/and
        operators
            .insert("or", OpInfo { precedence: Precedence(9), associativity: Associativity::Left });
        operators.insert(
            "xor",
            OpInfo { precedence: Precedence(9), associativity: Associativity::Left },
        );

        // Level 10: Low precedence logical and
        operators.insert(
            "and",
            OpInfo { precedence: Precedence(10), associativity: Associativity::Left },
        );

        // Level 11: Low precedence not
        operators.insert(
            "not",
            OpInfo { precedence: Precedence(11), associativity: Associativity::Right },
        );

        // Level 12: Comma and list operators
        // Already added above

        // Level 13: Named unary operators
        // These are prefix operators handled separately

        // Level 14: Relational operators
        operators
            .insert("<", OpInfo { precedence: Precedence(14), associativity: Associativity::None });
        operators
            .insert(">", OpInfo { precedence: Precedence(14), associativity: Associativity::None });
        operators.insert(
            "<=",
            OpInfo { precedence: Precedence(14), associativity: Associativity::None },
        );
        operators.insert(
            ">=",
            OpInfo { precedence: Precedence(14), associativity: Associativity::None },
        );
        operators.insert(
            "lt",
            OpInfo { precedence: Precedence(14), associativity: Associativity::None },
        );
        operators.insert(
            "gt",
            OpInfo { precedence: Precedence(14), associativity: Associativity::None },
        );
        operators.insert(
            "le",
            OpInfo { precedence: Precedence(14), associativity: Associativity::None },
        );
        operators.insert(
            "ge",
            OpInfo { precedence: Precedence(14), associativity: Associativity::None },
        );

        // Level 15: Equality operators
        operators.insert(
            "==",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );
        operators.insert(
            "!=",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );
        operators.insert(
            "<=>",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );
        operators.insert(
            "eq",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );
        operators.insert(
            "ne",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );
        operators.insert(
            "cmp",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );
        operators.insert(
            "~~",
            OpInfo { precedence: Precedence(15), associativity: Associativity::None },
        );

        // Level 16: ISA operator
        operators.insert(
            "isa",
            OpInfo { precedence: Precedence(16), associativity: Associativity::None },
        );

        // Level 17: Bitwise and
        operators
            .insert("&", OpInfo { precedence: Precedence(17), associativity: Associativity::Left });
        operators.insert(
            "&.",
            OpInfo { precedence: Precedence(17), associativity: Associativity::Left },
        );

        // Level 18: Bitwise or/xor
        operators
            .insert("|", OpInfo { precedence: Precedence(18), associativity: Associativity::Left });
        operators
            .insert("^", OpInfo { precedence: Precedence(18), associativity: Associativity::Left });
        operators.insert(
            "|.",
            OpInfo { precedence: Precedence(18), associativity: Associativity::Left },
        );
        operators.insert(
            "^.",
            OpInfo { precedence: Precedence(18), associativity: Associativity::Left },
        );

        // Level 19: C-style logical and
        // Already added &&

        // Level 20: C-style logical or
        // Already added ||

        // Level 21: Range
        // Already added .. and ...

        // Level 22: Additive operators
        operators
            .insert("+", OpInfo { precedence: Precedence(22), associativity: Associativity::Left });
        operators
            .insert("-", OpInfo { precedence: Precedence(22), associativity: Associativity::Left });
        operators
            .insert(".", OpInfo { precedence: Precedence(22), associativity: Associativity::Left });

        // Level 23: Multiplicative operators
        operators
            .insert("*", OpInfo { precedence: Precedence(23), associativity: Associativity::Left });
        operators
            .insert("/", OpInfo { precedence: Precedence(23), associativity: Associativity::Left });
        operators
            .insert("%", OpInfo { precedence: Precedence(23), associativity: Associativity::Left });
        operators
            .insert("x", OpInfo { precedence: Precedence(23), associativity: Associativity::Left });

        // Level 24: Shift operators
        operators.insert(
            "<<",
            OpInfo { precedence: Precedence(24), associativity: Associativity::Left },
        );
        operators.insert(
            ">>",
            OpInfo { precedence: Precedence(24), associativity: Associativity::Left },
        );

        // Level 25: Named unary operators and filetest operators
        // These are prefix operators

        // Level 26: Bitwise not
        operators.insert(
            "~",
            OpInfo { precedence: Precedence(26), associativity: Associativity::Right },
        );
        operators.insert(
            "~.",
            OpInfo { precedence: Precedence(26), associativity: Associativity::Right },
        );

        // Level 27: Unary plus/minus and logical negation
        // These are prefix operators

        // Level 28: Exponentiation
        operators.insert(
            "**",
            OpInfo { precedence: Precedence(28), associativity: Associativity::Right },
        );

        // Level 29: Pattern match and substitution
        operators.insert(
            "=~",
            OpInfo { precedence: Precedence(29), associativity: Associativity::Left },
        );
        operators.insert(
            "!~",
            OpInfo { precedence: Precedence(29), associativity: Associativity::Left },
        );

        // Level 30: Dereference and postfix operators
        // These are handled specially

        PrattParser { operators }
    }

    pub fn get_operator_info(&self, op: &str) -> Option<&OpInfo> {
        self.operators.get(op)
    }

    pub fn is_prefix_operator(op: &str) -> bool {
        matches!(
            op,
            "!" | "not"
                | "~"
                | "~."
                | "+"
                | "-"
                | "++"
                | "--"
                | "\\"
                | "defined"
                | "undef"
                | "scalar"
                | "my"
                | "our"
                | "local"
                | "state"
        )
    }

    pub fn is_postfix_operator(op: &str) -> bool {
        matches!(op, "++" | "--")
    }

    pub fn parse_expression_from_pairs<'a>(
        &self,
        pairs: Vec<Pair<'a, Rule>>,
        parser: &mut crate::pure_rust_parser::PureRustPerlParser,
    ) -> Result<AstNode, Box<dyn std::error::Error>> {
        if pairs.is_empty() {
            return Err("Empty expression".into());
        }

        // Simple implementation for now - handle binary expressions
        if pairs.len() == 1 {
            // Single element, just parse it
            parser
                .build_node(pairs.into_iter().next().ok_or(crate::error::ParseError::ParseFailed)?)?
                .ok_or_else(|| "Failed to parse".into())
        } else if pairs.len() >= 3 {
            // Binary expression - use precedence parsing
            self.parse_binary_expr(pairs, 0, parser)
        } else {
            // Fallback
            parser
                .build_node(pairs.into_iter().next().ok_or(crate::error::ParseError::ParseFailed)?)?
                .ok_or_else(|| "Failed to parse".into())
        }
    }

    fn parse_binary_expr(
        &self,
        pairs: Vec<Pair<'_, Rule>>,
        index: usize,
        parser: &mut crate::pure_rust_parser::PureRustPerlParser,
    ) -> Result<AstNode, Box<dyn std::error::Error>> {
        if index >= pairs.len() {
            return Err("Invalid expression".into());
        }

        // Parse left operand
        let mut left =
            parser.build_node(pairs[index].clone())?.ok_or("Failed to parse left operand")?;

        let mut i = index + 1;
        while i + 1 < pairs.len() {
            // Get operator
            let op = pairs[i].as_str();

            // Get operator info
            if let Some(_op_info) = self.get_operator_info(op) {
                // Parse right operand
                let right = parser
                    .build_node(pairs[i + 1].clone())?
                    .ok_or("Failed to parse right operand")?;

                // Create binary op node
                left = AstNode::BinaryOp {
                    op: Arc::from(op),
                    left: Box::new(left),
                    right: Box::new(right),
                };

                i += 2;
            } else {
                break;
            }
        }

        Ok(left)
    }
}
