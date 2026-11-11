//! Formula derivation for a dimension or variable of a document, with the
//! result formatted as plain text and LaTeX. Shared by gxpad and the C and
//! JS interfaces.

use crate::derive;
use crate::expr::bx::{Bx, latex, plain, to_bx};
use crate::expr::{Expr, sym};
use crate::model::Doc;
use std::collections::HashMap;

/// First symbol number of the subexpression names `u_1`, `u_2`, … that
/// common subexpression elimination introduces.
pub const CSE_BASE: u32 = 0xF000_0000;

/// Display name of symbol `s`: the parameter name from `names`, `u_k` for a
/// subexpression, or `p<s>`.
pub fn sym_name(names: &HashMap<usize, String>, s: u32) -> String {
    if s >= CSE_BASE {
        return format!("u_{}", s - CSE_BASE + 1);
    }
    names.get(&(s as usize)).cloned().unwrap_or_else(|| format!("p{s}"))
}

/// Quantity whose formula is derived.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// Index into `Doc::dims`.
    Dim(usize),
    /// Index into `Doc::vars`.
    Var(usize),
}

/// Name, expression and angle flag of target `t`.
pub fn target_expr(doc: &Doc, t: Target) -> Result<(String, Expr, bool), String> {
    match t {
        Target::Dim(i) => {
            let d = doc.dims.get(i).and_then(|d| d.as_ref()).ok_or("gone")?;
            let e = if d.driving { doc.dim_value_expr(i, 0)? } else { doc.measure(i) };
            Ok((d.name.clone(), e, d.kind.is_angle()))
        }
        Target::Var(i) => {
            let v = doc.vars.get(i).and_then(|v| v.as_ref()).ok_or("gone")?;
            let e = if v.expr.trim().is_empty() { sym(v.param as u32) } else { doc.parse_expr(&v.expr, 0)? };
            Ok((v.name.clone(), e, false))
        }
    }
}

/// A derived formula.
#[derive(Clone, Debug)]
pub struct Formula {
    /// Name of the target.
    pub name: String,
    /// `name = formula` as plain text.
    pub plain: String,
    /// `name = formula` as LaTeX.
    pub latex: String,
    /// False when some part had to be solved numerically.
    pub exact: bool,
    /// True when the target is an angle (in radians).
    pub is_angle: bool,
    /// Formula evaluated at the current configuration.
    pub value: f64,
    /// Measured value of the target.
    pub actual: f64,
    /// The formula itself.
    pub expr: Expr,
}

/// Derives the formula of target `t`, searching `extra_ms` longer than the
/// default budget (gxpad's "search harder" adds 10 000 ms per level).
pub fn derive_formula(doc: &Doc, t: Target, extra_ms: u64) -> Result<Formula, String> {
    let (name, e, is_angle) = target_expr(doc, t)?;
    let d = derive::derive_harder(doc, &e, extra_ms)?;
    let names = doc.param_names();
    let full = to_bx(&d.expr, &|s| sym_name(&names, s));
    Ok(Formula {
        plain: format!("{} = {}", name, plain(&full)),
        latex: format!("{} = {}", latex(&Bx::Sym(name.clone())), latex(&full)),
        name,
        exact: d.exact,
        is_angle,
        value: d.value,
        actual: d.actual,
        expr: d.expr,
    })
}
