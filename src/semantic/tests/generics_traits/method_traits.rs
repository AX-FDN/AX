use super::*;

#[test]
fn accepts_generic_impl_methods_and_method_calls() {
    let codes = check(
        "\
struct Box<T> { value: T }

impl<T> Box<T> {
    fn get(self: Box<T>) -> T {
        return self.value;
    }
}

fn main() -> i32 {
    let number: Box<i32> = Box { value: 7 };
    return number.get();
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_generic_trait_impl_for_trait_bounds() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Box<T> { value: T }

impl<T> Label for Box<T> {
    fn label(self: Box<T>) -> string {
        return to_string(self.value);
    }
}

fn render<T: Label>(value: T) -> string {
    return value.label();
}

fn main() -> i32 {
    let number: Box<i32> = Box { value: 42 };
    println(render(number));
    return 0;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_unknown_method_call() {
    let diagnostics = diagnostics(
        "\
struct Point { x: i32 }

fn main() -> i32 {
    let point: Point = Point { x: 1 };
    return point.missing();
}
",
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0057")
    );
}

#[test]
fn reports_trait_impl_method_self_shape_error() {
    let diagnostics = diagnostics(
        "\
trait Label {
    fn label(self: Self) -> string;
}

struct Point { x: i32 }

impl Label for Point {
    fn label(value: Point) -> string {
        return to_string(value.x);
    }
}

fn main() -> i32 {
    let point: Point = Point { x: 1 };
    return string_len(point.label());
}
",
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "S0056")
    );
}
