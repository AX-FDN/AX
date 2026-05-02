use super::format_source;
use crate::source::SourceFile;

#[test]
fn formats_current_prototype_syntax() {
    let source = SourceFile::anonymous(
        "struct Point{x:i32,y:i32} enum Flag{On,Off} fn main()->i32{let mut point:Point=Point{x:1,y:2};let values:[i32;3]=[1,2,3];if(point.x==1){println(\"ready\");}else if(point.x==2){println(values[1]);}else{println(\"other\");}for(let mut i:i32=0;i<2;i=i+1){if(i==1){continue;}point.x=point.x+i;if(i>2){break;}}return values[0];}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "struct Point {\n",
            "    x: i32,\n",
            "    y: i32,\n",
            "}\n",
            "\n",
            "enum Flag {\n",
            "    On,\n",
            "    Off,\n",
            "}\n",
            "\n",
            "fn main() -> i32 {\n",
            "    let mut point: Point = Point { x: 1, y: 2 };\n",
            "    let values: [i32; 3] = [1, 2, 3];\n",
            "    if (point.x == 1) {\n",
            "        println(\"ready\");\n",
            "    } else if (point.x == 2) {\n",
            "        println(values[1]);\n",
            "    } else {\n",
            "        println(\"other\");\n",
            "    }\n",
            "    for (let mut i: i32 = 0; i < 2; i = i + 1) {\n",
            "        if (i == 1) {\n",
            "            continue;\n",
            "        }\n",
            "        point.x = point.x + i;\n",
            "        if (i > 2) {\n",
            "            break;\n",
            "        }\n",
            "    }\n",
            "    return values[0];\n",
            "}\n"
        )
    );
}

#[test]
fn formats_const_items() {
    let source = SourceFile::anonymous("const EXIT_OK:i32=7;fn main()->i32{return EXIT_OK;}");
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        "const EXIT_OK: i32 = 7;\n\nfn main() -> i32 {\n    return EXIT_OK;\n}\n"
    );
}

#[test]
fn formats_public_items() {
    let source = SourceFile::anonymous("pub fn helper()->i32{return 1;}");
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(formatted, "pub fn helper() -> i32 {\n    return 1;\n}\n");
}

#[test]
fn formats_multiple_trait_bounds() {
    let source =
        SourceFile::anonymous("fn render<T:Label+Code>(value:T)->string{return value.label();}");
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        "fn render<T: Label + Code>(value: T) -> string {\n    return value.label();\n}\n"
    );
}

#[test]
fn formats_where_trait_bounds_into_canonical_generic_params() {
    let source = SourceFile::anonymous(
        "fn render<T>(value:T)->string where T:Label+Code{return value.label();}",
    );
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        "fn render<T: Label + Code>(value: T) -> string {\n    return value.label();\n}\n"
    );
}

#[test]
fn formats_type_alias_items() {
    let source = SourceFile::anonymous("type UserId=i32;fn main()->i32{return 0;}");
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        "type UserId = i32;\n\nfn main() -> i32 {\n    return 0;\n}\n"
    );
}

#[test]
fn formats_generic_impl_blocks() {
    let source = SourceFile::anonymous(
        "struct Box<T>{value:T}impl<T> Box<T>{fn get(self:Box<T>)->T{return self.value;}}",
    );
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "struct Box<T> {\n",
            "    value: T,\n",
            "}\n",
            "\n",
            "impl<T> Box<T> {\n",
            "    fn get(self: Box<T>) -> T {\n",
            "        return self.value;\n",
            "    }\n",
            "}\n"
        )
    );
}

#[test]
fn formats_generic_impl_methods() {
    let source = SourceFile::anonymous(
        "struct Pair<T,U>{left:T,right:U}impl<T> Pair<T,i32>{fn replace_right<U>(self:Pair<T,i32>,right:U)->Pair<T,U>{return Pair{left:self.left,right:right};}}",
    );
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        "struct Pair<T, U> {\n    left: T,\n    right: U,\n}\n\nimpl<T> Pair<T, i32> {\n    fn replace_right<U>(self: Pair<T, i32>, right: U) -> Pair<T, U> {\n        return Pair { left: self.left, right: right };\n    }\n}\n"
    );
}

#[test]
fn formats_generic_trait_impl_blocks() {
    let source = SourceFile::anonymous(
        "trait Label{fn label(self:Self)->string;}struct Box<T>{value:T}impl<T> Label for Box<T>{fn label(self:Box<T>)->string{return to_string(self.value);}}",
    );
    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "trait Label {\n",
            "    fn label(self: Self) -> string;\n",
            "}\n",
            "\n",
            "struct Box<T> {\n",
            "    value: T,\n",
            "}\n",
            "\n",
            "impl<T> Label for Box<T> {\n",
            "    fn label(self: Box<T>) -> string {\n",
            "        return to_string(self.value);\n",
            "    }\n",
            "}\n"
        )
    );
}

#[test]
fn formatting_is_idempotent_for_formatted_input() {
    let source = SourceFile::anonymous(
        "fn main() -> i32 {\n    let value: f32 = 3.0;\n    println(\"line\\nvalue\");\n    return 0;\n}\n",
    );

    let first = format_source(&source).expect("source should format");
    let second =
        format_source(&SourceFile::anonymous(first.clone())).expect("source should reformat");
    assert_eq!(first, second);
}

