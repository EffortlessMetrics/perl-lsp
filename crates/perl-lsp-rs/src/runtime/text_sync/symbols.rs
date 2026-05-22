use super::*;

impl LspServer {
    pub(super) fn reindex_document_symbols(&self, uri: &str, ast: &Node, source: &str) {
        let extractor = crate::symbol::SymbolExtractor::new_with_source(source);
        let table = extractor.extract(ast);
        let symbols = table.symbols.keys().cloned().collect::<Vec<_>>();
        self.symbol_index.lock().replace_document_symbols(uri, symbols);
    }

    pub(crate) fn clear_document_symbols(&self, uri: &str) {
        self.symbol_index.lock().remove_document(uri);
    }
}
