//! Conversion of expressions into a 2D math layout tree, plain text and LaTeX.

use super::*;

#[derive(Clone, Debug)]
pub enum Bx {
    Sym(String),
    Num(String),
    Op(String),
    Fn(String),
    Row(Vec<Bx>),
    Frac(Box<Bx>, Box<Bx>),
    Root(Box<Bx>, Option<u32>),
    Sup(Box<Bx>, Box<Bx>),
    Paren(Box<Bx>),
    Abs(Box<Bx>),
}

const P_ADD: u8 = 1;
const P_MUL: u8 = 2;
const P_POW: u8 = 3;

pub type Names<'a> = &'a dyn Fn(u32) -> String;

pub fn fmt_float(f: f64) -> String {
    if f == 0.0 {
        return "0".into();
    }
    let a = f.abs();
    if !(1e-5..1e9).contains(&a) {
        return format!("{f:.6e}");
    }
    let s = format!("{f:.10}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    // limit significant digits
    let digits = s.chars().filter(|c| c.is_ascii_digit()).count();
    if digits > 12 {
        let p = (12i32 - (a.log10().floor() as i32 + 1)).max(0) as usize;
        let s = format!("{f:.p$}");
        return s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    s.to_string()
}

fn wrap(b: Bx, need: bool) -> Bx {
    if need { Bx::Paren(Box::new(b)) } else { b }
}

fn row(v: Vec<Bx>) -> Bx {
    if v.len() == 1 { v.into_iter().next().unwrap() } else { Bx::Row(v) }
}

fn num_bx(n: Num) -> Bx {
    match n {
        Num::R(r) => {
            if r.d == 1 {
                Bx::Num(r.n.abs().to_string())
            } else {
                Bx::Frac(Box::new(Bx::Num(r.n.abs().to_string())), Box::new(Bx::Num(r.d.to_string())))
            }
        }
        Num::F(f) => Bx::Num(fmt_float(f.abs())),
    }
}

/// Builds the layout for `e` assuming its sign has already been handled.
pub fn to_bx(e: &Expr, names: Names) -> Bx {
    bx(e, 0, names)
}

fn bx(e: &Expr, prec: u8, names: Names) -> Bx {
    if e.is_negative_form() {
        let inner = bx_pos(&neg(e.clone()), P_MUL, names);
        return wrap(Bx::Row(vec![Bx::Op("−".into()), inner]), prec > P_ADD);
    }
    bx_pos(e, prec, names)
}

/// Approximate polynomial degree of a term, for display ordering.
fn degree(e: &Expr) -> f64 {
    match e.node() {
        Node::Num(_) => 0.0,
        Node::Pi => 0.0,
        Node::Sym(_) => 1.0,
        Node::Mul(fs) => fs.iter().map(degree).sum(),
        Node::Add(ts) => ts.iter().map(degree).fold(0.0, f64::max),
        Node::Pow(b, ex) => degree(b) * ex.as_num().map(|n| n.to_f64()).unwrap_or(1.0),
        _ => 1.0,
    }
}

fn first_sym(e: &Expr) -> u32 {
    let mut s: Vec<u32> = sym_set(e).into_iter().collect();
    s.sort();
    s.first().copied().unwrap_or(u32::MAX)
}

fn sorted_terms(ts: &[Expr]) -> Vec<Expr> {
    let key = |t: &Expr| {
        let d = degree(t);
        ((-d * 1000.0) as i64, first_sym(t))
    };
    let mut pos: Vec<Expr> = ts.iter().filter(|t| !t.is_negative_form()).cloned().collect();
    let mut negs: Vec<Expr> = ts.iter().filter(|t| t.is_negative_form()).cloned().collect();
    pos.sort_by_key(key);
    negs.sort_by_key(key);
    pos.extend(negs);
    pos
}

fn bx_pos(e: &Expr, prec: u8, names: Names) -> Bx {
    match e.node() {
        Node::Num(n) => {
            let b = num_bx(*n);
            if n.is_neg() {
                wrap(Bx::Row(vec![Bx::Op("−".into()), b]), prec > P_ADD)
            } else {
                b
            }
        }
        Node::Pi => Bx::Sym("π".into()),
        Node::Sym(s) => Bx::Sym(names(*s)),
        Node::Add(ts) => {
            if let Some((l, scaled)) = pull_denominator(e) {
                let f = Bx::Frac(Box::new(bx_pos(&scaled, 0, names)), Box::new(Bx::Num(l.to_string())));
                return f;
            }
            let mut v = Vec::new();
            for (i, t) in sorted_terms(ts).iter().enumerate() {
                if t.is_negative_form() {
                    v.push(Bx::Op("−".into()));
                    v.push(bx_pos(&neg(t.clone()), P_ADD + 1, names));
                } else {
                    if i > 0 {
                        v.push(Bx::Op("+".into()));
                    }
                    v.push(bx_pos(t, P_ADD + 1, names));
                }
            }
            wrap(Bx::Row(v), prec > P_ADD)
        }
        Node::Mul(fs) => {
            let mut numer: Vec<Expr> = Vec::new();
            let mut denom: Vec<Expr> = Vec::new();
            let mut coeff: Option<Num> = None;
            for f in fs {
                if let Some(n) = f.as_num() {
                    coeff = Some(n);
                    continue;
                }
                if let Node::Pow(b, ex) = f.node() {
                    if ex.is_negative_form() {
                        denom.push(pow(b.clone(), neg(ex.clone())));
                        continue;
                    }
                }
                numer.push(f.clone());
            }
            // a negative product with one sum factor reads better with the sum negated
            if coeff.is_some_and(|c| c.is_neg()) {
                if let Some(i) = numer.iter().position(|f| matches!(f.node(), Node::Add(_))) {
                    numer[i] = neg(numer[i].clone());
                    coeff = coeff.map(|c| c.neg());
                }
            }
            let mut extra: i128 = 1;
            for f in numer.iter_mut() {
                if let Some((l, scaled)) = pull_denominator(f) {
                    *f = scaled;
                    extra = extra.saturating_mul(l);
                }
            }
            let mut extra_up: i128 = 1;
            for f in denom.iter_mut() {
                if let Some((l, scaled)) = pull_denominator(f) {
                    *f = scaled;
                    extra_up = extra_up.saturating_mul(l);
                }
            }
            if extra > 1 || extra_up > 1 {
                let c = coeff.unwrap_or(Num::int(1));
                coeff = Some(c.mul(Num::R(Rat::new(extra_up, extra).unwrap_or(Rat::int(1)))));
            }
            let mut cn: Option<Bx> = None;
            let mut cd: Option<Bx> = None;
            let mut neg_sign = false;
            if let Some(c) = coeff {
                neg_sign = c.is_neg();
                match c.abs() {
                    Num::R(r) => {
                        if r.n != 1 {
                            cn = Some(Bx::Num(r.n.to_string()));
                        }
                        if r.d != 1 {
                            cd = Some(Bx::Num(r.d.to_string()));
                        }
                    }
                    Num::F(f) => cn = Some(Bx::Num(fmt_float(f))),
                }
            }
            let prod = |coef: Option<Bx>, fs: &[Expr], in_frac: bool| -> Option<Bx> {
                let mut v = Vec::new();
                let mut prev_num = false;
                if let Some(c) = coef {
                    v.push(c);
                    prev_num = true;
                }
                for (i, f) in fs.iter().enumerate() {
                    let p = if in_frac && fs.len() == 1 && coef_is_none(&v) { 0 } else { P_MUL + 1 };
                    let b = bx_pos(f, p, names);
                    if i > 0 || !v.is_empty() {
                        let juxt = prev_num && !starts_with_digit(&b);
                        if !juxt {
                            v.push(Bx::Op("·".into()));
                        }
                    }
                    prev_num = false;
                    v.push(b);
                }
                if v.is_empty() { None } else { Some(row(v)) }
            };
            let top = prod(cn, &numer, !denom.is_empty() || cd.is_some()).unwrap_or(Bx::Num("1".into()));
            let res = if denom.is_empty() && cd.is_none() {
                top
            } else {
                let bot = prod(cd, &denom, true).unwrap();
                Bx::Frac(Box::new(top), Box::new(bot))
            };
            if neg_sign {
                wrap(Bx::Row(vec![Bx::Op("−".into()), res]), prec > P_ADD)
            } else {
                // a bare product inside a power must be wrapped
                let is_frac = matches!(res, Bx::Frac(..));
                wrap(res, prec > P_MUL && !is_frac || prec > P_POW)
            }
        }
        Node::Pow(b, ex) => {
            if let Some(r) = ex.as_rat() {
                if r.n < 0 {
                    let inv = pow(b.clone(), num(Num::R(r.neg())));
                    return Bx::Frac(Box::new(Bx::Num("1".into())), Box::new(bx_pos(&inv, 0, names)));
                }
                if r.n == 1 && r.d > 1 {
                    return Bx::Root(Box::new(bx(b, 0, names)), if r.d == 2 { None } else { Some(r.d as u32) });
                }
            }
            let base = bx(b, P_POW + 1, names);
            let base = match (&base, b.node()) {
                (Bx::Frac(..), _) | (Bx::Root(..), _) => Bx::Paren(Box::new(base)),
                _ => base,
            };
            let expb = match ex.as_rat() {
                Some(r) if r.d != 1 => Bx::Row(vec![
                    Bx::Num(r.n.to_string()),
                    Bx::Op("/".into()),
                    Bx::Num(r.d.to_string()),
                ]),
                _ => bx(ex, 0, names),
            };
            Bx::Sup(Box::new(base), Box::new(expb))
        }
        Node::Fun(Fun::Abs, a) => Bx::Abs(Box::new(bx(a, 0, names))),
        Node::Fun(f, a) => Bx::Row(vec![Bx::Fn(f.name().into()), Bx::Paren(Box::new(bx(a, 0, names)))]),
        Node::Atan2(y, x) => Bx::Row(vec![
            Bx::Fn("atan2".into()),
            Bx::Paren(Box::new(Bx::Row(vec![bx(y, 0, names), Bx::Op(",".into()), bx(x, 0, names)]))),
        ]),
    }
}

fn lcm(a: i128, b: i128) -> i128 {
    let (mut x, mut y) = (a, b);
    while y != 0 {
        let t = x % y;
        x = y;
        y = t;
    }
    (a / x.max(1)).saturating_mul(b)
}

/// For a sum whose rational coefficients share a denominator L > 1, returns
/// L and the sum scaled by L.
fn pull_denominator(e: &Expr) -> Option<(i128, Expr)> {
    let Node::Add(ts) = e.node() else { return None };
    let mut l: i128 = 1;
    for t in ts {
        match t.split_coeff().0 {
            // only when every term is fractional; a/2 + b reads better than (a + 2b)/2
            Num::R(r) if r.d > 1 => l = lcm(l, r.d),
            _ => return None,
        }
        if l > 1_000_000 {
            return None;
        }
    }
    if l == 1 {
        return None;
    }
    Some((l, add(ts.iter().map(|t| mul(vec![int(l as i64), t.clone()])).collect())))
}

fn coef_is_none(v: &[Bx]) -> bool {
    v.is_empty()
}

fn starts_with_digit(b: &Bx) -> bool {
    match b {
        Bx::Num(_) | Bx::Frac(..) => true,
        Bx::Row(v) => v.first().is_some_and(starts_with_digit),
        Bx::Sup(b, _) => starts_with_digit(b),
        _ => false,
    }
}

// ---------------------------------------------------------------- text forms

fn is_compound(b: &Bx) -> bool {
    matches!(b, Bx::Row(v) if v.len() > 1) || matches!(b, Bx::Frac(..))
}

/// Plain text that the parser accepts back.
pub fn plain(b: &Bx) -> String {
    match b {
        Bx::Sym(s) => s.clone(),
        Bx::Num(s) => s.clone(),
        Bx::Op(s) => match s.as_str() {
            "·" => "*".into(),
            "−" => " - ".into(),
            "+" => " + ".into(),
            "," => ", ".into(),
            o => o.into(),
        },
        Bx::Fn(s) => s.clone(),
        Bx::Row(v) => {
            let mut s: String = v.iter().map(plain).collect();
            if s.starts_with(" - ") {
                s = format!("-{}", &s[3..]);
            }
            s
        }
        Bx::Frac(a, c) => {
            let pa = if is_compound(a) { format!("({})", plain(a)) } else { plain(a) };
            let pc = if is_compound(c) || matches!(**c, Bx::Sup(..)) { format!("({})", plain(c)) } else { plain(c) };
            format!("{pa}/{pc}")
        }
        Bx::Root(a, None) => format!("sqrt({})", plain(a)),
        Bx::Root(a, Some(n)) => format!("({})^(1/{n})", plain(a)),
        Bx::Sup(a, e) => {
            let pe = if is_compound(e) { format!("({})", plain(e)) } else { plain(e) };
            format!("{}^{}", plain(a), pe)
        }
        Bx::Paren(a) => format!("({})", plain(a)),
        Bx::Abs(a) => format!("abs({})", plain(a)),
    }
}

fn latex_name(s: &str) -> String {
    let greek = [
        ("α", "\\alpha"), ("β", "\\beta"), ("γ", "\\gamma"), ("δ", "\\delta"), ("ε", "\\varepsilon"),
        ("θ", "\\theta"), ("λ", "\\lambda"), ("μ", "\\mu"), ("π", "\\pi"), ("φ", "\\varphi"),
        ("ψ", "\\psi"), ("ω", "\\omega"), ("σ", "\\sigma"), ("τ", "\\tau"), ("ρ", "\\rho"),
    ];
    let (base, sub) = split_sub(s);
    let mut b = base.to_string();
    for (g, l) in greek {
        b = b.replace(g, &format!("{l} "));
    }
    let b = b.trim().to_string();
    let b = if b.chars().count() > 1 && !b.starts_with('\\') { format!("\\mathrm{{{b}}}") } else { b };
    match sub {
        Some(s) => format!("{b}_{{{s}}}"),
        None => b,
    }
}

/// Splits a name into base and subscript: `x_A` → (`x`, `A`), `d12` → (`d`, `12`).
pub fn split_sub(s: &str) -> (&str, Option<&str>) {
    if let Some((a, b)) = s.split_once('_') {
        return (a, Some(b));
    }
    let i = s.char_indices().find(|(_, c)| c.is_ascii_digit()).map(|(i, _)| i);
    match i {
        Some(i) if i > 0 && s[i..].chars().all(|c| c.is_ascii_digit()) => (&s[..i], Some(&s[i..])),
        _ => (s, None),
    }
}

pub fn latex(b: &Bx) -> String {
    match b {
        Bx::Sym(s) => latex_name(s),
        Bx::Num(s) => s.clone(),
        Bx::Op(s) => match s.as_str() {
            "·" => " \\cdot ".into(),
            "−" => " - ".into(),
            "+" => " + ".into(),
            o => o.into(),
        },
        Bx::Fn(s) => match s.as_str() {
            "sin" | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "exp" | "ln" => format!("\\{s}"),
            "asin" => "\\arcsin".into(),
            "acos" => "\\arccos".into(),
            "atan" => "\\arctan".into(),
            o => format!("\\operatorname{{{o}}}"),
        },
        Bx::Row(v) => v.iter().map(latex).collect::<String>().trim().to_string(),
        Bx::Frac(a, c) => format!("\\frac{{{}}}{{{}}}", latex(a), latex(c)),
        Bx::Root(a, None) => format!("\\sqrt{{{}}}", latex(a)),
        Bx::Root(a, Some(n)) => format!("\\sqrt[{n}]{{{}}}", latex(a)),
        Bx::Sup(a, e) => format!("{{{}}}^{{{}}}", latex(a), latex(e)),
        Bx::Paren(a) => format!("\\left({}\\right)", latex(a)),
        Bx::Abs(a) => format!("\\left|{}\\right|", latex(a)),
    }
}

#[cfg(test)]
pub fn to_plain(e: &Expr, names: Names) -> String {
    plain(&to_bx(e, names)).trim().to_string()
}
