//! Reading .sgp text into a document. Each statement makes the entities and
//! constraints it stands for, in the order of the file; the `where` lines
//! are read first, so every point starts where the file puts it. A point the
//! file gives no position starts where its construction puts it, or at a
//! spot of its own.

use super::{Error, RESERVED, VERSION, XAXIS, YAXIS, split_points};
use crate::build::foot;
use crate::expr::parse::plain_number;
use crate::model::*;
use crate::sketch::{contact_guess, crossing_near, sample_polys, tangent_contact_near, valid_ident};
use std::collections::{HashMap, HashSet};

/// Label offset of a dimension the file places no label for.
pub(super) const DEFAULT_LABEL: [f64; 2] = [0.0, 0.4];

pub fn read(text: &str) -> Result<Doc, Error> {
    let lines = parse(text, true)?;
    let mut r = R::new(Doc::new());
    r.run(&lines)?;
    r.finish()
}

/// What [`apply`] makes of statements applied to a document.
pub struct Applied {
    /// The document with what the statements make; None when they only
    /// move points.
    pub doc: Option<Doc>,
    /// Points that a `where` line places, which were there before: where
    /// they are to be dragged.
    pub moves: Vec<(usize, [f64; 2])>,
}

/// Applies statements to `doc` (no header needed): what they make is added
/// to a copy of it, numbered after what is there. The copy is not solved.
pub fn apply(doc: &Doc, text: &str) -> Result<Applied, Error> {
    let lines = parse(text, false)?;
    let mut r = R::new(doc.clone());
    r.run(&lines)?;
    let moves = std::mem::take(&mut r.moves);
    let changed = lines.iter().any(|(_, st, _)| !matches!(st, St::Header(_) | St::Section | St::Pos(..)));
    if !changed {
        return Ok(Applied { doc: None, moves });
    }
    // what is added must read: a file may keep an expression that names
    // something deleted, but nothing new can name what is not there
    let (dims, vars) = (doc.dims.len(), doc.vars.len());
    let d = r.finish()?;
    let exprs = d.dims[dims..].iter().flatten().map(|x| (&x.name, &x.expr)).chain(d.vars[vars..].iter().flatten().map(|v| (&v.name, &v.expr)));
    for (name, e) in exprs.filter(|(_, e)| !e.is_empty()) {
        d.parse_expr(e, 0).map_err(|m| Error { line: 0, msg: format!("{name} = {e}: {m}") })?;
    }
    Ok(Applied { doc: Some(d), moves })
}

/// The entity of `doc` named `s`: a point or curve by its name, or a line
/// by its points run together (`AB`).
pub fn lookup(doc: &Doc, s: &str) -> Result<usize, String> {
    let r = R::new(doc.clone());
    match r.names.get(s.trim()) {
        Some(&e) => Ok(e),
        None => r.line_by_points(s.trim()).map(|x| x.0),
    }
}

// ------------------------------------------------------------ syntax

/// One line of the file.
#[derive(Debug)]
enum St {
    Header(String),
    Section,
    Origin(String),
    /// `point A B C`, `point P on k`
    Points(Vec<String>, Option<String>),
    /// `segment AB`, `l = line A B`, `triangle ABC`, `polygon ABCD`
    Shape(Option<String>, String, Vec<String>),
    /// `X = f(args)`
    Make(String, String, Vec<String>),
    /// `s = 3`, `s ~ 3` (solved), `q = d1/d2`
    Var(String, String, bool),
    /// `d1: |AB| = 5`
    Dim(String, String, Option<String>),
    /// `X == f(args)`, `A B == directrix(e, F)`
    Same(Vec<String>, String, Vec<String>),
    /// `A == B`
    Coincident(String, String),
    /// `|AB| == r(k)`
    Equal(String, String),
    /// `a op b`, with `inside` and `near (x, y)` after `tan`
    Rel(String, String, String, bool, Option<[f64; 2]>),
    /// `horiz A B`, `vert A B`
    HV(bool, String, String),
    Pos(String, [f64; 2]),
    Radius(String, f64),
    /// a, b, angle (radians)
    Conic(String, f64, Option<f64>, f64),
    Label(String, [f64; 2]),
    /// `NAME [flags]`
    Flags(String),
}

/// Functions of `X = f(...)` that make a point.
const POINT_FNS: &[&str] = &["mid", "meet", "reflect", "tangent", "focus"];
/// Functions of `x = f(...)` that make a curve.
const CURVE_FNS: &[&str] = &["circle", "arc", "ellipse", "hyperbola", "parabola", "curve", "perp", "par", "bisect", "asymptote", "directrix"];
/// Functions of `X == f(...)`.
const SAME_FNS: &[&str] = &["mid", "reflect", "tangent", "focus", "offset", "bisect", "asymptote", "directrix"];
const REL_OPS: &[&str] = &["on", "tan", "conc", "coll", "⟂", "perp", "∥", "par"];

/// The statements of `text`, which starts with the header line when `header`.
fn parse(text: &str, header: bool) -> Result<Vec<(usize, St, Vec<String>)>, Error> {
    let mut out = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let n = i + 1;
        let line = raw.split('#').next().unwrap().trim();
        if line.is_empty() {
            continue;
        }
        if out.is_empty() && line.starts_with('{') {
            return Err(Error { line: n, msg: "this is a sgpad document of the old JSON format, which this version does not read".into() });
        }
        let (body, flags) = strip_flags(line);
        let st = statement(body).map_err(|msg| Error { line: n, msg })?;
        if header && out.is_empty() && !matches!(st, St::Header(_)) {
            return Err(Error { line: n, msg: format!("not a sgp document: it must start with `sgp {VERSION}`") });
        }
        if let St::Header(v) = &st {
            if v.trim() != VERSION.to_string() {
                return Err(Error { line: n, msg: format!("sgp version {} is not readable here (this reads version {VERSION})", v.trim()) });
            }
        }
        out.push((n, st, flags));
    }
    if header && out.is_empty() {
        return Err(Error { line: 0, msg: "empty document".into() });
    }
    Ok(out)
}

