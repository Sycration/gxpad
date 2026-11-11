//! Parser for user-typed expressions.

use super::*;

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(Rat, f64),
    Id(String),
    Op(char),
}

fn lex(s: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && cs.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let st = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            if i < cs.len() && (cs[i] == 'e' || cs[i] == 'E') {
                let mut j = i + 1;
                if j < cs.len() && (cs[j] == '-' || cs[j] == '+') {
                    j += 1;
                }
                if j < cs.len() && cs[j].is_ascii_digit() {
                    i = j;
                    while i < cs.len() && cs[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let t: String = cs[st..i].iter().collect();
            let f: f64 = t.parse().map_err(|_| format!("bad number {t}"))?;
            let r = parse_decimal(&t).unwrap_or(Rat { n: 0, d: 0 });
            out.push(Tok::Num(r, f));
        } else if c.is_alphabetic() || c == '_' {
            let st = i;
            while i < cs.len()
                && (cs[i].is_alphanumeric() || cs[i] == '_' || (cs[i] == '.' && cs.get(i + 1).is_some_and(|d| d.is_alphabetic())))
            {
                i += 1;
            }
            out.push(Tok::Id(cs[st..i].iter().collect()));
        } else {
            let op = match c {
                '−' | '–' => '-',
                '·' | '×' | '⋅' => '*',
                '÷' => '/',
                _ => c,
            };
            if !"+-*/^(),|√²³°=".contains(op) {
                return Err(format!("unexpected '{c}'"));
            }
            out.push(Tok::Op(op));
            i += 1;
        }
    }
    Ok(out)
}

fn parse_decimal(t: &str) -> Option<Rat> {
    if t.contains(['e', 'E']) {
        return None;
    }
    let (ip, fp) = match t.split_once('.') {
        Some((a, b)) => (a, b),
        None => (t, ""),
    };
    if fp.len() > 25 {
        return None;
    }
    let digits = format!("{ip}{fp}");
    let n: i128 = if digits.is_empty() { 0 } else { digits.parse().ok()? };
    Rat::new(n, 10i128.checked_pow(fp.len() as u32)?)
}

pub type Resolver<'a> = &'a dyn Fn(&str) -> Option<Expr>;

struct P<'a> {
    t: Vec<Tok>,
    i: usize,
    res: Resolver<'a>,
}

