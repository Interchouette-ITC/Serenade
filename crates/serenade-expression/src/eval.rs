//! Evaluate a parsed expression against a context.

use crate::context::ExpressionContext;
use crate::error::ExpressionError;
use crate::parse::{BinaryOp, Expr, UnaryOp, parse};
use crate::value::Value;

/// Parse `source` and evaluate it in `ctx`.
///
/// # Errors
///
/// Returns parse, unknown-variable, type, or division errors.
pub fn evaluate(source: &str, ctx: &ExpressionContext) -> Result<Value, ExpressionError> {
    let ast = parse(source)?;
    eval_expr(&ast, ctx)
}

/// Parse `source` and coerce the result to a boolean.
///
/// # Errors
///
/// Same as [`evaluate`].
pub fn evaluate_bool(source: &str, ctx: &ExpressionContext) -> Result<bool, ExpressionError> {
    Ok(evaluate(source, ctx)?.as_bool_truthy())
}

fn eval_expr(expr: &Expr, ctx: &ExpressionContext) -> Result<Value, ExpressionError> {
    match expr {
        Expr::Bool(v) => Ok(Value::Bool(*v)),
        Expr::Int(v) => Ok(Value::Int(*v)),
        Expr::Str(v) => Ok(Value::Str(v.clone())),
        Expr::Var(name) => ctx
            .get(name)
            .cloned()
            .ok_or_else(|| ExpressionError::UnknownVariable { name: name.clone() }),
        Expr::Unary { op, expr } => eval_unary(*op, eval_expr(expr, ctx)?),
        Expr::Binary { op, left, right } => match *op {
            BinaryOp::And | BinaryOp::Or => eval_logic(*op, left, right, ctx),
            BinaryOp::Eq => Ok(Value::Bool(eval_expr(left, ctx)? == eval_expr(right, ctx)?)),
            BinaryOp::Ne => Ok(Value::Bool(eval_expr(left, ctx)? != eval_expr(right, ctx)?)),
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let l = eval_expr(left, ctx)?;
                let r = eval_expr(right, ctx)?;
                cmp_ord(*op, &l, &r).map(Value::Bool)
            }
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div => {
                let l = eval_expr(left, ctx)?;
                let r = eval_expr(right, ctx)?;
                let kind = match *op {
                    BinaryOp::Add => ArithOp::Add,
                    BinaryOp::Sub => ArithOp::Sub,
                    BinaryOp::Mul => ArithOp::Mul,
                    _ => ArithOp::Div,
                };
                arith(kind, l, r)
            }
        },
    }
}

fn eval_logic(
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    ctx: &ExpressionContext,
) -> Result<Value, ExpressionError> {
    let l = eval_expr(left, ctx)?;
    if matches!(op, BinaryOp::And) {
        if !l.as_bool_truthy() {
            return Ok(Value::Bool(false));
        }
        return Ok(Value::Bool(eval_expr(right, ctx)?.as_bool_truthy()));
    }
    if l.as_bool_truthy() {
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(eval_expr(right, ctx)?.as_bool_truthy()))
}

fn eval_unary(op: UnaryOp, value: Value) -> Result<Value, ExpressionError> {
    match op {
        UnaryOp::Not => Ok(Value::Bool(!value.as_bool_truthy())),
        UnaryOp::Neg => match value {
            Value::Int(n) => Ok(Value::Int(-n)),
            other => Err(ExpressionError::Type {
                message: format!("unary '-' expects int, got {other:?}"),
            }),
        },
    }
}

fn cmp_ord(op: BinaryOp, left: &Value, right: &Value) -> Result<bool, ExpressionError> {
    let ord = match op {
        BinaryOp::Lt => OrdOp::Lt,
        BinaryOp::Le => OrdOp::Le,
        BinaryOp::Gt => OrdOp::Gt,
        BinaryOp::Ge => OrdOp::Ge,
        _ => {
            return Err(ExpressionError::Type {
                message: "internal: non-compare op in cmp_ord".to_owned(),
            });
        }
    };
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(int_ord(ord, *a, *b)),
        (Value::Str(a), Value::Str(b)) => Ok(str_ord(ord, a, b)),
        _ => Err(ExpressionError::Type {
            message: format!("cannot compare {left:?} with {right:?}"),
        }),
    }
}

#[derive(Clone, Copy)]
enum OrdOp {
    Lt,
    Le,
    Gt,
    Ge,
}

const fn int_ord(op: OrdOp, a: i64, b: i64) -> bool {
    match op {
        OrdOp::Lt => a < b,
        OrdOp::Le => a <= b,
        OrdOp::Gt => a > b,
        OrdOp::Ge => a >= b,
    }
}

fn str_ord(op: OrdOp, a: &str, b: &str) -> bool {
    match op {
        OrdOp::Lt => a < b,
        OrdOp::Le => a <= b,
        OrdOp::Gt => a > b,
        OrdOp::Ge => a >= b,
    }
}

#[derive(Clone, Copy)]
enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
}

fn arith(op: ArithOp, left: Value, right: Value) -> Result<Value, ExpressionError> {
    let (Value::Int(a), Value::Int(b)) = (left, right) else {
        return Err(ExpressionError::Type {
            message: "arithmetic requires int operands".to_owned(),
        });
    };
    let n = match op {
        ArithOp::Add => a.checked_add(b),
        ArithOp::Sub => a.checked_sub(b),
        ArithOp::Mul => a.checked_mul(b),
        ArithOp::Div => {
            if b == 0 {
                return Err(ExpressionError::DivisionByZero);
            }
            a.checked_div(b)
        }
    };
    n.map(Value::Int).ok_or_else(|| ExpressionError::Type {
        message: "integer overflow".to_owned(),
    })
}