/// Splits `[aux, hidden]` off the end of a line.
fn strip_flags(line: &str) -> (&str, Vec<String>) {
    if line.ends_with(']') {
        if let Some(i) = line.rfind('[') {
            let inner = &line[i + 1..line.len() - 1];
            let words: Vec<String> = inner.split([',', ' ']).filter(|w| !w.is_empty()).map(String::from).collect();
            if words.iter().all(|w| w.chars().all(|c| c.is_alphabetic())) {
                return (line[..i].trim(), words);
            }
        }
    }
    (line, vec![])
}

/// The identifier `s` starts with, and the rest after it.
fn ident(s: &str) -> Option<(&str, &str)> {
    let end = s.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(s.len());
    let id = &s[..end];
    if id.chars().next().is_some_and(|c| c.is_alphabetic()) { Some((id, &s[end..])) } else { None }
}

/// `f(args)` filling all of `s`: the function and the text between the brackets.
fn call(s: &str) -> Option<(&str, &str)> {
    let (f, rest) = ident(s.trim())?;
    let rest = rest.trim_start();
    let inner = rest.strip_prefix('(')?;
    let close = matching(rest)?;
    if close + 1 != rest.len() {
        return None;
    }
    Some((f, &inner[..close - 1]))
}

/// Index of the bracket closing the one `s` starts with.
fn matching(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Splits at commas outside brackets.
fn split_args(s: &str) -> Vec<String> {
    let mut out = vec![];
    let (mut depth, mut cur) = (0i32, String::new());
    for c in s.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
        if c == ',' && depth == 0 {
            out.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(c);
        }
    }
    if !cur.trim().is_empty() || !out.is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

fn number(s: &str) -> Result<f64, String> {
    s.trim().replace('−', "-").parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("{:?} is not a number", s.trim()))
}

fn degrees(s: &str) -> Result<f64, String> {
    Ok(number(s.trim().trim_end_matches('°'))?.to_radians())
}

/// `(x, y)`.
fn coords(s: &str) -> Result<[f64; 2], String> {
    let s = s.trim();
    let inner = s.strip_prefix('(').and_then(|s| s.strip_suffix(')')).ok_or_else(|| format!("expected (x, y), found {s:?}"))?;
    match split_args(inner).as_slice() {
        [x, y] => Ok([number(x)?, number(y)?]),
        _ => Err(format!("expected (x, y), found {s:?}")),
    }
}

/// Point names written apart (`A B`) or run together (`AB`).
fn point_list(s: &str) -> Result<Vec<String>, String> {
    let words: Vec<&str> = s.split_whitespace().collect();
    if words.len() == 1 {
        if let Some(ps) = split_points(words[0]).filter(|p| p.len() > 1) {
            return Ok(ps.into_iter().map(String::from).collect());
        }
    }
    Ok(words.into_iter().map(String::from).collect())
}

/// Splits `s` at the first top-level occurrence of `op` (outside brackets
/// and bars).
fn split_top<'s>(s: &'s str, op: &str) -> Option<(&'s str, &'s str)> {
    let (mut depth, mut bars) = (0i32, false);
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            '|' => bars = !bars,
            _ => {}
        }
        if depth == 0 && !bars && s[i..].starts_with(op) {
            return Some((&s[..i], &s[i + op.len()..]));
        }
    }
    None
}

fn statement(s: &str) -> Result<St, String> {
    let s = s.replace('⟂', " ⟂ ").replace('∥', " ∥ ");
    let s = s.trim();
    let first = ident(s).map(|(w, rest)| (w, rest.trim()));
    if let Some((w, rest)) = first.filter(|(_, rest)| !rest.starts_with(['=', ':', '~'])) {
        match w {
            "sgp" => return Ok(St::Header(rest.into())),
            "given" | "construct" | "find" | "where" if rest.is_empty() => return Ok(St::Section),
            "origin" => return Ok(St::Origin(rest.into())),
            "point" => {
                let words: Vec<&str> = rest.split_whitespace().collect();
                return match words.iter().position(|w| *w == "on") {
                    Some(i) if i + 2 == words.len() => Ok(St::Points(words[..i].iter().map(|w| w.to_string()).collect(), Some(words[i + 1].into()))),
                    Some(_) => Err("expected `point P on k`".into()),
                    None => Ok(St::Points(words.iter().map(|w| w.to_string()).collect(), None)),
                };
            }
            "segment" | "line" | "triangle" | "polygon" => return Ok(St::Shape(None, w.into(), point_list(rest)?)),
            "horiz" | "vert" => {
                return match point_list(rest)?.as_slice() {
                    [a, b] => Ok(St::HV(w == "horiz", a.clone(), b.clone())),
                    _ => Err(format!("expected `{w} A B`")),
                };
            }
            _ => {}
        }
    }
    if let Some((w, rest)) = first {
        // `d1: measure [= value]`
        if let Some(m) = rest.strip_prefix(':') {
            return Ok(match split_top(m, "=") {
                Some((m, v)) => St::Dim(w.into(), m.trim().into(), Some(v.trim().into())),
                None => St::Dim(w.into(), m.trim().into(), None),
            });
        }
    }
    if let Some((l, r)) = split_top(s, "==") {
        let (l, r) = (l.trim(), r.trim());
        if l.starts_with('|') || l.starts_with("len(") || l.starts_with("r(") {
            return Ok(St::Equal(l.into(), r.into()));
        }
        if let Some((f, args)) = call(r).filter(|(f, _)| SAME_FNS.contains(f)) {
            return Ok(St::Same(l.split_whitespace().map(String::from).collect(), f.into(), split_args(args)));
        }
        return Ok(St::Coincident(l.into(), r.into()));
    }
    let Some((w, rest)) = first else { return Err(format!("cannot read {s:?}")) };
    if let Some(rhs) = rest.strip_prefix('=') {
        let rhs = rhs.trim();
        if let Some((word, pts)) = ident(rhs).filter(|(word, _)| *word == "segment" || *word == "line") {
            return Ok(St::Shape(Some(w.into()), word.into(), point_list(pts.trim())?));
        }
        if let Some((f, args)) = call(rhs).filter(|(f, _)| POINT_FNS.contains(f) || CURVE_FNS.contains(f)) {
            let args = if f == "curve" { vec![args.trim().to_string()] } else { split_args(args) };
            return Ok(St::Make(w.into(), f.into(), args));
        }
        return Ok(St::Var(w.into(), rhs.into(), false));
    }
    if let Some(rhs) = rest.strip_prefix('~') {
        return Ok(St::Var(w.into(), rhs.trim().into(), true));
    }
    if rest.starts_with('(') {
        return Ok(St::Pos(w.into(), coords(rest)?));
    }
    if rest.is_empty() {
        return Ok(St::Flags(w.into()));
    }
    let words: Vec<&str> = rest.split_whitespace().collect();
    match words[0] {
        "r" if words.len() == 2 => return Ok(St::Radius(w.into(), number(words[1])?)),
        "label" => return Ok(St::Label(w.into(), coords(rest["label".len()..].trim())?)),
        "a" => {
            let (mut a, mut b, mut t) = (None, None, None);
            for kv in words.chunks(2) {
                match kv {
                    ["a", v] => a = Some(number(v)?),
                    ["b", v] => b = Some(number(v)?),
                    ["angle", v] => t = Some(degrees(v)?),
                    _ => return Err(format!("cannot read the shape of {w}: expected `{w} a A b B angle T°`")),
                }
            }
            return Ok(St::Conic(w.into(), a.unwrap(), b, t.unwrap_or(0.0)));
        }
        _ => {}
    }
    if REL_OPS.contains(&words[0]) && words.len() >= 2 {
        let mut inside = false;
        let mut near = None;
        let mut tail = rest[words[0].len()..].trim();
        let b = tail.split_whitespace().next().unwrap().to_string();
        tail = tail[b.len()..].trim();
        while !tail.is_empty() {
            if let Some(t) = tail.strip_prefix("inside") {
                inside = true;
                tail = t.trim();
            } else if let Some(t) = tail.strip_prefix("near") {
                let t = t.trim();
                let close = matching(t).ok_or("expected near (x, y)")?;
                near = Some(coords(&t[..=close])?);
                tail = t[close + 1..].trim();
            } else {
                return Err(format!("cannot read {tail:?}"));
            }
        }
        return Ok(St::Rel(w.into(), words[0].into(), b, inside, near));
    }
    Err(format!("cannot read {s:?}"))
}

