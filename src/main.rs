#[macro_use]
extern crate quote;
extern crate syn;

use std::{fs, io};
use std::path::Path;
use serde::{Deserialize, Serialize};
use quote::ToTokens;
use syn::{ImplItem, Item, ItemEnum, ItemFn, ItemImpl, ItemStruct};
use syn::spanned::Spanned;


#[derive(Debug, Clone, Serialize, Deserialize)]
struct TContext {
    module: Option<String>,
    file_path: Option<String>,
    file_name: Option<String>,
    struct_name: Option<String>,
    snippet: Option<String>,
    upper_lines: Option<String>,
    lower_lines: Option<String>,
}

impl TContext {
    fn add_snippet(&mut self, lines: &[&str], line_from: usize, line_to: usize) {
        let mut snippet = String::new();
        for line in &lines[line_from - 1..line_to] {
            snippet.push_str(line);
            snippet.push_str("\n");
        }
        self.snippet = Some(snippet);
    }
    fn add_rest_lines(&mut self, lines: &[&str], line_from: usize, line_to: usize) {
        let mut upper_lines = String::new();
        let mut lower_lines = String::new();
        for line in &lines[0..line_from - 1] {
            upper_lines.push_str(line);
            upper_lines.push_str("\n");
        }
        for line in &lines[line_to..lines.len()] {
            lower_lines.push_str(line);
            lower_lines.push_str("\n");
        }
        self.upper_lines = Some(upper_lines);
        self.lower_lines = Some(lower_lines);
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum CodeType {
    Function,
    Struct,
    Enum,
    Impl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TCode {
    name: String,
    signature: String,
    code_type: CodeType,
    docstring: Option<String>,
    line: usize,
    line_from: usize,
    line_to: usize,
    context: Option<TContext>,
}

fn parse_impl(item: &ItemImpl, context: TContext, lines: &[&str]) -> Vec<TCode> {
    let mut functions = Vec::new();
    for item in &item.items {
        match item {
            &ImplItem::Method(ref method) => {
                let signature = &method.sig;
                let docstring = method.attrs.iter()
                    .find(|attr| attr.path.is_ident("doc"))
                    .map(|attr| attr.tokens.to_string());

                let mut context = context.clone();
                let line = method.sig.ident.span().start().line;
                let line_from = method.span().start().line;
                let line_to = method.span().end().line;
                context.add_snippet(lines, line_from, line_to);
                context.add_rest_lines(lines, line_from, line_to);

                let function = TCode {
                    name: method.sig.ident.to_string(),
                    signature: quote!(#signature).to_string(),
                    code_type: CodeType::Function,
                    docstring,
                    line,
                    line_from,
                    line_to,
                    context: Some(context.clone()),
                };
                functions.push(function);
            }
            _ => {}
        }
    }
    functions
}

fn parse_enum(item: &ItemEnum, mut context: TContext, lines: &[&str]) -> TCode {
    let line = item.ident.span().start().line;
    let line_from = item.span().start().line;
    let line_to = item.span().end().line;
    context.add_snippet(lines, line_from, line_to);
    context.add_rest_lines(lines, line_from, line_to);

    let docstring = item.attrs.iter()
        .find(|attr| attr.path.is_ident("doc"))
        .map(|attr| attr.tokens.to_string());

    TCode {
        name: item.ident.to_string(),
        signature: quote!(#item).to_string(),
        code_type: CodeType::Enum,
        docstring,
        line,
        line_from,
        line_to,
        context: Some(context),
    }
}

fn parse_struct(item: &ItemStruct, mut context: TContext, lines: &[&str]) -> TCode {
    let line = item.ident.span().start().line;
    let line_from = item.span().start().line;
    let line_to = item.span().end().line;
    context.add_snippet(lines, line_from, line_to);
    context.add_rest_lines(lines, line_from, line_to);

    let docstring = item.attrs.iter()
        .find(|attr| attr.path.is_ident("doc"))
        .map(|attr| attr.tokens.to_string());

    TCode {
        name: item.ident.to_string(),
        signature: quote!(#item).to_string(),
        code_type: CodeType::Struct,
        docstring,
        line,
        line_from,
        line_to,
        context: Some(context),
    }
}

fn parse_fn(item: &ItemFn, mut context: TContext, lines: &[&str]) -> TCode {
    let signature = &item.sig;
    let docstring = item.attrs.iter()
        .find(|attr| attr.path.is_ident("doc"))
        .map(|attr| attr.tokens.to_string());
    let line = item.sig.ident.span().start().line;

    let line_from = item.span().start().line;
    let line_to = item.span().end().line;
    context.add_snippet(lines, line_from, line_to);
    context.add_rest_lines(lines, line_from, line_to);

    let function = TCode {
        name: item.sig.ident.to_string(),
        signature: quote!(#signature).to_string(),
        code_type: CodeType::Function,
        docstring,
        line,
        line_from: line,
        line_to: item.block.span().end().line,
        context: Some(context),
    };
    function
}

fn parse_item(item: &Item, context: TContext, lines: &[&str]) -> (Vec<TCode>, Vec<TCode>) {
    let mut structs = vec![];
    let mut functions = vec![];
    let mut context = context.clone();

    match item {
        Item::Impl(item) => {
            let impl_block_name = item.self_ty.to_token_stream().to_string();
            context.struct_name = Some(impl_block_name);
            functions.extend(parse_impl(item, context, lines));
        }
        Item::Enum(item) => {
            structs.push(parse_enum(item, context, lines));
        }
        Item::Struct(item) => {
            structs.push(parse_struct(item, context, lines));
        }
        Item::Fn(item) => {
            functions.push(parse_fn(item, context, lines));
        }
        _ => {}
    }

    (functions, structs)
}

// one possible implementation of walking a directory only visiting files
fn visit_rs_files(dir: &Path, cb: &mut dyn FnMut(&Path)) -> io::Result<()>
{
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().unwrap() != "target" {
                    visit_rs_files(&path, cb)?;
                }
            } else {
                let path = entry.path();
                if path.extension().unwrap_or_default() == "rs" {
                    cb(&path);
                }
            }
        }
    }
    Ok(())
}


fn main() {

    let mut functions = vec![];
    let mut structs = vec![];

    let first_cli_arg = std::env::args().nth(1).unwrap_or("../qdrant/lib/segment/src/".to_string());

    let dir_path = Path::new(&first_cli_arg);

    visit_rs_files(dir_path, &mut |path| {
        let relative_path = path.strip_prefix(dir_path).unwrap();

        let file_content = fs::read_to_string(path).unwrap();

        let lines = file_content.lines().collect::<Vec<&str>>();

        let syntax = syn::parse_file(&file_content).unwrap();

        for item in &syntax.items {
            let (mut f, mut s) = parse_item(item, TContext {
                module: Some(relative_path.parent().unwrap().file_name().unwrap_or_default().to_str().unwrap().to_string()),
                file_path: Some(relative_path.to_str().unwrap().to_string()),
                file_name: Some(path.file_name().unwrap().to_str().unwrap().to_string()),
                struct_name: None,
                snippet: None,
                upper_lines: None,
                lower_lines: None,
            }, &lines);
            functions.append(&mut f);
            structs.append(&mut s);
        }
    }).unwrap();


    for struct_ in structs {
        println!("{}", serde_json::to_string(&struct_).unwrap());
    }

    for function in functions {
        println!("{}", serde_json::to_string(&function).unwrap());
    }
}