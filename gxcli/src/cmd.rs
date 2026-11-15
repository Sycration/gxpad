//! The session: a document, the commands that are not statements, and what
//! each line changed.

use gxgeo::ops::{self, Document, EntKind, Formula, Lang, Target};

pub struct Session {
    pub doc: Document,
    /// The file opened or saved last, for `save` with no file.
    file: Option<String>,
    /// The last formula, with its target and the extra search time it had.
    last: Option<(Target, u64, Formula)>,
}

/// What the document holds before a change: everything at or after these
/// indices is new.
struct Mark {
    ents: usize,
    dims: usize,
    vars: usize,
}

impl Session {
    pub fn new() -> Session {
        Session { doc: Document::new(), file: None, last: None }
    }

    /// Runs one line; returns what to print.
    pub fn run(&mut self, line: &str) -> Result<Vec<String>, String> {
        let line = line.split('#').next().unwrap().trim();
        if line.is_empty() {
            return Ok(vec![]);
        }
        let (word, rest) = match line.split_once(char::is_whitespace) {
            Some((w, r)) => (w, r.trim()),
            None => (line, ""),
        };
        match word {
            "??" if rest.is_empty() => self.derive_more(),
            "?" => self.ask(rest),
            _ if line.starts_with('?') && !line.starts_with("??") => self.ask(line[1..].trim()),
            "help" => Ok(crate::help::help(rest)),
            "ls" => self.ls(rest == "all"),
            "plot" => {
                let n: Vec<usize> = rest.split_whitespace().filter_map(|w| w.parse().ok()).collect();
                Ok(crate::plot::plot(&self.doc, n.first().copied().unwrap_or(72), n.get(1).copied().unwrap_or(24)))
            }
            "code" => self.code(rest),
            "latex" => Ok(vec![self.formula()?.latex.clone()]),
            "png" => self.png(rest),
            "undo" | "redo" => {
                let n: usize = if rest.is_empty() { 1 } else { rest.parse().map_err(|_| format!("{word} N"))? };
                let done = (0..n).take_while(|_| if word == "undo" { ops::undo(&mut self.doc) } else { ops::redo(&mut self.doc) }).count();
                if done == 0 {
                    return Err(format!("nothing to {word}"));
                }
                Ok(vec![self.status()])
            }
            "new" => {
                ops::clear(&mut self.doc);
                self.file = None;
                Ok(vec![])
            }
            "open" => self.open(rest),
            "run" => self.run_file(rest),
            "save" => self.save(rest),
            "del" => self.delete(rest),
            "name" => match rest.split_whitespace().collect::<Vec<_>>().as_slice() {
                [old, new] => match ops::rename(&mut self.doc, old, new)? {
                    true => Ok(vec![]),
                    false => Err(format!("{new} cannot be a name, or is taken")),
                },
                _ => Err("name OLD NEW".into()),
            },
            "unset" => {
                let i = self.dim(rest).ok_or_else(|| format!("no dimension {rest}"))?;
                ops::set_dim(&mut self.doc, i, "")?;
                Ok(vec![format!("{rest} is measured"), self.status()])
            }
            _ => match self.set(line) {
                Some(r) => r,
                None => self.apply(line),
            },
        }
    }

    // ------------------------------------------------------------ changes

    fn mark(&self) -> Mark {
        Mark { ents: ops::ent_count(&self.doc), dims: ops::dim_count(&self.doc), vars: ops::var_count(&self.doc) }
    }

    /// A statement of the .gxp format, or several on lines of their own.
    fn apply(&mut self, text: &str) -> Result<Vec<String>, String> {
        let m = self.mark();
        if !ops::apply_gxp(&mut self.doc, text).map_err(|e| one_line(text, e))? {
            return Err("rejected: the constraints would not solve, or already say this".into());
        }
        let mut out = self.made(&m);
        out.push(self.status());
        Ok(out)
    }