// ------------------------------------------------------------ making the document

struct R {
    d: Doc,
    /// Entities by the name the file gives them.
    names: HashMap<String, usize>,
    pos: HashMap<String, [f64; 2]>,
    /// Parameters `where` marks fixed, by entity name.
    fixed: HashMap<String, Vec<String>>,
    radius: HashMap<String, f64>,
    conic: HashMap<String, (f64, Option<f64>, f64)>,
    labels: HashMap<String, [f64; 2]>,
    /// Names the `where` lines refer to that the construction has used.
    placed: HashSet<String>,
    /// Points placed without a position so far.
    spread: usize,
    /// Tangencies written without `near`.
    tangents: Vec<usize>,
    /// Points already there that `where` lines place.
    moves: Vec<(usize, [f64; 2])>,
    /// Dimensions before this reading, which it leaves as they are.
    first_dim: usize,
}

impl R {
    /// A reader adding to `d`, which knows its named points and curves.
    fn new(d: Doc) -> R {
        let mut names = HashMap::new();
        for (e, x) in d.ents.iter().enumerate() {
            if let Some(x) = x.as_ref().filter(|x| !x.name.is_empty()) {
                names.insert(x.name.clone(), e);
            }
        }
        names.insert(XAXIS.to_string(), d.xaxis);
        names.insert(YAXIS.to_string(), d.yaxis);
        let spread = (0..d.ents.len()).filter(|&e| d.is_point(e) && !d.is_fixed_ent(e)).count();
        let first_dim = d.dims.len();
        R {
            d,
            names,
            pos: HashMap::new(),
            radius: HashMap::new(),
            conic: HashMap::new(),
            fixed: HashMap::new(),
            labels: HashMap::new(),
            placed: HashSet::new(),
            spread,
            tangents: vec![],
            moves: vec![],
            first_dim,
        }
    }

    fn run(&mut self, lines: &[(usize, St, Vec<String>)]) -> Result<(), Error> {
        for (n, st, flags) in lines {
            self.solution(st, flags).map_err(|msg| Error { line: *n, msg })?;
        }
        for (n, st, flags) in lines {
            self.lower(st, flags).map_err(|msg| Error { line: *n, msg })?;
        }
        Ok(())
    }

    /// Reads a `where` line.
    fn solution(&mut self, st: &St, flags: &[String]) -> Result<(), String> {
        let (name, vals): (&str, &[&str]) = match st {
            St::Pos(n, p) if self.names.contains_key(n) => {
                let e = self.point(n)?;
                if !flags.is_empty() {
                    return Err(format!("{n} is there already: only its position can change"));
                }
                self.moves.push((e, *p));
                return Ok(());
            }
            St::Radius(n, _) | St::Conic(n, ..) if self.names.contains_key(n) => return Err(format!("{n} is there already: only positions of points can change")),
            St::Pos(n, p) => {
                self.pos.insert(n.clone(), *p);
                (n, &["x", "y"])
            }
            St::Radius(n, r) => {
                self.radius.insert(n.clone(), *r);
                (n, &["r"])
            }
            St::Conic(n, a, b, t) => {
                self.conic.insert(n.clone(), (*a, *b, *t));
                (n, &["a", "b", "angle"])
            }
            St::Label(n, off) => {
                self.labels.insert(n.clone(), *off);
                (n, &[])
            }
            _ => return Ok(()),
        };
        let fixed: Vec<String> = match flags.split_first() {
            None => vec![],
            Some((f, [])) if f == "fixed" && !vals.is_empty() => vals.iter().map(|v| v.to_string()).collect(),
            Some((f, some)) if f == "fixed" && some.iter().all(|v| vals.contains(&v.as_str())) => some.to_vec(),
            _ => return Err(format!("unknown flags {flags:?} (here: [fixed] or [fixed {}])", vals.join(" "))),
        };
        self.fixed.insert(name.to_string(), fixed);
        Ok(())
    }

