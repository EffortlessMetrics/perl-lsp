#![allow(clippy::unwrap_used, clippy::expect_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use perl_lsp_completion::CompletionProvider;
use perl_parser_core::Parser;
use perl_workspace::workspace_index::WorkspaceIndex;
use std::hint::black_box;
use std::sync::Arc;

const SIMPLE_SCRIPT: &str = "use strict;\nmy $count = 42;\nmy $name = \"hello\";\nmy @items = (1, 2, 3);\nsub process { my ($input) = @_; return $input * 2; }\nmy $result = process($count);\n";

const MODULE_WITH_OO: &str = "package MyApp::Handler;\nuse strict;\nour $VERSION = '1.00';\nsub new { my ($class, %args) = @_; bless { name => $args{name} // 'default', count => $args{count} // 0 }, $class }\nsub process_data { my ($self, $data) = @_; return 1; }\nsub transform { my ($self, $value) = @_; return $value * 2; }\nsub get_summary { my ($self) = @_; return { name => 'test' }; }\n1;\n";

const LARGE_MODULE: &str = "package MyApp::LargeModule;\nuse strict;\nsub func_a { 1 }\nsub func_b { 2 }\nsub func_c { 3 }\nsub func_d { 4 }\nsub func_e { 5 }\nsub func_f { 6 }\nsub func_g { 7 }\nsub func_h { 8 }\nsub func_i { 9 }\nsub func_j { 10 }\nsub helper_alpha { 'a' }\nsub helper_beta { 'b' }\nsub helper_gamma { 'c' }\nsub helper_delta { 'd' }\nsub helper_epsilon { 'e' }\n1;\n";

fn bench_variable_completion(c: &mut Criterion) {
    let source = "use strict;\nmy $count = 42;\nmy $config = {};\n$c";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new(&ast);
    let position = source.len();
    c.bench_function("completion_variable", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(source), black_box(position))))
    });
}

fn bench_keyword_completion(c: &mut Criterion) {
    let source = "use strict;\npri";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new(&ast);
    let position = source.len();
    c.bench_function("completion_keyword", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(source), black_box(position))))
    });
}

fn bench_module_completion(c: &mut Criterion) {
    let source_with_trigger = format!("{}$self->g", &MODULE_WITH_OO[..MODULE_WITH_OO.len() - 3]);
    let mut parser = Parser::new(&source_with_trigger);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new(&ast);
    let position = source_with_trigger.len();
    c.bench_function("completion_method_in_module", |b| {
        b.iter(|| {
            black_box(
                provider.get_completions(black_box(&source_with_trigger), black_box(position)),
            )
        })
    });
}

fn bench_workspace_completion(c: &mut Criterion) {
    let index = Arc::new(WorkspaceIndex::new());
    for (uri, content) in &[
        ("file:///lib/MyApp/Handler.pm", MODULE_WITH_OO),
        ("file:///lib/MyApp/LargeModule.pm", LARGE_MODULE),
        ("file:///lib/MyApp/Script.pm", SIMPLE_SCRIPT),
    ] {
        let url = url::Url::parse(uri).expect("valid url");
        index.index_file(url, content.to_string()).ok();
    }
    let source = "use MyApp::Handler;\nmy $h = MyApp::Handler->new();\n$h->";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let position = source.len();
    c.bench_function("completion_with_workspace", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(source), black_box(position))))
    });
}

fn bench_large_module_completion(c: &mut Criterion) {
    let source_with_trigger = format!("{}func_", LARGE_MODULE.trim_end_matches('\n'));
    let mut parser = Parser::new(&source_with_trigger);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new(&ast);
    let position = source_with_trigger.len();
    c.bench_function("completion_large_module", |b| {
        b.iter(|| {
            black_box(
                provider.get_completions(black_box(&source_with_trigger), black_box(position)),
            )
        })
    });
}

fn bench_empty_prefix_completion(c: &mut Criterion) {
    let source = format!("{}\n", MODULE_WITH_OO.trim_end_matches('\n'));
    let mut parser = Parser::new(&source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new(&ast);
    let position = source.len();
    c.bench_function("completion_empty_prefix", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(&source), black_box(position))))
    });
}

/// Generate a Perl module with ~10 symbols for workspace scale benchmarks.
fn gen_module_for_completion(idx: usize) -> (String, String) {
    let uri = format!("file:///lib/Scale/Module{}.pm", idx);
    let src = format!(
        "package Scale::Module{};\nuse strict;\nsub new {{ bless {{}}, shift }}\n\
         sub method_a_{0} {{ 1 }}\nsub method_b_{0} {{ 2 }}\nsub method_c_{0} {{ 3 }}\n\
         sub helper_{0} {{ 4 }}\nsub process_{0} {{ 5 }}\n\
         sub transform_{0} {{ 6 }}\nsub validate_{0} {{ 7 }}\nsub format_{0} {{ 8 }}\n1;\n",
        idx
    );
    (uri, src)
}

/// Build an in-memory workspace index with `n_files` generated modules.
///
/// Index construction happens outside Criterion's timing loop, so only the
/// completion call itself is measured.
fn build_scale_index(n_files: usize) -> Arc<WorkspaceIndex> {
    let index = Arc::new(WorkspaceIndex::new());
    let files: Vec<(url::Url, String)> = (0..n_files)
        .map(|i| {
            let (uri, src) = gen_module_for_completion(i);
            (url::Url::parse(&uri).expect("valid url"), src)
        })
        .collect();
    index.index_files_batch(files);
    index
}

fn bench_completion_with_100_file_workspace(c: &mut Criterion) {
    let index = build_scale_index(100);
    let source = "use Scale::Module0;\nmy $obj = Scale::Module0->new();\n$obj->";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let position = source.len();
    c.bench_function("completion_workspace_100_files", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(source), black_box(position))))
    });
}

fn bench_completion_with_500_file_workspace(c: &mut Criterion) {
    let index = build_scale_index(500);
    let source = "use Scale::Module0;\nmy $obj = Scale::Module0->new();\n$obj->";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let position = source.len();
    c.bench_function("completion_workspace_500_files", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(source), black_box(position))))
    });
}

fn bench_completion_with_1000_file_workspace(c: &mut Criterion) {
    let index = build_scale_index(1000);
    let source = "use Scale::Module0;\nmy $obj = Scale::Module0->new();\n$obj->";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let provider = CompletionProvider::new_with_index(&ast, Some(index));
    let position = source.len();
    c.bench_function("completion_workspace_1000_files", |b| {
        b.iter(|| black_box(provider.get_completions(black_box(source), black_box(position))))
    });
}

criterion_group!(
    benches,
    bench_variable_completion,
    bench_keyword_completion,
    bench_module_completion,
    bench_workspace_completion,
    bench_large_module_completion,
    bench_empty_prefix_completion,
    bench_completion_with_100_file_workspace,
    bench_completion_with_500_file_workspace,
    bench_completion_with_1000_file_workspace,
);
criterion_main!(benches);