    /// `d1 = 6`, `s = 2`, `q = d1/d2`: a new value of a dimension or variable
    /// that is there. None when the line is not one.
    fn set(&mut self, line: &str) -> Option<Result<Vec<String>, String>> {
        if line.contains("==") {
            return None;
        }
        let (name, value) = line.split_once('=')?;
        let (name, value) = (name.trim(), value.trim());
        if let Some(i) = self.dim(name) {
            return Some(match ops::set_dim(&mut self.doc, i, value) {
                Ok(true) => Ok(vec![format!("{name} = {}", num(ops::dim_value(&self.doc, i).unwrap_or(f64::NAN))), self.status()]),
                Ok(false) => Err(format!("{name} cannot be {value}")),
                Err(e) => Err(e),
            });
        }
        let i = self.var(name)?;
        let r = match value.parse::<f64>() {
            Ok(v) => ops::set_var(&mut self.doc, i, v),
            Err(_) => ops::set_var_expr(&mut self.doc, i, value),
        };
        Some(match r {
            Ok(true) => Ok(vec![format!("{name} = {}", num(ops::var_value(&self.doc, i).unwrap_or(f64::NAN))), self.status()]),
            Ok(false) => Err(format!("{name} cannot be {value}")),
            Err(e) => Err(e),
        })
    }

    fn delete(&mut self, rest: &str) -> Result<Vec<String>, String> {
        let (mut ents, mut dims, mut vars) = (vec![], vec![], vec![]);
        for n in rest.split_whitespace() {
            if let Some(i) = self.dim(n) {
                dims.push(i);
            } else if let Some(i) = self.var(n) {
                vars.push(i);
            } else {
                ents.push(ops::ent_by_name(&self.doc, n)?);
            }
        }
        if !ops::delete(&mut self.doc, &ents, &[], &dims, &vars)? {
            return Err("rejected: the constraints would not solve without it".into());
        }
        Ok(vec![self.status()])
    }

    /// What was made since mark `m`.
    fn made(&self, m: &Mark) -> Vec<String> {
        let d = &self.doc;
        let mut out = Vec::new();
        let mut pts = Vec::new();
        for e in m.ents..ops::ent_count(d) {
            let (Ok(kind), Ok(name)) = (ops::ent_kind(d, e), ops::ent_name(d, e)) else { continue };
            if ops::ent_hidden(d, e).unwrap_or(false) && name.is_empty() {
                continue;
            }
            match kind {
                EntKind::Point => {
                    let p = ops::point_pos(d, e).unwrap();
                    pts.push(format!("{name} ({}, {})", num(p[0]), num(p[1])));
                }
                EntKind::Line if name.is_empty() => {
                    let (a, b, inf) = ops::line_pts(d, e).unwrap();
                    let n = |p| ops::ent_name(d, p).unwrap_or_default();
                    out.push(format!("+ {} {}{}", if inf { "line" } else { "segment" }, n(a), n(b)));
                }
                k => out.push(format!("+ {name} ({})", k.name())),
            }
        }
        if !pts.is_empty() {
            out.insert(0, format!("+ {}", pts.join("   ")));
        }
        for i in m.dims..ops::dim_count(d) {
            let (Ok(name), Ok(v), Ok(driving)) = (ops::dim_name(d, i), ops::dim_value(d, i), ops::dim_driving(d, i)) else { continue };
            let deg = if ops::dim_kind(d, i) == Ok(ops::DimType::Angle) { format!(" ({}°)", num(v.to_degrees())) } else { String::new() };
            out.push(format!("+ {name} = {}{deg}   {}", num(v), if driving { "driving" } else { "measured" }));
        }
        for i in m.vars..ops::var_count(d) {
            let (Ok(name), Ok(v), Ok(e)) = (ops::var_name(d, i), ops::var_value(d, i), ops::var_expr(d, i)) else { continue };
            out.push(if e.is_empty() { format!("+ {name} = {}", num(v)) } else { format!("+ {name} = {e}") });
        }
        out
    }

    fn status(&self) -> String {
        let mut s = format!("dof {}", ops::dof(&self.doc));
        let c = ops::conflict_count(&self.doc);
        if c > 0 {
            s.push_str(&format!(", {c} not satisfied"));
        }
        for e in ops::errors(&self.doc) {
            s.push_str(&format!("\n  ! {e}"));
        }
        s
    }

