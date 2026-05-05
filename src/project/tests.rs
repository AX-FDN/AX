use super::{PROJECT_MANIFEST_FILE, resolve_input};
use std::fs;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn resolves_project_directory_to_main_entry() {
    let project_dir = repo_root().join("examples").join("project_hello");
    let resolved = resolve_input(&project_dir).expect("project directory should resolve");

    let project = resolved
        .project
        .as_ref()
        .expect("project metadata should be available");
    assert_eq!(project.target_name(), "project_hello");
    assert!(
        resolved
            .source
            .display_path()
            .replace('\\', "/")
            .ends_with("examples/project_hello/src/main.ax")
    );
}

#[test]
fn resolves_manifest_path_to_same_entry() {
    let manifest_path = repo_root()
        .join("examples")
        .join("project_hello")
        .join(PROJECT_MANIFEST_FILE);
    let resolved = resolve_input(&manifest_path).expect("manifest path should resolve");

    let project = resolved.project.expect("project metadata should exist");
    assert_eq!(project.target_name(), "project_hello");
    assert!(
        project
            .entry_path()
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("examples/project_hello/src/main.ax")
    );
}

#[test]
fn resolves_support_sources_before_entry_file() {
    let project_root = repo_root().join("target").join("project-resolve-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_supports\"
entry = \"src/main.ax\"
sources = [\"src/lib.ax\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("src").join("lib.ax"),
        "fn helper() -> i32 { return 1; }\n",
    )
    .expect("lib.ax should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return helper(); }\n",
    )
    .expect("main.ax should exist");

    let resolved =
        resolve_input(&project_root).expect("project with support sources should resolve");
    let project = resolved.project.expect("project metadata should exist");
    assert_eq!(project.source_paths().len(), 1);
    assert!(
        project.source_paths()[0]
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("target/project-resolve-test/src/lib.ax")
    );
    assert!(resolved.source.text().contains("fn helper() -> i32"));
    assert!(resolved.source.text().contains("fn main() -> i32"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn resolves_support_source_directories_in_sorted_recursive_path_order() {
    let project_root = repo_root().join("target").join("project-resolve-dir-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib").join("nested"))
        .expect("project lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_support_dir\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("z.ax"),
        "fn helper_z() -> i32 { return 30; }\n",
    )
    .expect("z.ax should exist");
    fs::write(
        project_root.join("lib").join("a.ax"),
        "fn helper_a() -> i32 { return 10; }\n",
    )
    .expect("a.ax should exist");
    fs::write(
        project_root.join("lib").join("nested").join("c.ax"),
        "fn helper_c() -> i32 { return 20; }\n",
    )
    .expect("c.ax should exist");
    fs::write(project_root.join("lib").join("notes.txt"), "ignore me\n")
        .expect("notes.txt should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return helper_a() + helper_c() + helper_z(); }\n",
    )
    .expect("main.ax should exist");

    let resolved =
        resolve_input(&project_root).expect("project with support directory should resolve");
    let project = resolved.project.expect("project metadata should exist");
    let source_paths = project
        .source_paths()
        .iter()
        .map(|path| path.display().to_string().replace('\\', "/"))
        .collect::<Vec<_>>();
    assert_eq!(source_paths.len(), 3);
    assert!(source_paths[0].ends_with("target/project-resolve-dir-test/lib/a.ax"));
    assert!(source_paths[1].ends_with("target/project-resolve-dir-test/lib/nested/c.ax"));
    assert!(source_paths[2].ends_with("target/project-resolve-dir-test/lib/z.ax"));
    assert!(resolved.source.text().contains("fn helper_a() -> i32"));
    assert!(resolved.source.text().contains("fn helper_c() -> i32"));
    assert!(resolved.source.text().contains("fn helper_z() -> i32"));
    assert!(resolved.source.text().contains("fn main() -> i32"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn resolves_support_sources_from_shared_sibling_directory() {
    let test_root = repo_root()
        .join("target")
        .join("project-shared-support-test");
    let project_root = test_root.join("project");
    let shared_root = test_root.join("shared");
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(&shared_root).expect("shared foundation directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_shared_supports\"
entry = \"src/main.ax\"
sources = [\"../shared\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        shared_root.join("foundation.ax"),
        "fn shared_helper() -> i32 { return 7; }\n",
    )
    .expect("shared foundation file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return shared_helper(); }\n",
    )
    .expect("main.ax should exist");

    let resolved = resolve_input(&project_root)
        .expect("project with shared sibling support sources should resolve");
    let project = resolved.project.expect("project metadata should exist");
    assert_eq!(project.source_paths().len(), 1);
    assert!(
        project.source_paths()[0]
            .display()
            .to_string()
            .replace('\\', "/")
            .ends_with("target/project-shared-support-test/shared/foundation.ax")
    );
    assert!(resolved.source.text().contains("fn shared_helper() -> i32"));
    assert!(resolved.source.text().contains("fn main() -> i32"));

    let _ = fs::remove_dir_all(&test_root);
}

#[test]
fn derives_expected_module_paths_for_support_sources() {
    let test_root = repo_root().join("target").join("project-module-path-test");
    let project_root = test_root.join("project");
    let shared_root = test_root.join("foundation");
    let _ = fs::remove_dir_all(&test_root);
    fs::create_dir_all(project_root.join("lib").join("audit"))
        .expect("project lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(&shared_root).expect("shared root should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_module_paths\"
entry = \"src/main.ax\"
sources = [\"../foundation\", \"lib/report.ax\", \"lib/audit\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        shared_root.join("search.ax"),
        "fn helper() -> i32 { return 1; }\n",
    )
    .expect("shared support file should exist");
    fs::write(
        project_root.join("lib").join("report.ax"),
        "fn build_report() -> i32 { return 2; }\n",
    )
    .expect("project report file should exist");
    fs::write(
        project_root.join("lib").join("audit").join("summary.ax"),
        "fn summarize() -> i32 { return 3; }\n",
    )
    .expect("audit summary file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("main.ax should exist");

    let resolved = resolve_input(&project_root).expect("project should resolve");
    let project = resolved.project.expect("project metadata should exist");

    assert_eq!(
        project.expected_module_path(&shared_root.join("search.ax")),
        Some("foundation.search")
    );
    assert_eq!(
        project.expected_module_path(&project_root.join("lib").join("report.ax")),
        Some("report")
    );
    assert_eq!(
        project.expected_module_path(&project_root.join("lib").join("audit").join("summary.ax")),
        Some("audit.summary")
    );

    let _ = fs::remove_dir_all(&test_root);
}

#[test]
fn rejects_duplicate_module_root_aliases() {
    let project_root = repo_root()
        .join("target")
        .join("project-duplicate-module-root-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(project_root.join("lib")).expect("project lib directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_duplicate_module_root\"
entry = \"src/main.ax\"
sources = [\"lib\", \"lib.ax\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("report.ax"),
        "fn report() -> i32 { return 1; }\n",
    )
    .expect("lib/report.ax should exist");
    fs::write(
        project_root.join("lib.ax"),
        "fn helper() -> i32 { return 2; }\n",
    )
    .expect("lib.ax should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("main.ax should exist");

    let error = resolve_input(&project_root).expect_err("duplicate module roots should fail");
    assert!(error.contains("reuses module root alias `lib`"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn resolves_local_path_dependency_sources_under_dependency_alias() {
    let project_root = repo_root()
        .join("target")
        .join("project-path-dependency-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(
        project_root
            .join("packages")
            .join("config_rules")
            .join("src"),
    )
    .expect("dependency src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_path_dependency\"
entry = \"src/main.ax\"

[dependencies]
config_rules = { path = \"packages/config_rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root
            .join("packages")
            .join("config_rules")
            .join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"config_rules\"
sources = [\"src\"]
",
    )
    .expect("dependency manifest should exist");
    let dependency_source = project_root
        .join("packages")
        .join("config_rules")
        .join("src")
        .join("validate.ax");
    fs::write(
        &dependency_source,
        "module config_rules.validate;\nfn require_field() -> i32 { return 1; }\n",
    )
    .expect("dependency source should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "import config_rules.validate;\nfn main() -> i32 { return config_rules.validate.require_field(); }\n",
    )
    .expect("project entry should exist");

    let resolved =
        resolve_input(&project_root).expect("project with path dependency should resolve");
    let project = resolved.project.expect("project metadata should exist");
    assert_eq!(project.source_paths().len(), 1);
    assert_eq!(project.local_path_dependencies().len(), 1);
    assert_eq!(project.local_path_dependencies()[0].alias(), "config_rules");
    assert_eq!(project.local_path_dependencies()[0].source_paths().len(), 1);
    assert_eq!(
        project.expected_module_path(&dependency_source),
        Some("config_rules.validate")
    );
    assert!(
        resolved
            .source
            .text()
            .contains("module config_rules.validate")
    );
    assert!(resolved.source.text().contains("fn main() -> i32"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn rejects_dependency_alias_that_conflicts_with_support_source_root() {
    let project_root = repo_root()
        .join("target")
        .join("project-dependency-alias-conflict-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(project_root.join("lib")).expect("project lib directory should exist");
    fs::create_dir_all(project_root.join("packages").join("lib").join("src"))
        .expect("dependency src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_dependency_alias_conflict\"
entry = \"src/main.ax\"
sources = [\"lib\"]

[dependencies]
lib = { path = \"packages/lib\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root
            .join("packages")
            .join("lib")
            .join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"lib_package\"
sources = [\"src\"]
",
    )
    .expect("dependency manifest should exist");
    fs::write(
        project_root
            .join("packages")
            .join("lib")
            .join("src")
            .join("rules.ax"),
        "module lib.rules;\nfn value() -> i32 { return 1; }\n",
    )
    .expect("dependency source should exist");
    fs::write(
        project_root.join("lib").join("rules.ax"),
        "module lib.rules;\nfn local_value() -> i32 { return 2; }\n",
    )
    .expect("support source should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("project entry should exist");

    let error =
        resolve_input(&project_root).expect_err("alias conflict should fail project loading");
    assert!(error.contains("PX0005"));
    assert!(error.contains("reuses module root alias `lib`"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn rejects_invalid_dependency_alias_with_stable_package_code() {
    let project_root = repo_root()
        .join("target")
        .join("project-invalid-dependency-alias-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(project_root.join("packages").join("bad-name"))
        .expect("dependency directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_invalid_dependency_alias\"
entry = \"src/main.ax\"

[dependencies]
\"bad-name\" = { path = \"packages/bad-name\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("project entry should exist");

    let error = resolve_input(&project_root).expect_err("invalid alias should fail");
    assert!(error.contains("PX0001"));
    assert!(error.contains("may only contain ASCII letters, digits, and `_`"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn rejects_uninstalled_registry_dependency_with_stable_package_code() {
    let project_root = repo_root()
        .join("target")
        .join("project-registry-dependency-preview-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_registry_dependency_preview\"
entry = \"src/main.ax\"

[dependencies]
text_tools = { registry = \"ax\", version = \"0.1.0\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("project entry should exist");

    let error = resolve_input(&project_root).expect_err("uninstalled registry package should fail");
    assert!(error.contains("PX0112"));
    assert!(error.contains("registry dependency `text_tools`"));
    assert!(error.contains("AX.lock schema v2"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn rejects_locked_registry_dependency_without_cache_with_stable_package_code() {
    let project_root = repo_root()
        .join("target")
        .join("project-registry-dependency-cache-missing-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_registry_dependency_cache_missing\"
entry = \"src/main.ax\"

[dependencies]
text_tools = { registry = \"ax\", version = \"0.1.0\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root.join("AX.lock"),
        r#"{
  "schema_version": 2,
  "package": {
    "name": "project_registry_dependency_cache_missing"
  },
  "dependencies": [
    {
      "alias": "text_tools",
      "kind": "registry",
      "package": "text_tools_cache_missing",
      "version": "0.1.0",
      "source": {
        "registry": "ax",
        "url": "https://github.com/AX-FDN/AX-PKG.git",
        "rev": "0000000000000000000000000000000000000000",
        "path": "packages/text_tools"
      },
      "checksum": "sha256:preview-text-tools",
      "modules": [
        "text_tools.normalize"
      ]
    }
  ]
}
"#,
    )
    .expect("lockfile should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("project entry should exist");

    let error = resolve_input(&project_root).expect_err("missing registry cache should fail");
    assert!(error.contains("PX0116"));
    assert!(error.contains("locked but not cached"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn rejects_missing_dependency_manifest_with_stable_package_code() {
    let project_root = repo_root()
        .join("target")
        .join("project-missing-dependency-manifest-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(project_root.join("packages").join("config_rules"))
        .expect("dependency directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_missing_dependency_manifest\"
entry = \"src/main.ax\"

[dependencies]
config_rules = { path = \"packages/config_rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("project entry should exist");

    let error = resolve_input(&project_root).expect_err("missing manifest should fail");
    assert!(error.contains("PX0003"));
    assert!(error.contains("failed to read dependency `config_rules` manifest"));

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn rejects_transitive_path_dependency_with_stable_package_code() {
    let project_root = repo_root()
        .join("target")
        .join("project-transitive-dependency-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("src")).expect("project src directory should exist");
    fs::create_dir_all(
        project_root
            .join("packages")
            .join("config_rules")
            .join("src"),
    )
    .expect("dependency src directory should exist");
    fs::write(
        project_root.join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"project_transitive_dependency\"
entry = \"src/main.ax\"

[dependencies]
config_rules = { path = \"packages/config_rules\" }
",
    )
    .expect("project manifest should exist");
    fs::write(
        project_root
            .join("packages")
            .join("config_rules")
            .join(PROJECT_MANIFEST_FILE),
        "\
manifest_version = 1

[package]
name = \"config_rules\"
sources = [\"src\"]

[dependencies]
nested_rules = { path = \"../nested_rules\" }
",
    )
    .expect("dependency manifest should exist");
    fs::write(
        project_root
            .join("packages")
            .join("config_rules")
            .join("src")
            .join("validate.ax"),
        "module config_rules.validate;\nfn value() -> i32 { return 1; }\n",
    )
    .expect("dependency source should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return 0; }\n",
    )
    .expect("project entry should exist");

    let error = resolve_input(&project_root).expect_err("transitive dependency should fail");
    assert!(error.contains("PX0006"));
    assert!(error.contains("transitive path packages are not supported in v0"));

    let _ = fs::remove_dir_all(&project_root);
}
