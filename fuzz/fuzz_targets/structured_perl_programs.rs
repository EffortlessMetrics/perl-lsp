#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_parser::{format_with_trivia, Parser, SymbolExtractor, TriviaPreservingParser};

const MAX_PROGRAM_CHARS: usize = 4096;
const MAX_STATEMENTS: usize = 32;
const MAX_FRAGMENT_CHARS: usize = 48;
const IDENT_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_";
const IDENT_TAIL_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_";
const STRING_CHARS: &[u8] =
    b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_ -+*/=()[]{}.,:;\n\t";

struct ByteCursor<'a> {
    data: &'a [u8],
    index: usize,
}

impl<'a> ByteCursor<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, index: 0 }
    }

    fn next_u8(&mut self) -> u8 {
        if self.data.is_empty() {
            return 0;
        }

        let byte = self.data[self.index % self.data.len()];
        self.index = self.index.saturating_add(1);
        byte
    }

    fn bounded(&mut self, max_exclusive: usize) -> usize {
        if max_exclusive == 0 {
            return 0;
        }

        usize::from(self.next_u8()) % max_exclusive
    }

    fn identifier(&mut self) -> String {
        let mut ident = String::new();
        ident.push(char::from(IDENT_CHARS[self.bounded(IDENT_CHARS.len())]));

        let tail_len = self.bounded(12);
        for _ in 0..tail_len {
            ident.push(char::from(IDENT_TAIL_CHARS[self.bounded(IDENT_TAIL_CHARS.len())]));
        }

        ident
    }

    fn package_name(&mut self) -> String {
        let parts = 1 + self.bounded(3);
        let mut package = self.identifier();
        for _ in 1..parts {
            package.push_str("::");
            package.push_str(&self.identifier());
        }
        package
    }

    fn string_fragment(&mut self) -> String {
        let len = self.bounded(MAX_FRAGMENT_CHARS);
        let mut fragment = String::new();
        for _ in 0..len {
            let byte = self.next_u8();
            let ch = match byte % 12 {
                0 => '\\',
                1 => '\'',
                2 => '"',
                3 => '$',
                4 => '@',
                5 => '%',
                6 => '🦀',
                7 => 'λ',
                _ => char::from(STRING_CHARS[usize::from(byte) % STRING_CHARS.len()]),
            };
            fragment.push(ch);
        }
        fragment
    }

    fn quoted_single(&mut self) -> String {
        let escaped = self.string_fragment().replace('\\', "\\\\").replace('\'', "\\'");
        format!("'{escaped}'")
    }

    fn quoted_double(&mut self) -> String {
        let escaped = self.string_fragment().replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    }

    fn scalar(&mut self) -> String {
        format!("${}", self.identifier())
    }

    fn expression(&mut self, depth: usize) -> String {
        if depth > 2 {
            return self.quoted_single();
        }

        match self.bounded(10) {
            0 => self.scalar(),
            1 => self.quoted_single(),
            2 => self.quoted_double(),
            3 => format!("q{{{}}}", self.string_fragment()),
            4 => format!("qw({})", self.string_fragment()),
            5 => format!("[{}, {}]", self.expression(depth + 1), self.expression(depth + 1)),
            6 => format!("{{ {} => {} }}", self.quoted_single(), self.expression(depth + 1)),
            7 => format!("{} + {}", self.expression(depth + 1), self.expression(depth + 1)),
            8 => format!("qr/{}/", self.string_fragment().replace('/', "\\/")),
            _ => self.bounded(10_000).to_string(),
        }
    }

    fn statement(&mut self, depth: usize) -> String {
        if depth > 2 {
            return format!("{};\n", self.expression(0));
        }

        match self.bounded(14) {
            0 => format!("package {};\n", self.package_name()),
            1 => format!("use {} {};\n", self.package_name(), self.expression(0)),
            2 => format!("no {} {};\n", self.package_name(), self.expression(0)),
            3 => format!("require {};\n", self.quoted_single()),
            4 => format!("my {} = {};\n", self.scalar(), self.expression(0)),
            5 => format!("our {} = {};\n", self.scalar(), self.expression(0)),
            6 => format!(
                "sub {} {{ my {} = {}; return {}; }}\n",
                self.identifier(),
                self.scalar(),
                self.expression(0),
                self.expression(0)
            ),
            7 => format!(
                "if ({}) {{ {} }} else {{ {} }}\n",
                self.expression(0),
                self.statement(depth + 1),
                self.statement(depth + 1)
            ),
            8 => format!(
                "for my {} ({}) {{ {} }}\n",
                self.scalar(),
                self.expression(0),
                self.statement(depth + 1)
            ),
            9 => format!("map {{ {} }} @{};\n", self.statement(depth + 1), self.identifier()),
            10 => format!("grep {{ {} }} @{};\n", self.statement(depth + 1), self.identifier()),
            11 => format!(
                "{} =~ s/{}/{}/g;\n",
                self.scalar(),
                self.string_fragment().replace('/', "\\/"),
                self.string_fragment().replace('/', "\\/")
            ),
            12 => format!("=head1 {}\n\n{}\n\n=cut\n", self.identifier(), self.string_fragment()),
            _ => {
                let marker = self.identifier();
                format!(
                    "my {} = <<'{}';\n{}\n{}\n",
                    self.scalar(),
                    marker,
                    self.string_fragment(),
                    marker
                )
            }
        }
    }
}

fn build_program(data: &[u8]) -> String {
    let mut cursor = ByteCursor::new(data);
    let statement_count = 1 + cursor.bounded(MAX_STATEMENTS);
    let mut program = String::from("use strict;\nuse warnings;\n");

    for _ in 0..statement_count {
        program.push_str(&cursor.statement(0));
        if program.chars().count() >= MAX_PROGRAM_CHARS {
            break;
        }
    }

    program.chars().take(MAX_PROGRAM_CHARS).collect()
}

fn exercise_parser(source: &str) {
    let mut parser = Parser::new(source);
    let result = parser.parse();

    let trivia_tree = TriviaPreservingParser::new(source.to_string()).parse();
    let _formatted = format_with_trivia(&trivia_tree);

    if let Ok(ast) = &result {
        let extractor = SymbolExtractor::new_with_source(source);
        let symbol_table = extractor.extract(ast);
        let _ = symbol_table.symbols.len();
        let _ = symbol_table.references.len();
    }
}

fuzz_target!(|data: &[u8]| {
    let program = build_program(data);
    exercise_parser(&program);

    let reversed: String = program.chars().rev().take(MAX_PROGRAM_CHARS).collect();
    exercise_parser(&reversed);
});