#[test]
fn formats_slice_types_and_expressions() {
    let source = SourceFile::anonymous(
        "fn take(window:[i32])->i32{let head:[i32]=window[0:2];return head[1];}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn take(window: [i32]) -> i32 {\n",
            "    let head: [i32] = window[0:2];\n",
            "    return head[1];\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_statements() {
    let source = SourceFile::anonymous(
        "enum Flag{On,Off} fn choose(flag:Flag)->i32{match(flag){Flag.On=>{return 1;} Flag.Off=>{return 0;}}}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "enum Flag {\n",
            "    On,\n",
            "    Off,\n",
            "}\n",
            "\n",
            "fn choose(flag: Flag) -> i32 {\n",
            "    match (flag) {\n",
            "        Flag.On => {\n",
            "            return 1;\n",
            "        }\n",
            "        Flag.Off => {\n",
            "            return 0;\n",
            "        }\n",
            "    }\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_expressions() {
    let source = SourceFile::anonymous(
        "fn main()->i32{let flag:bool=true;let value:i32=match(flag){true=>1,false=>0};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let flag: bool = true;\n",
            "    let value: i32 = match (flag) { true => 1, false => 0 };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_block_valued_match_expression_arms() {
    let source = SourceFile::anonymous(
        "fn main()->i32{let value:i32=match(true){true=>{let base:i32=40;base+2},false=>0};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let value: i32 = match (true) { true => { let base: i32 = 40; base + 2 }, false => 0 };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_binding_patterns() {
    let source = SourceFile::anonymous(
        "fn main()->i32{let value:i32=match(4){0=>1,other=>other};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let value: i32 = match (4) { 0 => 1, other => other };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_or_patterns() {
    let source =
        SourceFile::anonymous("fn main()->i32{let value:i32=match(1){0|1=>10,_=>0};return value;}");

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let value: i32 = match (1) { 0 | 1 => 10, _ => 0 };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_guards() {
    let source = SourceFile::anonymous(
        "fn main()->i32{let value:i32=match(2){2 if true=>10,_=>0};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let value: i32 = match (2) { 2 if true => 10, _ => 0 };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_range_patterns() {
    let source = SourceFile::anonymous(
        "fn main()->i32{let value:i32=match(404){400..=499=>4,_=>0};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let value: i32 = match (404) { 400..=499 => 4, _ => 0 };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_match_struct_patterns() {
    let source = SourceFile::anonymous(
        "struct Point{x:i32,y:i32} fn main()->i32{let point:Point=Point{x:1,y:2};let value:i32=match(point){Point{x,y}=>x+y};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "struct Point {\n",
            "    x: i32,\n",
            "    y: i32,\n",
            "}\n",
            "\n",
            "fn main() -> i32 {\n",
            "    let point: Point = Point { x: 1, y: 2 };\n",
            "    let value: i32 = match (point) { Point { x, y } => x + y };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_payload_enum_variants_and_patterns() {
    let source = SourceFile::anonymous(
        "enum Result{Ok(i32),Err(string),Empty} fn main()->i32{let result:Result=Result.Ok(7);let value:i32=match(result){Result.Ok(found)=>found,Result.Err(_)=>0,Result.Empty=>-1};return value;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "enum Result {\n",
            "    Ok(i32),\n",
            "    Err(string),\n",
            "    Empty,\n",
            "}\n",
            "\n",
            "fn main() -> i32 {\n",
            "    let result: Result = Result.Ok(7);\n",
            "    let value: i32 = match (result) { Result.Ok(found) => found, Result.Err(_) => 0, Result.Empty => -1 };\n",
            "    return value;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_result_error_propagation() {
    let source = SourceFile::anonymous(
        "fn load()->Result<i32,string>{let value:i32=parse()?;return Result.ok(value);}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn load() -> Result<i32, string> {\n",
            "    let value: i32 = parse()?;\n",
            "    return Result.ok(value);\n",
            "}\n"
        )
    );
}

#[test]
fn formats_impl_methods() {
    let source = SourceFile::anonymous(
        "struct Point{x:i32,y:i32} impl Point{fn sum(self:Point)->i32{return self.x+self.y;}}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "struct Point {\n",
            "    x: i32,\n",
            "    y: i32,\n",
            "}\n",
            "\n",
            "impl Point {\n",
            "    fn sum(self: Point) -> i32 {\n",
            "        return self.x + self.y;\n",
            "    }\n",
            "}\n",
        )
    );
}

#[test]
fn formats_logical_operator_precedence() {
    let source =
        SourceFile::anonymous("fn main()->i32{if(!(true||false)&&true){return 1;}return 0;}");

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    if (!(true || false) && true) {\n",
            "        return 1;\n",
            "    }\n",
            "    return 0;\n",
            "}\n"
        )
    );
}

#[test]
fn formats_modulo_operator_precedence() {
    let source = SourceFile::anonymous("fn main()->i32{return 8%3*2+1;}");

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!("fn main() -> i32 {\n", "    return 8 % 3 * 2 + 1;\n", "}\n")
    );
}

#[test]
fn formats_for_in_statements() {
    let source = SourceFile::anonymous(
        "fn main()->i32{let values:[i32;3]=[1,2,3];for(let value:i32 in values){println(value);}return 0;}",
    );

    let formatted = format_source(&source).expect("source should format");
    assert_eq!(
        formatted,
        concat!(
            "fn main() -> i32 {\n",
            "    let values: [i32; 3] = [1, 2, 3];\n",
            "    for (let value: i32 in values) {\n",
            "        println(value);\n",
            "    }\n",
            "    return 0;\n",
            "}\n"
        )
    );
}

#[test]
fn formatter_reports_parse_errors() {
    let source = SourceFile::anonymous("fn main() -> i32 { let value: i32 = 1 return value; }");
    assert!(format_source(&source).is_err());
}
