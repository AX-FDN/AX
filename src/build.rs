const BUILD_MANIFEST_FILE: &str = "build-manifest.json";
const SOURCE_COPY_FILE: &str = "source.ax";
const PROJECT_SOURCES_DIR: &str = "project-sources";
const HIR_FILE: &str = "program.hir.json";
const MIR_FILE: &str = "program.mir.json";

mod aot_rules;
mod input;
mod model;
mod program;
mod readiness;

pub use input::{
    build_input_from_project, build_input_from_source, default_output_dir, target_name_from_file,
};
pub use model::*;
pub use program::build_program;
pub use readiness::assess_aot_readiness;

#[cfg(test)]
#[path = "build/tests.rs"]
mod tests;