    // ------------------------------------------------------------ names

    fn dim(&self, name: &str) -> Option<usize> {
        (0..ops::dim_count(&self.doc)).find(|&i| ops::dim_name(&self.doc, i).is_ok_and(|n| n == name))
    }

    fn var(&self, name: &str) -> Option<usize> {
        (0..ops::var_count(&self.doc)).find(|&i| ops::var_name(&self.doc, i).is_ok_and(|n| n == name))
    }

    fn target(&self, name: &str) -> Option<Target> {
        self.dim(name).map(Target::Dim).or_else(|| self.var(name).map(Target::Var))
    }

    /// A name no dimension or variable has, starting with `prefix`.
    fn fresh(&self, prefix: &str) -> String {
        (1..).map(|i| format!("{prefix}{i}")).find(|n| self.target(n).is_none()).unwrap()
    }

    // ------------------------------------------------------------ formulas

    /// `? d1`, `? |AC|`, `? q = d1/d2`, `? h: dist(A, BC)`.
    fn ask(&mut self, what: &str) -> Result<Vec<String>, String> {
        if what.is_empty() {
            return Err("? what: a measure (? |AB|), a dimension or a variable".into());
        }
        if let Some(t) = self.target(what) {
            return self.derive(t, 0, vec![]);
        }
        let ident = |s: &str| s.chars().next().is_some_and(|c| c.is_alphabetic()) && s.chars().all(|c| c.is_alphanumeric() || c == '_');
        // a dimension or variable with the name given
        let named = match (what.split_once(':'), what.split_once('=')) {
            (Some((n, _)), _) if ident(n.trim()) => Some(n.trim()),
            (_, Some((n, _))) if ident(n.trim()) && !what.contains("==") => Some(n.trim()),
            _ => None,
        };
        if let Some(n) = named {
            let m = self.mark();
            if !ops::apply_gxp(&mut self.doc, what).map_err(|e| one_line(what, e))? {
                return Err("rejected".into());
            }
            let made = self.made(&m);
            let t = self.target(n).ok_or_else(|| format!("no dimension or variable {n}"))?;
            return self.derive(t, 0, made);
        }
        // a measure: the dimension measuring it, made when there is none
        let prefix = if what.starts_with('∠') || what.starts_with('<') || what.starts_with("angle(") {
            "θ"
        } else if what.starts_with("r(") {
            "r"
        } else if what.starts_with("diam(") {
            "D"
        } else {
            "d"
        };
        let stmt = format!("{}: {what}", self.fresh(prefix));
        let mut trial = ops::copy(&self.doc);
        if !ops::apply_gxp(&mut trial, &stmt).map_err(|e| one_line(&stmt, e))? {
            return Err("rejected".into());
        }
        let j = ops::dim_count(&trial) - 1;
        let same = |i: usize| -> bool {
            let ents = |d: &Document, k: usize| {
                let mut e = ops::dim_ents(d, k).unwrap_or_default();
                e.sort();
                e
            };
            ops::dim_kind(&self.doc, i) == ops::dim_kind(&trial, j)
                && ents(&self.doc, i) == ents(&trial, j)
                && (ops::dim_value(&self.doc, i).unwrap_or(f64::NAN) - ops::dim_value(&trial, j).unwrap_or(f64::NAN)).abs() < 1e-9
        };
        if let Some(i) = (0..ops::dim_count(&self.doc)).find(|&i| ops::dim_name(&self.doc, i).is_ok() && same(i)) {
            return self.derive(Target::Dim(i), 0, vec![]);
        }
        let m = self.mark();
        ops::apply_gxp(&mut self.doc, &stmt)?;
        let made = self.made(&m);
        self.derive(Target::Dim(j), 0, made)
    }

    fn derive(&mut self, t: Target, extra_ms: u64, mut out: Vec<String>) -> Result<Vec<String>, String> {
        let f = ops::derive(&self.doc, t, extra_ms, None)?;
        out.push(f.plain.clone());
        let deg = if f.is_angle { format!(" ({}°)", num(f.value.to_degrees())) } else { String::new() };
        let how = if f.exact { "exact" } else { "approximate: part of it was solved numerically (?? searches longer)" };
        out.push(format!("  ≈ {}{deg}   {how}", num(f.value)));
        self.last = Some((t, extra_ms, f));
        Ok(out)
    }