    /// Locks the parameters `where` marks fixed for entity e named n.
    fn lock(&mut self, n: &str, e: usize) {
        let Some(fixed) = self.fixed.get(n) else { return };
        let params: Vec<(&str, usize)> = match *self.d.kind(e).unwrap() {
            Ent::Point { x, y } => vec![("x", x), ("y", y)],
            Ent::Circle { r, .. } => vec![("r", r)],
            Ent::Conic { a, b, t, .. } => vec![("a", a), ("b", b), ("angle", t)],
            _ => vec![],
        };
        for (v, p) in params {
            if fixed.iter().any(|f| f == v) {
                self.d.params[p].locked = true;
            }
        }
    }

    fn lower(&mut self, st: &St, flags: &[String]) -> Result<(), String> {
        let ent_flags = |flags: &[String]| -> Result<(bool, bool), String> {
            let mut f = (false, false);
            for w in flags {
                match w.as_str() {
                    "aux" => f.0 = true,
                    "hidden" => f.1 = true,
                    _ => return Err(format!("unknown flag {w:?} (an entity takes [aux] and [hidden])")),
                }
            }
            Ok(f)
        };
        let numeric = |flags: &[String]| -> Result<bool, String> {
            match flags {
                [] => Ok(false),
                [f] if f == "numeric" => Ok(true),
                _ => Err(format!("unknown flags {flags:?} (a dimension or variable takes [numeric])")),
            }
        };
        match st {
            St::Header(_) | St::Section | St::Pos(..) | St::Radius(..) | St::Conic(..) | St::Label(..) => {}
            St::Origin(n) => {
                let o = self.d.origin;
                let old = self.d.ent(o).unwrap().name.clone();
                self.check_new_ent(n)?;
                self.names.remove(&old);
                self.names.insert(n.clone(), o);
                self.d.ents[o].as_mut().unwrap().name = n.clone();
            }
            St::Flags(n) => {
                let e = self.ent(n)?;
                self.set_flags(e, ent_flags(flags)?);
            }
            St::Points(ns, on) => {
                let f = ent_flags(flags)?;
                if ns.is_empty() {
                    return Err("`point` names no point".into());
                }
                let on = on.as_ref().map(|c| self.curve(c)).transpose()?;
                for n in ns {
                    let p = self.new_point(n, |r: &R| on.map(|c| r.nearest_on(c)))?;
                    self.set_flags(p, f);
                    if let Some(c) = on {
                        self.d.cons.push(Some(Con::OnCurve(p, c)));
                    }
                }
            }
            St::Shape(name, word, pts) => {
                let f = ent_flags(flags)?;
                let closed = word == "triangle" || word == "polygon";
                match (word.as_str(), pts.len()) {
                    ("segment" | "line", 2) => {}
                    ("triangle", 3) => {}
                    ("polygon", n) if n >= 3 => {}
                    _ => return Err(format!("a {word} with {} points", pts.len())),
                }
                if closed && name.is_some() {
                    return Err(format!("a {word} has no name of its own"));
                }
                let mut ps = Vec::new();
                for n in pts {
                    ps.push(match self.names.get(n) {
                        Some(&p) if self.d.is_point(p) => p,
                        Some(_) => return Err(format!("{n} is not a point")),
                        None => self.new_point(n, |_| None)?,
                    });
                }
                let n = ps.len();
                let sides = if closed { n } else { 1 };
                for i in 0..sides {
                    let (a, b) = (ps[i], ps[(i + 1) % n]);
                    if a == b {
                        return Err("a segment needs two different points".into());
                    }
                    let l = self.d.add_ent(Ent::Line { a, b, infinite: word == "line" });
                    self.set_flags(l, f);
                    if let Some(name) = name {
                        self.name(l, name)?;
                    }
                }
            }
            St::Make(n, f, args) => self.make(n, f, args, ent_flags(flags)?)?,
            St::Var(n, rhs, solved) => {
                self.check_new(n)?;
                let numeric = numeric(flags)?;
                let i = match (plain_number(rhs).filter(|_| !rhs.contains('°')), solved) {
                    (Some(v), solved) => {
                        let i = self.d.add_var(n.clone(), v);
                        let p = self.d.vars[i].as_ref().unwrap().param;
                        self.d.params[p].locked = !solved;
                        i
                    }
                    (None, true) => return Err(format!("a solved variable starts at a number, not {rhs:?}")),
                    (None, false) => {
                        let i = self.d.add_var(n.clone(), 0.0);
                        self.d.vars[i].as_mut().unwrap().expr = rhs.clone();
                        i
                    }
                };
                self.d.vars[i].as_mut().unwrap().numeric = numeric;
            }
            St::Dim(n, m, value) => {
                self.check_new(n)?;
                let (kind, supp) = self.measure(m)?;
                let (driving, expr, v) = match value {
                    None => (false, String::new(), 0.0),
                    Some(v) => match plain_number(v) {
                        Some(x) => (true, String::new(), if kind.is_angle() { x.to_radians() } else { x }),
                        None => (true, v.clone(), 0.0),
                    },
                };
                let param = self.d.add_param(v, true);
                let off = self.labels.get(n).copied().unwrap_or(DEFAULT_LABEL);
                self.placed.insert(format!("{n} label"));
                let numeric = numeric(flags)?;
                self.d.dims.push(Some(Dim { kind, name: n.clone(), driving, expr, param, s: 1.0, supp, off, numeric }));
            }
            St::Same(lhs, f, args) => self.same(lhs, f, args)?,
            St::Coincident(a, b) => {
                let (a, b) = (self.point(a)?, self.point(b)?);
                self.d.cons.push(Some(Con::Coincident(a, b)));
            }
            St::Equal(a, b) => {
                let (a, b) = (self.size(a)?, self.size(b)?);
                self.d.cons.push(Some(Con::Equal(a, b)));
            }
            St::HV(h, a, b) => {
                let (a, b) = (self.point(a)?, self.point(b)?);
                self.d.cons.push(Some(if *h { Con::Horizontal(a, b) } else { Con::Vertical(a, b) }));
            }
            St::Rel(a, op, b, inside, near) => self.relation(a, op, b, *inside, *near)?,
        }
        if !flags.is_empty() && matches!(st, St::Header(_) | St::Section | St::Origin(_) | St::Same(..) | St::Coincident(..) | St::Equal(..) | St::HV(..) | St::Rel(..)) {
            return Err(format!("unexpected flags {flags:?}"));
        }
        Ok(())
    }

