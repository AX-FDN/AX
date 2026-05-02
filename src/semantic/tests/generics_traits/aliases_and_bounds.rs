use super::*;

#[test]
fn accepts_type_aliases_in_type_positions() {
    let codes = check(
        "\
type UserId = i32;
type Scores = [i32; 2];

fn first(scores: Scores) -> UserId {
    return scores[0];
}

fn main() -> i32 {
    let scores: Scores = [4, 5];
    let id: UserId = first(scores);
    return id;
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_generic_type_aliases_in_type_positions() {
    let codes = check(
        "\
type Boxed<T> = Box<T>;

struct Box<T> { value: T }

fn unwrap(boxed: Boxed<i32>) -> i32 {
    return boxed.value;
}

fn main() -> i32 {
    let boxed: Boxed<i32> = Box { value: 7 };
    return unwrap(boxed);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_generic_impl_methods() {
    let codes = check(
        "\
struct Pair<T, U> { left: T, right: U }

impl<T> Pair<T, i32> {
    fn replace_right<U>(self: Pair<T, i32>, right: U) -> Pair<T, U> {
        return Pair { left: self.left, right: right };
    }
}

fn main() -> i32 {
    let pair: Pair<string, i32> = Pair { left: \"ax\", right: 1 };
    let changed: Pair<string, string> = pair.replace_right(\"ok\");
    return string_len(changed.right);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn reports_generic_function_trait_bound_mismatch() {
    let codes = check(
        "\
trait Label {
    fn label(self: Self) -> string;
}

fn render<T: Label>(value: T) -> string {
    return value.label();
}

fn main() -> i32 {
    return string_len(render(1));
}
",
    );
    assert!(codes.iter().any(|code| code == "S0059"));
}

#[test]
fn reports_unknown_trait_in_generic_function_bound() {
    let codes = check(
        "\
fn render<T: MissingTrait>(value: T) -> T {
    return value;
}

fn main() -> i32 {
    return render(1);
}
",
    );
    assert!(codes.iter().any(|code| code == "S0059"));
}
