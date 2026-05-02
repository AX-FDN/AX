use super::*;

#[test]
fn accepts_trait_impl_methods() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Command { name: string }

impl Label for Command {
    fn label(self: Command) -> string {
        return self.name;
    }
}

fn main() -> i32 {
    let command: Command = Command { name: \"build\" };
    println(command.label());
    return string_len(command.label());
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_trait_impl_missing_method() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Command { name: string }

impl Label for Command {
}

fn main() -> i32 {
    let command: Command = Command { name: \"build\" };
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0059"));
}

#[test]
fn reports_trait_impl_signature_mismatch() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Command { name: string }

impl Label for Command {
    fn label(self: Command) -> i32 {
        return 1;
    }
}

fn main() -> i32 {
    return 0;
}
",
    );
    assert!(codes.iter().any(|code| code == "S0059"));
}

#[test]
fn accepts_generic_function_trait_bounds() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Command { name: string }

impl Label for Command {
    fn label(self: Command) -> string {
        return self.name;
    }
}

fn render<T: Label>(value: T) -> string {
    return value.label();
}

fn main() -> i32 {
    let command: Command = Command { name: \"build\" };
    println(render(command));
    return string_len(render(Command { name: \"check\" }));
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_generic_function_multiple_trait_bounds() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

trait Code {
    fn code(self: Self) -> i32;
}

struct Command { name: string, exit_code: i32 }

impl Label for Command {
    fn label(self: Command) -> string {
        return self.name;
    }
}

impl Code for Command {
    fn code(self: Command) -> i32 {
        return self.exit_code;
    }
}

fn render<T: Label + Code>(value: T) -> string {
    return value.label() + \":\" + to_string(value.code());
}

fn main() -> i32 {
    let command: Command = Command { name: \"build\", exit_code: 5 };
    println(render(command));
    return 0;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_where_trait_bounds_on_generic_functions() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

trait Code {
    fn code(self: Self) -> i32;
}

struct Command { name: string, exit_code: i32 }

impl Label for Command {
    fn label(self: Command) -> string {
        return self.name;
    }
}

impl Code for Command {
    fn code(self: Command) -> i32 {
        return self.exit_code;
    }
}

fn render<T>(value: T) -> string where T: Label + Code {
    return value.label() + \":\" + to_string(value.code());
}

fn main() -> i32 {
    let command: Command = Command { name: \"build\", exit_code: 5 };
    println(render(command));
    return 0;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}