    fn finish(mut self) -> Result<Doc, Error> {
        let labels: Vec<String> = self.labels.keys().map(|n| format!("{n} label")).collect();
        let unused = self.pos.keys().chain(self.radius.keys()).chain(self.conic.keys()).chain(&labels).find(|n| !self.placed.contains(*n));
        if let Some(n) = unused {
            return Err(Error { line: 0, msg: format!("`where` places {n}, which the construction does not have") });
        }
        // a tangency of two curves other than lines and circles is stated at
        // a point on both, which the file may make after it; else the
        // contact is solved for, starting where the curves come closest
        for k in std::mem::take(&mut self.tangents) {
            let Some(Con::Tangent { a, b, .. }) = self.d.cons[k] else { continue };
            let d = &self.d;
            let special = (d.is_line(a) && d.is_circlelike(b)) || (d.is_circlelike(a) && d.is_circlelike(b));
            if !special && d.tangent_contact(a, b).is_none() {
                let p = contact_guess(&sample_polys(d, a), &sample_polys(d, b), [0.0; 2]);
                let aux = Some((self.d.add_param(p[0], false), self.d.add_param(p[1], false)));
                if let Some(Con::Tangent { aux: x, .. }) = &mut self.d.cons[k] {
                    *x = aux;
                }
            }
        }
        // orientations as the positions have them, measured values as they are now
        for i in self.first_dim..self.d.dims.len() {
            let Some(x) = self.d.dims[i].clone() else { continue };
            let s = self.d.dim_sign(&x);
            self.d.dims[i].as_mut().unwrap().s = s;
            if !x.driving || !x.expr.is_empty() {
                let v = self.d.measure_value(i);
                self.d.params[x.param].v = v;
            }
        }
        Ok(self.d)
    }

    // ------------------------------------------------------------ names

    /// Checks a new name of a variable or dimension. Entities have names of
    /// their own: a point A and a variable A can both be.
    fn check_new(&self, n: &str) -> Result<(), String> {
        if !valid_ident(n) {
            return Err(format!("{n:?} cannot be a name"));
        }
        if self.d.vars.iter().flatten().any(|v| v.name == n) || self.d.dims.iter().flatten().any(|x| x.name == n) {
            return Err(format!("{n} is named twice"));
        }
        if self.d.name_taken(n) {
            return Err(format!("{n} cannot name a variable or dimension"));
        }
        Ok(())
    }

    /// Checks a new name of a point or curve.
    fn check_new_ent(&self, n: &str) -> Result<(), String> {
        if !valid_ident(n) {
            return Err(format!("{n:?} cannot be a name"));
        }
        if RESERVED.contains(&n) {
            return Err(format!("{n} is a keyword, which cannot name a point or curve"));
        }
        if self.names.contains_key(n) {
            return Err(format!("{n} is named twice"));
        }
        Ok(())
    }

    fn name(&mut self, e: usize, n: &str) -> Result<(), String> {
        self.check_new_ent(n)?;
        self.names.insert(n.to_string(), e);
        self.d.ents[e].as_mut().unwrap().name = n.to_string();
        Ok(())
    }

    fn set_flags(&mut self, e: usize, (aux, hidden): (bool, bool)) {
        let x = self.d.ents[e].as_mut().unwrap();
        x.construction = aux;
        x.hidden = hidden;
    }

    fn ent(&self, n: &str) -> Result<usize, String> {
        self.names.get(n).copied().ok_or_else(|| format!("nothing is named {n}"))
    }

    fn point(&self, n: &str) -> Result<usize, String> {
        let e = self.ent(n.trim())?;
        if self.d.is_point(e) { Ok(e) } else { Err(format!("{n} is not a point")) }
    }

    /// The line through the points run together in `s` (`AB`), and whether
    /// it runs from the second to the first.
    fn line_by_points(&self, s: &str) -> Result<(usize, bool), String> {
        let pts = split_points(s).filter(|p| p.len() == 2).ok_or_else(|| format!("nothing is named {s}"))?;
        let (a, b) = (self.point(pts[0])?, self.point(pts[1])?);
        (0..self.d.ents.len())
            .find_map(|l| match self.d.kind(l) {
                Some(Ent::Line { a: x, b: y, .. }) if (*x, *y) == (a, b) => Some((l, false)),
                Some(Ent::Line { a: x, b: y, .. }) if (*x, *y) == (b, a) => Some((l, true)),
                _ => None,
            })
            .ok_or_else(|| format!("there is no line {s}"))
    }

    /// A line, curve or circle, by its name or (a line) by its points.
    fn curve(&self, s: &str) -> Result<usize, String> {
        let s = s.trim();
        match self.names.get(s) {
            Some(&e) if self.d.is_point(e) => Err(format!("{s} is a point, not a curve")),
            Some(&e) => Ok(e),
            None => Ok(self.line_by_points(s)?.0),
        }
    }

    /// A line with a direction: `AB` from A to B, `l`, or `-l` against l.
    fn line(&self, s: &str) -> Result<(usize, bool), String> {
        let s = s.trim();
        let (rev, name) = match s.strip_prefix('-') {
            Some(n) => (true, n.trim()),
            None => (false, s),
        };
        match self.names.get(name) {
            Some(&e) if self.d.is_line(e) => Ok((e, rev)),
            Some(_) => Err(format!("{name} is not a line")),
            None if rev => Err(format!("nothing is named {name}")),
            None => self.line_by_points(s),
        }
    }

