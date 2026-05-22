use super::config::FormatConfig;

/// Native formatter document tree.
///
/// This is the small, lossless-friendly formatting IR from the replacement
/// contract. It is deliberately independent of Perl syntax for now; later
/// parser-facing formatter passes should lower CST/AST fragments into this
/// tree and then render it deterministically.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatDoc {
    /// Literal text that may be laid out with surrounding IR.
    Text(String),
    /// One ordinary space.
    Space,
    /// A newline at the current indentation level.
    Line,
    /// A line break that becomes a space when its containing group fits.
    SoftLine,
    /// A newline that cannot be flattened.
    HardLine,
    /// A layout group that may render flat or broken.
    Group(Vec<FormatDoc>),
    /// A nested document rendered one indentation level deeper when broken.
    Indent(Vec<FormatDoc>),
    /// Render one branch when broken and another branch when flat.
    IfBreak {
        /// Document to render when the containing group breaks.
        broken: Box<FormatDoc>,
        /// Document to render when the containing group fits flat.
        flat: Box<FormatDoc>,
    },
    /// Literal source text that must be preserved byte-for-byte.
    LiteralPreserve(String),
}

impl FormatDoc {
    /// Create literal text.
    #[must_use]
    pub fn text(value: impl Into<String>) -> Self {
        Self::Text(value.into())
    }

    /// Create a layout group.
    #[must_use]
    pub fn group(parts: impl Into<Vec<FormatDoc>>) -> Self {
        Self::Group(parts.into())
    }

    /// Create an indented document.
    #[must_use]
    pub fn indent(parts: impl Into<Vec<FormatDoc>>) -> Self {
        Self::Indent(parts.into())
    }

    /// Create an if-break choice.
    #[must_use]
    pub fn if_break(broken: FormatDoc, flat: FormatDoc) -> Self {
        Self::IfBreak { broken: Box::new(broken), flat: Box::new(flat) }
    }

    /// Create a literal-preserve region.
    #[must_use]
    pub fn literal_preserve(value: impl Into<String>) -> Self {
        Self::LiteralPreserve(value.into())
    }

    /// Render this document using the native formatter configuration.
    #[must_use]
    pub fn render(&self, config: &FormatConfig) -> String {
        let mut renderer = DocRenderer::new(config);
        renderer.render_doc(self, 0, false, false);
        renderer.output
    }

    fn flat_width(&self) -> Option<usize> {
        match self {
            Self::Text(text) | Self::LiteralPreserve(text) => {
                (!text.contains('\n')).then_some(text.chars().count())
            }
            Self::Space | Self::SoftLine => Some(1),
            Self::Line | Self::HardLine => None,
            Self::Group(parts) | Self::Indent(parts) => {
                parts.iter().try_fold(0_usize, |sum, doc| doc.flat_width().map(|width| sum + width))
            }
            Self::IfBreak { flat, .. } => flat.flat_width(),
        }
    }
}

struct DocRenderer<'a> {
    config: &'a FormatConfig,
    output: String,
    column: usize,
}

impl<'a> DocRenderer<'a> {
    fn new(config: &'a FormatConfig) -> Self {
        Self { config, output: String::new(), column: 0 }
    }

    fn render_doc(&mut self, doc: &FormatDoc, indent_level: usize, flat: bool, broken: bool) {
        match doc {
            FormatDoc::Text(text) | FormatDoc::LiteralPreserve(text) => self.push_text(text),
            FormatDoc::Space => self.push_text(" "),
            FormatDoc::Line | FormatDoc::HardLine => self.push_line(indent_level),
            FormatDoc::SoftLine if flat => self.push_text(" "),
            FormatDoc::SoftLine => self.push_line(indent_level),
            FormatDoc::Group(parts) => {
                let fits = doc
                    .flat_width()
                    .is_some_and(|width| self.column + width <= self.config.line_width as usize);
                for part in parts {
                    self.render_doc(part, indent_level, fits, !fits);
                }
            }
            FormatDoc::Indent(parts) => {
                for part in parts {
                    self.render_doc(part, indent_level + 1, flat, broken);
                }
            }
            FormatDoc::IfBreak { broken: broken_doc, flat: flat_doc } => {
                let selected = if broken { broken_doc } else { flat_doc };
                self.render_doc(selected, indent_level, flat, broken);
            }
        }
    }

    fn push_text(&mut self, text: &str) {
        self.output.push_str(text);
        if let Some((_, tail)) = text.rsplit_once('\n') {
            self.column = tail.chars().count();
        } else {
            self.column += text.chars().count();
        }
    }

    fn push_line(&mut self, indent_level: usize) {
        self.output.push('\n');
        let indent = if self.config.use_tabs {
            "\t".repeat(indent_level)
        } else {
            " ".repeat(indent_level * self.config.indent_width as usize)
        };
        self.output.push_str(&indent);
        self.column = indent.chars().count();
    }
}