impl<'a> P<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.t.get(self.i)
    }
    fn is_op(&self, c: char) -> bool {
        self.peek() == Some(&Tok::Op(c))
    }
    fn eat(&mut self, c: char) -> bool {
        if self.is_op(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn sum(&mut self) -> Result<Expr, String> {
        let mut terms = vec![self.term()?];
        loop {
            if self.eat('+') {
                terms.push(self.term()?);
            } else if self.eat('-') {
                terms.push(neg(self.term()?));
            } else {
                break;
            }
        }
        Ok(add(terms))
    }
    fn starts_atom(&self) -> bool {
        match self.peek() {
            Some(Tok::Num(..)) | Some(Tok::Id(_)) => true,
            Some(Tok::Op(c)) => *c == '(' || *c == '√',
            None => false,
        }
    }
    fn term(&mut self) -> Result<Expr, String> {
        let mut f = vec![self.unary()?];
        loop {
            if self.eat('*') {
                f.push(self.unary()?);
            } else if self.eat('/') {
                f.push(pow(self.unary()?, int(-1)));
            } else if self.starts_atom() {
                f.push(self.power()?);
            } else {
                break;
            }
        }
        Ok(mul(f))
    }
    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat('-') {
            return Ok(neg(self.unary()?));
        }
        if self.eat('+') {
            return self.unary();
        }
        self.power()
    }
    fn power(&mut self) -> Result<Expr, String> {
        let b = self.postfix()?;
        if self.eat('^') {
            let e = self.unary()?;
            return Ok(pow(b, e));
        }
        Ok(b)
    }
    fn postfix(&mut self) -> Result<Expr, String> {
        let mut a = self.atom()?;
        loop {
            if self.eat('²') {
                a = pow(a, int(2));
            } else if self.eat('³') {
                a = pow(a, int(3));
            } else if self.eat('°') {
                a = mul(vec![a, pi(), rat(1, 180)]);
            } else {
                break;
            }
        }
        Ok(a)
    }
    fn args(&mut self) -> Result<Vec<Expr>, String> {
        if !self.eat('(') {
            return Err("expected (".into());
        }
        let mut v = vec![self.sum()?];
        while self.eat(',') {
            v.push(self.sum()?);
        }
        if !self.eat(')') {
            return Err("expected )".into());
        }
        Ok(v)
    }
    fn atom(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Tok::Num(r, f)) => {
                self.i += 1;
                Ok(if r.d != 0 { num(Num::R(r)) } else { float_raw(f) })
            }
            Some(Tok::Op('(')) => {
                self.i += 1;
                let e = self.sum()?;
                if !self.eat(')') {
                    return Err("expected )".into());
                }
                Ok(e)
            }
            Some(Tok::Op('|')) => {
                self.i += 1;
                let e = self.sum()?;
                if !self.eat('|') {
                    return Err("expected |".into());
                }
                Ok(fun(Fun::Abs, e))
            }
            Some(Tok::Op('√')) => {
                self.i += 1;
                Ok(sqrt(self.postfix()?))
            }
            Some(Tok::Id(name)) => {
                self.i += 1;
                let call = self.is_op('(');
                if call {
                    if let Some(f) = Fun::from_name(&name) {
                        let a = self.args()?;
                        if a.len() != 1 {
                            return Err(format!("{name} takes one argument"));
                        }
                        return Ok(fun(f, a[0].clone()));
                    }
                    match name.as_str() {
                        "sqrt" => {
                            let a = self.args()?;
                            return Ok(sqrt(a[0].clone()));
                        }
                        "cbrt" => {
                            let a = self.args()?;
                            return Ok(pow(a[0].clone(), rat(1, 3)));
                        }
                        "atan2" => {
                            let a = self.args()?;
                            if a.len() != 2 {
                                return Err("atan2 takes two arguments".into());
                            }
                            return Ok(atan2(a[0].clone(), a[1].clone()));
                        }
                        "hypot" => {
                            let a = self.args()?;
                            return Ok(sqrt(add(a.into_iter().map(sq).collect())));
                        }
                        _ => {}
                    }
                }
                if let Some(e) = (self.res)(&name) {
                    return Ok(e);
                }
                match name.as_str() {
                    "pi" | "π" => Ok(pi()),
                    "e" => Ok(fun(Fun::Exp, one())),
                    "deg" => Ok(mul(vec![pi(), rat(1, 180)])),
                    _ => Err(format!("unknown '{name}'")),
                }
            }
            Some(Tok::Op(c)) => Err(format!("unexpected '{c}'")),
            None => Err("unexpected end".into()),
        }
    }
}

/// Parses an expression; names are resolved through `res`.
pub fn parse(s: &str, res: Resolver) -> Result<Expr, String> {
    let t = lex(s)?;
    if t.is_empty() {
        return Err("empty".into());
    }
    let mut p = P { t, i: 0, res };
    let e = p.sum()?;
    if p.i != p.t.len() {
        return Err("unexpected input".into());
    }
    Ok(e)
}

/// Parses `lhs = rhs` into `lhs - rhs`, or a plain expression unchanged.
pub fn parse_equation(s: &str, res: Resolver) -> Result<(Option<Expr>, Expr), String> {
    match s.split_once('=') {
        Some((l, r)) => Ok((Some(parse(l, res)?), parse(r, res)?)),
        None => Ok((None, parse(s, res)?)),
    }
}

/// Returns true when the text is a plain number (allowing a degree sign).
pub fn plain_number(s: &str) -> Option<f64> {
    let t = s.trim().trim_end_matches('°').trim();
    let t = t.replace('−', "-");
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_basic() {
        let r = |n: &str| if n == "a" { Some(sym(0)) } else { None };
        let e = parse("2a^2 + 3·a - sqrt(4)", &r).unwrap();
        assert!((eval(&e, &|_| 2.0) - (8.0 + 6.0 - 2.0)).abs() < 1e-12);
        let e = parse("-a^2", &r).unwrap();
        assert!((eval(&e, &|_| 3.0) + 9.0).abs() < 1e-12);
        let e = parse("0.1 + 0.2", &r).unwrap();
        assert_eq!(e, rat(3, 10));
        assert!(parse("2 +", &r).is_err());
    }
}