    /// A direction: from point A to point B for `AB` (which needs no line
    /// through them), or along a line.
    fn dir(&self, s: &str) -> Result<(usize, usize), String> {
        let s = s.trim();
        if !s.starts_with('-') && !self.names.contains_key(s) {
            if let Some([a, b]) = split_points(s).as_deref() {
                return Ok((self.point(a)?, self.point(b)?));
            }
        }
        let (l, rev) = self.line(s)?;
        let (a, b) = self.d.line_pts(l);
        Ok(if rev { (b, a) } else { (a, b) })
    }

    /// The line or circle of a size: `|AB|`, `len(l)`, `r(k)`.
    fn size(&self, s: &str) -> Result<usize, String> {
        let s = s.trim();
        if let Some(inner) = s.strip_prefix('|').and_then(|t| t.strip_suffix('|')) {
            let l = self.curve(inner)?;
            return if self.d.is_line(l) { Ok(l) } else { Err(format!("{inner} is not a line")) };
        }
        match call(s) {
            Some(("len", a)) => self.line(a).map(|x| x.0),
            Some(("r", a)) => {
                let c = self.curve(a)?;
                if self.d.is_circlelike(c) { Ok(c) } else { Err(format!("{a} is not a circle or arc")) }
            }
            _ => Err(format!("expected |AB|, len(l) or r(k), found {s:?}")),
        }
    }

    // ------------------------------------------------------------ placing

    /// A new point named `n`, where the file places it, else where `guess`
    /// does, else at a spot of its own.
    fn new_point(&mut self, n: &str, guess: impl FnOnce(&R) -> Option<[f64; 2]>) -> Result<usize, String> {
        self.check_new_ent(n)?;
        let p = match self.pos.get(n) {
            Some(&p) => p,
            None => guess(self).filter(|p| p[0].is_finite() && p[1].is_finite()).unwrap_or_else(|| self.next_spot()),
        };
        self.placed.insert(n.to_string());
        let e = self.d.add_point(p[0], p[1]);
        self.name(e, n)?;
        self.lock(n, e);
        Ok(e)
    }

    /// Spots for points the file gives no position, spread out so that no
    /// three are on a line.
    fn next_spot(&mut self) -> [f64; 2] {
        let i = self.spread as f64;
        self.spread += 1;
        let (r, t) = (3.0 + 0.4 * i, 0.7 + 2.399963 * i);
        [r * t.cos(), r * t.sin()]
    }

    /// The point of curve c nearest the next free spot.
    fn nearest_on(&self, c: usize) -> [f64; 2] {
        let i = self.spread as f64;
        let t = 0.7 + 2.399963 * i;
        let at = [3.0 * t.cos(), 3.0 * t.sin()];
        let near = sample_polys(&self.d, c).into_iter().flatten().min_by(|p, q| {
            let d = |p: &[f64; 2]| (p[0] - at[0]).hypot(p[1] - at[1]);
            d(p).total_cmp(&d(q))
        });
        near.unwrap_or(at)
    }

    fn pv(&self, e: usize) -> [f64; 2] {
        self.d.pval(e)
    }

    // ------------------------------------------------------------ statements