    /// `??`: the last formula, searched for 10 s longer.
    fn derive_more(&mut self) -> Result<Vec<String>, String> {
        let (t, extra, _) = self.last.as_ref().ok_or("no formula yet: ? first")?;
        let (t, extra) = (*t, *extra + 10_000);
        self.derive(t, extra, vec![])
    }

    fn formula(&self) -> Result<&Formula, String> {
        self.last.as_ref().map(|l| &l.2).ok_or_else(|| "no formula yet: ? first".into())
    }

    fn code(&self, lang: &str) -> Result<Vec<String>, String> {
        let lang = match lang {
            "py" => "python",
            "mma" => "mathematica",
            "" => "python",
            l => l,
        };
        let lang = Lang::from_name(lang).map_err(|_| "code c | rust | js | py | mma | maxima".to_string())?;
        Ok(ops::formula_code(self.formula()?, lang).lines().map(String::from).collect())
    }

    fn png(&self, file: &str) -> Result<Vec<String>, String> {
        let f = self.formula()?;
        let file = if file.is_empty() { format!("{}.png", f.name) } else { file.to_string() };
        let png = ops::latex_png(&f.latex, 16.0, 192.0, [0, 0, 0])?;
        std::fs::write(&file, png).map_err(|e| format!("{file}: {e}"))?;
        Ok(vec![format!("wrote {file}")])
    }

    // ------------------------------------------------------------ files

    /// The document as .gxp text; without `all`, only the construction.
    fn ls(&self, all: bool) -> Result<Vec<String>, String> {
        let text = ops::to_gxp(&self.doc)?;
        let text = if all { &text[..] } else { text.split("\nwhere\n").next().unwrap() };
        Ok(text.lines().skip(1).skip_while(|l| l.is_empty()).map(String::from).collect())
    }

    fn open(&mut self, file: &str) -> Result<Vec<String>, String> {
        if file.is_empty() {
            return Err("open FILE.gxp".into());
        }
        if !file.ends_with(".gxp") {
            return self.run_file(file);
        }
        let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        ops::load_gxp(&mut self.doc, &text)?;
        self.file = Some(file.to_string());
        Ok(vec![format!("opened {file}"), self.status()])
    }

    fn run_file(&mut self, file: &str) -> Result<Vec<String>, String> {
        let text = std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        let mut out = Vec::new();
        for line in text.lines().filter(|l| !l.split('#').next().unwrap().trim().is_empty()) {
            out.push(format!("> {line}"));
            match self.run(line) {
                Ok(o) => out.extend(o.into_iter().map(|l| format!("  {l}"))),
                Err(e) => return Err(format!("{e}\n  (in {file}: {line})")),
            }
        }
        Ok(out)
    }

    fn save(&mut self, file: &str) -> Result<Vec<String>, String> {
        let file = if file.is_empty() { self.file.clone().ok_or("save FILE.gxp")? } else { file.to_string() };
        if !file.ends_with(".gxp") {
            return Err("save FILE.gxp".into());
        }
        std::fs::write(&file, ops::to_gxp(&self.doc)?).map_err(|e| format!("{file}: {e}"))?;
        self.file = Some(file.clone());
        Ok(vec![format!("saved {file}")])
    }
}

/// An error of applying `text`, without the line number when it is one line.
fn one_line(text: &str, e: String) -> String {
    match e.strip_prefix("line 1: ") {
        Some(m) if !text.contains('\n') => m.to_string(),
        _ => e,
    }
}

/// `v` to 6 significant digits.
pub fn num(v: f64) -> String {
    if v == 0.0 || !v.is_finite() {
        return if v.is_finite() { "0".into() } else { format!("{v}") };
    }
    let dec = (5 - v.abs().log10().floor() as i32).clamp(0, 12) as usize;
    let s = format!("{v:.dec$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.') } else { &s };
    if s == "-0" { "0".into() } else { s.into() }
}
