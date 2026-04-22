pub mod ai;
pub mod ast;
pub mod cli;
pub mod diagnostics;
pub mod frontend;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod source;
pub mod token;

pub use cli::run_cli;