    fn make(&mut self, n: &str, f: &str, args: &[String], flags: (bool, bool)) -> Result<(), String> {
        let want = |k: usize| -> Result<(), String> {
            if args.len() == k { Ok(()) } else { Err(format!("{f} takes {k} argument{}", if k == 1 { "" } else { "s" })) }
        };
        let e = match f {
            "mid" => match args.len() {
                1 => {
                    let l = self.line(&args[0])?.0;
                    let (a, b) = self.d.line_pts(l);
                    let (pa, pb) = (self.pv(a), self.pv(b));
                    let p = self.new_point(n, |_| Some([(pa[0] + pb[0]) / 2.0, (pa[1] + pb[1]) / 2.0]))?;
                    self.d.cons.push(Some(Con::Midpoint(p, l)));
                    p
                }
                _ => {
                    want(2)?;
                    let (a, b) = (self.point(&args[0])?, self.point(&args[1])?);
                    let (pa, pb) = (self.pv(a), self.pv(b));
                    let p = self.new_point(n, |_| Some([(pa[0] + pb[0]) / 2.0, (pa[1] + pb[1]) / 2.0]))?;
                    self.d.cons.push(Some(Con::MidpointOf(p, a, b)));
                    p
                }
            },
            "meet" => {
                want(2)?;
                let (a, b) = (self.curve(&args[0])?, self.curve(&args[1])?);
                if a == b {
                    return Err("meet needs two different curves".into());
                }
                let hint = self.d.center_of_val(a);
                let p = self.new_point(n, |r: &R| crossing_near(&r.d, a, b, hint))?;
                self.d.cons.push(Some(Con::OnCurve(p, a)));
                self.d.cons.push(Some(Con::OnCurve(p, b)));
                p
            }
            "reflect" => {
                want(2)?;
                let (s, l) = (self.point(&args[0])?, self.line(&args[1])?.0);
                let ps = self.pv(s);
                let ft = foot(&self.d, ps, l);
                let p = self.new_point(n, |_| Some([2.0 * ft[0] - ps[0], 2.0 * ft[1] - ps[1]]))?;
                self.d.cons.push(Some(Con::Symmetric(s, p, l)));
                p
            }
            "tangent" => {
                want(2)?;
                let (s, c) = (self.point(&args[0])?, self.curve(&args[1])?);
                let ps = self.pv(s);
                let t = self.new_point(n, |r: &R| tangent_contact_near(&r.d, ps, c, ps))?;
                self.d.cons.push(Some(Con::TangentAt { p: s, t, c }));
                t
            }
            "focus" => {
                want(1)?;
                let c = self.conic_arg(&args[0])?;
                let p = self.new_point(n, |r: &R| r.d.focus_pos(c))?;
                self.d.cons.push(Some(Con::Focus { p, c }));
                p
            }
            "circle" => {
                want(1)?;
                let c = self.point(&args[0])?;
                let r = self.radius.get(n).copied().unwrap_or(1.0);
                self.placed.insert(n.to_string());
                self.d.add_circle(c, r)
            }
            "arc" => {
                want(3)?;
                let (c, a, b) = (self.point(&args[0])?, self.point(&args[1])?, self.point(&args[2])?);
                self.d.add_ent(Ent::Arc { c, a, b })
            }
            "ellipse" | "hyperbola" | "parabola" => {
                want(1)?;
                let c = self.point(&args[0])?;
                let kind = match f {
                    "ellipse" => ConicKind::Ellipse,
                    "hyperbola" => ConicKind::Hyperbola,
                    _ => ConicKind::Parabola,
                };
                let (a, b, t) = match self.conic.get(n) {
                    Some(&(a, b, t)) => (a, b.unwrap_or(1.0), t),
                    None if kind == ConicKind::Ellipse => (2.0, 1.0, 0.0),
                    None => (1.0, 1.0, 0.0),
                };
                self.placed.insert(n.to_string());
                self.d.add_conic(kind, c, a, b, t)
            }
            "curve" => {
                want(1)?;
                self.d.add_ent(Ent::Curve { src: args[0].clone() })
            }
            "perp" | "par" => {
                want(2)?;
                let ((a, b), p) = (self.dir(&args[0])?, self.point(&args[1])?);
                let rot = f == "perp";
                let (pa, pb, pp) = (self.pv(a), self.pv(b), self.pv(p));
                let v = [pb[0] - pa[0], pb[1] - pa[1]];
                let v = if rot { [-v[1], v[0]] } else { v };
                let q = self.d.add_helper(pp[0] + v[0], pp[1] + v[1]);
                self.d.cons.push(Some(Con::Offset { q, p, a, b, rot }));
                self.d.add_ent(Ent::Line { a: p, b: q, infinite: true })
            }
            "bisect" => {
                if args.len() != 2 && args.len() != 3 {
                    return Err("bisect takes two lines, and the vertex when they share no point".into());
                }
                let ((l1, r1), (l2, r2)) = (self.line(&args[0])?, self.line(&args[1])?);
                let x = match args.get(2) {
                    Some(x) => self.point(x)?,
                    None => self.d.shared_point(l1, l2).ok_or_else(|| format!("{} and {} share no point: name the vertex, bisect(l1, l2, X)", args[0], args[1]))?,
                };
                let (s1, s2) = (if r1 { -1.0 } else { 1.0 }, if r2 { -1.0 } else { 1.0 });
                let px = self.pv(x);
                let dir = crate::build::bisector_dir(&self.d, l1, l2, (s1, s2)).unwrap_or([1.0, 0.0]);
                let q = self.d.add_helper(px[0] + dir[0], px[1] + dir[1]);
                self.d.cons.push(Some(Con::Bisect { q, x, l1, l2, s1, s2 }));
                self.d.add_ent(Ent::Line { a: x, b: q, infinite: true })
            }
            "asymptote" => {
                want(2)?;
                let c = self.conic_arg(&args[0])?;
                let s = match number(&args[1])? {
                    v if v == 1.0 => 1.0,
                    v if v == -1.0 => -1.0,
                    _ => return Err("asymptote(h, s) takes s = 1 or -1".into()),
                };
                let Some(Ent::Conic { kind: ConicKind::Hyperbola, c: ctr, a, b, t }) = self.d.kind(c).cloned() else {
                    return Err(format!("{} is not a hyperbola", args[0]));
                };
                let (o, a, b, t) = (self.pv(ctr), self.d.params[a].v, self.d.params[b].v, self.d.params[t].v);
                let (e1, e2) = ([t.cos(), t.sin()], [-t.sin(), t.cos()]);
                let q = self.d.add_helper(o[0] + a * e1[0] + s * b * e2[0], o[1] + a * e1[1] + s * b * e2[1]);
                self.d.cons.push(Some(Con::Asymptote { q, c, s }));
                self.d.add_ent(Ent::Line { a: ctr, b: q, infinite: true })
            }
            "directrix" => {
                want(2)?;
                let (c, fp) = (self.conic_arg(&args[0])?, self.point(&args[1])?);
                let pf = self.pv(fp);
                let [pa, pb] = self.d.directrix_pts(c, fp).unwrap_or([pf, [pf[0], pf[1] + 1.0]]);
                let (a, b) = (self.d.add_helper(pa[0], pa[1]), self.d.add_helper(pb[0], pb[1]));
                self.d.cons.push(Some(Con::Directrix { a, b, f: fp, c }));
                self.d.add_ent(Ent::Line { a, b, infinite: true })
            }
            _ => unreachable!(),
        };
        if !self.d.is_point(e) {
            self.name(e, n)?;
            self.lock(n, e);
        }
        self.set_flags(e, flags);
        Ok(())
    }

    fn conic_arg(&self, s: &str) -> Result<usize, String> {
        let c = self.curve(s)?;
        if self.d.is_conic(c) { Ok(c) } else { Err(format!("{s} is not a conic")) }
    }

    /// `X == f(args)`: an existing point (or line) that a construction would make.
    fn same(&mut self, lhs: &[String], f: &str, args: &[String]) -> Result<(), String> {
        let n = args.len();
        let con = match (f, lhs, n) {
            ("mid", [m], 1) => Con::Midpoint(self.point(m)?, self.line(&args[0])?.0),
            ("mid", [m], 2) => Con::MidpointOf(self.point(m)?, self.point(&args[0])?, self.point(&args[1])?),
            ("reflect", [q], 2) => Con::Symmetric(self.point(&args[0])?, self.point(q)?, self.line(&args[1])?.0),
            ("tangent", [t], 2) => Con::TangentAt { p: self.point(&args[0])?, t: self.point(t)?, c: self.curve(&args[1])? },
            ("focus", [p], 1) => Con::Focus { p: self.point(p)?, c: self.conic_arg(&args[0])? },
            ("offset", [q], 3 | 4) => {
                let rot = match args.get(3) {
                    None => false,
                    Some(a) if degrees(a)? == 90f64.to_radians() => true,
                    Some(a) => return Err(format!("offset turns by 90° or not at all, not {a}")),
                };
                Con::Offset { q: self.point(q)?, p: self.point(&args[0])?, a: self.point(&args[1])?, b: self.point(&args[2])?, rot }
            }
            ("bisect", [q], 3) => {
                let ((l1, r1), (l2, r2)) = (self.line(&args[0])?, self.line(&args[1])?);
                Con::Bisect { q: self.point(q)?, x: self.point(&args[2])?, l1, l2, s1: if r1 { -1.0 } else { 1.0 }, s2: if r2 { -1.0 } else { 1.0 } }
            }
            ("asymptote", [q], 2) => {
                let s = if number(&args[1])? < 0.0 { -1.0 } else { 1.0 };
                Con::Asymptote { q: self.point(q)?, c: self.conic_arg(&args[0])?, s }
            }
            ("directrix", [l], 2) => Con::DirectrixOf { l: self.line(l)?.0, f: self.point(&args[1])?, c: self.conic_arg(&args[0])? },
            ("directrix", [a, b], 2) => Con::Directrix { a: self.point(a)?, b: self.point(b)?, f: self.point(&args[1])?, c: self.conic_arg(&args[0])? },
            _ => return Err(format!("cannot read {} == {f}(...) with {n} argument{}", lhs.join(" "), if n == 1 { "" } else { "s" })),
        };
        self.d.cons.push(Some(con));
        Ok(())
    }

