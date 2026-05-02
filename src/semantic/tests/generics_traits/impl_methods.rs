use super::*;

#[test]
fn accepts_impl_methods_and_method_calls() {
    let codes = check(
        "\
struct Point { x: i32, y: i32 }

impl Point {
    fn sum(self: Point) -> i32 {
        return self.x + self.y;
    }

    fn offset_sum(self: Point, delta: i32) -> i32 {
        return self.sum() + delta;
    }
}

fn main() -> i32 {
    let point: Point = Point { x: 4, y: 5 };
    return point.offset_sum(3);
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}

#[test]
fn accepts_static_impl_methods() {
    let codes = check(
        "\
struct Point { x: i32, y: i32 }

impl Point {
    fn with(x: i32, y: i32) -> Point {
        return Point { x: x, y: y };
    }

    fn sum(self: Point) -> i32 {
        return self.x + self.y;
    }
}

fn main() -> i32 {
    let point: Point = Point.with(4, 8);
    return point.sum();
}
",
    );
    assert!(codes.is_empty(), "unexpected diagnostics: {codes:?}");
}
