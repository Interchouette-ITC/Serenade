use super::*;

#[test]
fn bool_and_compare() {
    let mut ctx = ExpressionContext::new();
    ctx.insert("user.role", Value::string("admin"));
    ctx.insert("count", Value::Int(3));
    assert!(evaluate_bool(r#"user.role == "admin" && count > 0"#, &ctx).expect("ok"));
    assert!(!evaluate_bool(r#"user.role == "guest""#, &ctx).expect("ok"));
}

#[test]
fn arithmetic_and_parens() {
    let ctx = ExpressionContext::new();
    assert_eq!(evaluate("(1 + 2) * 3", &ctx).expect("ok"), Value::Int(9));
    assert_eq!(evaluate("-5 + 2", &ctx).expect("ok"), Value::Int(-3));
}

#[test]
fn logic_short_circuit_unknown_right() {
    let ctx = ExpressionContext::new();
    // Right side never evaluated when left is false for &&
    assert!(!evaluate_bool("false && missing.var", &ctx).expect("ok"));
    assert!(evaluate_bool("true || missing.var", &ctx).expect("ok"));
}

#[test]
fn unknown_variable_errors() {
    let ctx = ExpressionContext::new();
    let err = evaluate("nope", &ctx).expect_err("missing");
    assert!(matches!(err, ExpressionError::UnknownVariable { .. }));
}

#[test]
fn parse_errors() {
    let ctx = ExpressionContext::new();
    assert!(evaluate("1 +", &ctx).is_err());
    assert!(evaluate("\"unterminated", &ctx).is_err());
    assert!(evaluate("1 @ 2", &ctx).is_err());
}

#[test]
fn division_by_zero() {
    let ctx = ExpressionContext::new();
    assert!(matches!(
        evaluate("1 / 0", &ctx).expect_err("div0"),
        ExpressionError::DivisionByZero
    ));
}

#[test]
fn not_and_string_compare() {
    let mut ctx = ExpressionContext::new();
    ctx.insert("name", Value::string("a"));
    assert!(evaluate_bool(r#"!(name == "b")"#, &ctx).expect("ok"));
    assert!(evaluate_bool(r#"name < "b""#, &ctx).expect("ok"));
}

#[test]
fn truthy_coerce() {
    assert!(Value::Int(1).as_bool_truthy());
    assert!(!Value::Int(0).as_bool_truthy());
    assert!(!Value::string("").as_bool_truthy());
    assert!(Value::string("x").as_bool_truthy());
}

#[test]
fn version_non_empty() {
    assert_ne!(version(), "");
}

#[test]
fn ne_and_ge() {
    let ctx = ExpressionContext::new();
    assert!(evaluate_bool("1 != 2", &ctx).expect("ok"));
    assert!(evaluate_bool("2 >= 2", &ctx).expect("ok"));
    assert!(evaluate_bool("3 <= 3", &ctx).expect("ok"));
}

#[test]
fn more_parse_and_type_errors() {
    let ctx = ExpressionContext::new();
    assert!(evaluate("1 2", &ctx).is_err());
    assert!(evaluate("(1", &ctx).is_err());
    assert!(evaluate("a.", &ctx).is_err());
    assert!(evaluate("\"\\", &ctx).is_err());
    assert!(evaluate("\"\\x\"", &ctx).is_err());
    assert!(evaluate("-true", &ctx).is_err());
    assert!(evaluate(r#"1 < "a""#, &ctx).is_err());
    assert!(evaluate("1 + true", &ctx).is_err());
    assert!(evaluate(&format!("{} + 1", i64::MAX), &ctx).is_err());
    assert!(evaluate("999999999999999999999", &ctx).is_err());
}

#[test]
fn or_evaluates_right_when_left_false() {
    let ctx = ExpressionContext::new();
    assert!(evaluate_bool("false || true", &ctx).expect("ok"));
    assert!(!evaluate_bool("false || false", &ctx).expect("ok"));
}

#[test]
fn string_escape_nt() {
    let ctx = ExpressionContext::new();
    assert_eq!(
        evaluate("\"a\\nb\\tc\"", &ctx).expect("ok"),
        Value::string("a\nb\tc")
    );
}

#[test]
fn context_get_and_mul_sub_div() {
    let mut ctx = ExpressionContext::new();
    ctx.insert("n", Value::Int(10));
    assert_eq!(ctx.get("n"), Some(&Value::Int(10)));
    assert_eq!(evaluate("10 - 3", &ctx).expect("ok"), Value::Int(7));
    assert_eq!(evaluate("10 * 3", &ctx).expect("ok"), Value::Int(30));
    assert_eq!(evaluate("10 / 2", &ctx).expect("ok"), Value::Int(5));
}

#[test]
fn comparisons_cover_lt_gt() {
    let ctx = ExpressionContext::new();
    assert!(evaluate_bool("1 < 2", &ctx).expect("ok"));
    assert!(evaluate_bool("2 > 1", &ctx).expect("ok"));
    assert!(evaluate_bool(r#""b" > "a""#, &ctx).expect("ok"));
    assert!(evaluate_bool(r#""b" >= "a""#, &ctx).expect("ok"));
    assert!(evaluate_bool(r#""a" <= "a""#, &ctx).expect("ok"));
}

#[test]
fn string_escapes_quote_and_backslash() {
    let ctx = ExpressionContext::new();
    assert_eq!(
        evaluate("\"a\\\"b\\\\c\"", &ctx).expect("ok"),
        Value::string("a\"b\\c")
    );
}
