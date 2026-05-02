#[path = "semantic/checker.rs"]
mod checker;
#[path = "semantic/driver.rs"]
mod driver;
#[path = "semantic/helpers.rs"]
mod helpers;
#[path = "semantic/program_info.rs"]
mod program_info;
#[path = "semantic/return_analysis.rs"]
mod return_analysis;
#[path = "semantic/types.rs"]
mod types;

pub use driver::{check_program, check_program_with_project};

#[cfg(test)]
#[path = "semantic/tests.rs"]
mod tests;
