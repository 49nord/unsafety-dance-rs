#![feature(rustc_private)]

// dependencies
extern crate ansi_term;

// built-in crates
extern crate getopts;
extern crate rustc;
extern crate rustc_driver;
extern crate syntax;

use std::{env, io};

use ansi_term::Colour::Yellow;
use getopts::Matches;
use rustc::session::Session;
use rustc_driver::driver::{CompileController, CompileState};
use rustc_driver::{Compilation, CompilerCalls, RustcDefaultCalls};
use syntax::{ast, codemap, visit};

fn dump_snippet<W: io::Write>(
    mut out: W,
    codemap: &codemap::CodeMap,
    span: codemap::Span,
) -> io::Result<()> {
    writeln!(out, "{}:", Yellow.paint(codemap.span_to_string(span)))?;
    writeln!(out, "{}\n", codemap.span_to_snippet(span).unwrap())?;
    Ok(())
}

struct UnsafeAnalyzer {
    // default_calls: RustcDefaultCalls,
}

impl UnsafeAnalyzer {
    fn new() -> UnsafeAnalyzer {
        UnsafeAnalyzer {
            // default_calls: RustcDefaultCalls,
        }
    }
}

struct UnsafeVisitor {
    unsafe_nodes: Vec<codemap::Span>,
}

impl UnsafeVisitor {
    fn new() -> UnsafeVisitor {
        UnsafeVisitor {
            unsafe_nodes: Vec::new(),
        }
    }
}

impl<'a> visit::Visitor<'a> for UnsafeVisitor {
    fn visit_block(&mut self, b: &ast::Block) {
        // FIXME: Do we need to check `CompilerGenerated` here?
        if let ast::BlockCheckMode::Unsafe(_) = b.rules {
            self.unsafe_nodes.push(b.span);
        }

        visit::walk_block(self, b);
    }

    fn visit_item(&mut self, i: &ast::Item) {
        if let ast::ItemKind::Fn(ref _decl, ref header, ref _generics, ref _block) = i.node {
            if header.unsafety == ast::Unsafety::Unsafe {
                self.unsafe_nodes.push(i.span);
            }
        }

        visit::walk_item(self, i)
    }

    fn visit_mac(&mut self, mac: &ast::Mac) {
        visit::walk_mac(self, mac);
    }
}

fn process_ast(state: &mut CompileState) {
    let krate = state.krate.as_ref().expect("missing crate");

    let mut visitor = UnsafeVisitor::new();
    visit::walk_crate(&mut visitor, krate);

    // We are done walking the crate, output the unsafe node's IDs.
    println!(
        "Found {} unsafe blocks or functions.\n",
        visitor.unsafe_nodes.len()
    );

    // FIXME: Still missing trait impls?

    let codemap = state.session.codemap();

    for span in visitor.unsafe_nodes {
        dump_snippet(std::io::stdout(), codemap, span).expect("could not print output");
    }
}

impl<'a> CompilerCalls<'a> for UnsafeAnalyzer {
    fn build_controller(self: Box<Self>, _: &Session, _: &Matches) -> CompileController<'a> {
        let mut control = CompileController::basic();
        control.after_parse.stop = Compilation::Stop;
        control.after_parse.callback = Box::new(process_ast);

        control
    }
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let (compile_result, session) =
        rustc_driver::run_compiler(&args, Box::new(UnsafeAnalyzer::new()), None, None);
}