    fn relation(&mut self, a: &str, op: &str, b: &str, inside: bool, near: Option<[f64; 2]>) -> Result<(), String> {
        if (inside || near.is_some()) && op != "tan" {
            return Err("`inside` and `near` go with `tan`".into());
        }
        let con = match op {
            "on" => Con::OnCurve(self.point(a)?, self.curve(b)?),
            "conc" => {
                let (a, b) = (self.curve(a)?, self.curve(b)?);
                Con::Concentric(a, b)
            }
            "coll" => Con::Collinear(self.line(a)?.0, self.line(b)?.0),
            "⟂" | "perp" => Con::Perpendicular(self.line(a)?.0, self.line(b)?.0),
            "∥" | "par" => Con::Parallel(self.line(a)?.0, self.line(b)?.0),
            "tan" => {
                let (ea, eb) = (self.curve(a)?, self.curve(b)?);
                let d = &self.d;
                let Some(Con::Tangent { a: ta, b: tb, s, .. }) = d.tangent_con(ea, eb) else { return Err(format!("{a} and {b} cannot be tangent")) };
                let circles = d.is_circlelike(ta) && d.is_circlelike(tb);
                if inside && !circles {
                    return Err("`inside` is for two circles".into());
                }
                let s = if circles { if inside { -1.0 } else { 1.0 } } else { s };
                if near.is_none() {
                    self.tangents.push(self.d.cons.len());
                }
                let aux = near.map(|p| (self.d.add_param(p[0], false), self.d.add_param(p[1], false)));
                Con::Tangent { a: ta, b: tb, s, aux }
            }
            _ => unreachable!(),
        };
        self.d.cons.push(Some(con));
        Ok(())
    }

    /// The dimension kind of a measure, and whether an angle is the
    /// supplement of the one between the directions of its lines.
    fn measure(&self, m: &str) -> Result<(DimKind, bool), String> {
        let m = m.trim();
        if let Some(inner) = m.strip_prefix('|').and_then(|t| t.strip_suffix('|')) {
            let inner = inner.trim();
            if let Ok(l) = self.curve(inner) {
                return if self.d.is_line(l) { Ok((DimKind::Length(l), false)) } else { Err(format!("{inner} is not a line")) };
            }
            return match split_points(inner).as_deref() {
                Some([a, b]) => Ok((DimKind::Distance(self.point(a)?, self.point(b)?), false)),
                _ => Err(format!("nothing is named {inner}")),
            };
        }
        if let Some(pts) = m.strip_prefix('∠').or_else(|| m.strip_prefix('<')) {
            let ps = point_list(pts)?;
            let [p, v, q] = ps.as_slice() else { return Err(format!("∠ takes three points, found {pts:?}")) };
            let (l1, r1) = self.line(&format!("{v}{p}"))?;
            let (l2, r2) = self.line(&format!("{v}{q}"))?;
            return Ok((DimKind::Angle(l1, l2), r1 != r2));
        }
        let (f, inner) = call(m).ok_or_else(|| format!("cannot read the measure {m:?}"))?;
        let args = split_args(inner);
        let one = |k: &str| -> Result<&str, String> {
            match args.as_slice() {
                [a] => Ok(a.as_str()),
                _ => Err(format!("{k} takes one argument")),
            }
        };
        let two = |k: &str| -> Result<(&str, &str), String> {
            match args.as_slice() {
                [a, b] => Ok((a.as_str(), b.as_str())),
                _ => Err(format!("{k} takes two arguments")),
            }
        };
        Ok(match f {
            "angle" => {
                let (a, b) = two(f)?;
                let ((l1, r1), (l2, r2)) = (self.line(a)?, self.line(b)?);
                (DimKind::Angle(l1, l2), r1 != r2)
            }
            "dist" => {
                let (a, b) = two(f)?;
                let p = self.point(a)?;
                match self.names.get(b.trim()) {
                    Some(&q) if self.d.is_point(q) => (DimKind::Distance(p, q), false),
                    _ => (DimKind::PointLine(p, self.line(b)?.0), false),
                }
            }
            "dx" | "dy" => {
                let (a, b) = two(f)?;
                let (a, b) = (self.point(a)?, self.point(b)?);
                (if f == "dx" { DimKind::HDist(a, b) } else { DimKind::VDist(a, b) }, false)
            }
            "len" => (DimKind::Length(self.line(one(f)?)?.0), false),
            "r" | "diam" => {
                let c = self.curve(one(f)?)?;
                if !self.d.is_circlelike(c) {
                    return Err(format!("{f}: not a circle or arc"));
                }
                (if f == "r" { DimKind::Radius(c) } else { DimKind::Diameter(c) }, false)
            }
            "a" | "b" => {
                let c = self.conic_arg(one(f)?)?;
                (if f == "a" { DimKind::ConicA(c) } else { DimKind::ConicB(c) }, false)
            }
            "x" | "y" => {
                let p = self.point(one(f)?)?;
                (if f == "x" { DimKind::CoordX(p) } else { DimKind::CoordY(p) }, false)
            }
            _ => return Err(format!("{f} is not a measure")),
        })
    }
}
