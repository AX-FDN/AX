use super::*;

#[test]
fn reports_missing_module_declaration_in_module_mode_project() {
    let project_root = repo_root()
        .join("target")
        .join("semantic-module-missing-decl-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
    fs::write(
        project_root.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"semantic_module_missing_decl\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("report.ax"),
        "fn helper() -> i32 { return 1; }\n",
    )
    .expect("support file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "import lib.report;\nfn main() -> i32 { return 0; }\n",
    )
    .expect("entry file should exist");

    let diagnostics = project_diagnostics(&project_root);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0038")
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn reports_missing_import_for_cross_module_reference() {
    let project_root = repo_root()
        .join("target")
        .join("semantic-module-missing-import-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
    fs::write(
        project_root.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"semantic_module_missing_import\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("report.ax"),
        "module lib.report;\nfn helper() -> i32 { return 1; }\n",
    )
    .expect("support file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { return lib.report.helper(); }\n",
    )
    .expect("entry file should exist");

    let diagnostics = project_diagnostics(&project_root);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0043")
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0007")
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn reports_missing_import_for_cross_module_enum_constructor_without_function_noise() {
    let project_root = repo_root()
        .join("target")
        .join("semantic-module-missing-import-enum-constructor-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
    fs::write(
        project_root.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"semantic_module_missing_import_enum_constructor\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("result.ax"),
        "module lib.result;\nenum Result { Ok(i32) }\n",
    )
    .expect("support file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "fn main() -> i32 { lib.result.Result.Ok(1); return 0; }\n",
    )
    .expect("entry file should exist");

    let diagnostics = project_diagnostics(&project_root);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0043")
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0007")
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn reports_missing_import_for_cross_module_struct_literal_without_unknown_type_noise() {
    let project_root = repo_root()
        .join("target")
        .join("semantic-module-missing-import-struct-literal-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
    fs::write(
        project_root.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"semantic_module_missing_import_struct_literal\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("point.ax"),
        "module lib.point;\nstruct Point { value: i32 }\n",
    )
    .expect("support file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "\
fn main() -> i32 {
    let point: lib.point.Point = lib.point.Point { value: 1 };
    return point.value;
}
",
    )
    .expect("entry file should exist");

    let diagnostics = project_diagnostics(&project_root);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0043")
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0006")
    );

    let _ = fs::remove_dir_all(&project_root);
}

#[test]
fn reports_missing_import_for_cross_module_enum_value_without_undefined_variable_noise() {
    let project_root = repo_root()
        .join("target")
        .join("semantic-module-missing-import-enum-value-test");
    let _ = fs::remove_dir_all(&project_root);
    fs::create_dir_all(project_root.join("lib")).expect("lib directory should exist");
    fs::create_dir_all(project_root.join("src")).expect("src directory should exist");
    fs::write(
        project_root.join("AX.toml"),
        "\
manifest_version = 1

[package]
name = \"semantic_module_missing_import_enum_value\"
entry = \"src/main.ax\"
sources = [\"lib\"]
",
    )
    .expect("manifest should exist");
    fs::write(
        project_root.join("lib").join("result.ax"),
        "module lib.result;\nenum Result { Ok, Err }\n",
    )
    .expect("support file should exist");
    fs::write(
        project_root.join("src").join("main.ax"),
        "\
fn main() -> i32 {
    let result: lib.result.Result = lib.result.Result.Ok;
    return 0;
}
",
    )
    .expect("entry file should exist");

    let diagnostics = project_diagnostics(&project_root);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0043")
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0002")
    );
    assert!(
        !diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0006")
    );

    let _ = fs::remove_dir_all(&project_root);
}
