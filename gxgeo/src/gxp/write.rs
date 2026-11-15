//! Writing a document as .gxp text. Each entity is declared by one
//! statement, which also stands for the constraints that make it (a
//! midpoint, a crossing, a perpendicular through a point). The statements
//! come in an order they can be built in: first what is given, then what is
//! constructed from it. The remaining constraints and the driving
//! dimensions follow the statement of the last thing they involve, the
//! measured dimensions come under `find`, and the solved values under
//! `where`.

use super::{RESERVED, XAXIS, YAXIS, degrees, exact, joinable, sig};
use crate::model::*;
use crate::sketch::valid_ident;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

pub fn write(d: &Doc) -> String {
    W::new(d).run()
}

/// How the statement of an entity makes it, beyond the entity itself.
#[derive(Clone)]
enum Def {
    /// A point made by these constraints (two for a crossing).
    Point(Vec<usize>),
    /// A line through helper points, made by this constraint.
    Line(usize),
}

struct Stmt {
    /// Entities the statement declares; the last is its own.
    ents: Vec<usize>,
    /// Entities that other statements declare and this one refers to.
    args: Vec<usize>,
    text: String,
    /// Made from other things (a construction) rather than given.
    made: bool,
    /// A segment, line or polygon, which may also declare its points.
    shape: bool,
    /// Free points with no flags, which may share one `point` line.
    plain_point: bool,
    /// Relations and driving dimensions written under it.
    under: Vec<String>,
}

struct W<'a> {
    d: &'a Doc,
    live: Vec<usize>,
    /// Name of each entity in the file; empty for the ones without (helper
    /// points, the handles of the axes, lines written by their points).
    /// Variables and dimensions have names of their own, which entities may
    /// share: the two never stand in the same place.
    names: Vec<String>,
    taken: HashSet<String>,
    defs: HashMap<usize, Def>,
    /// Helper points: declared with the line they belong to.
    helpers: HashSet<usize>,
    /// Constraints written as part of a statement.
    folded: HashSet<usize>,
    /// Lines written by their points, as `AB`.
    by_points: HashSet<usize>,
}

impl<'a> W<'a> {
    fn new(d: &'a Doc) -> W<'a> {
        let live = (0..d.ents.len()).filter(|&e| d.ent(e).is_some()).collect();
        W {
            d,
            live,
            names: vec![String::new(); d.ents.len()],
            taken: HashSet::new(),
            defs: HashMap::new(),
            helpers: HashSet::new(),
            folded: HashSet::new(),
            by_points: HashSet::new(),
        }
    }

    fn base(&self, e: usize) -> bool {
        self.d.is_fixed_ent(e)
    }

    /// A point only a construction line goes through: hidden and unnamed.
    fn helper_like(&self, e: usize) -> bool {
        self.d.ent(e).is_some_and(|x| matches!(x.kind, Ent::Point { .. }) && x.hidden && x.name.is_empty()) && !self.base(e)
    }

    fn run(mut self) -> String {
        self.name_points();
        self.fold_lines();
        self.fold_points();
        self.name_rest();
        let stmts = self.statements();
        self.emit(stmts)
    }

    // ------------------------------------------------------------ names

    fn fresh(&mut self, prefix: &str, upper: bool) -> String {
        if upper {
            for round in 0.. {
                for c in 'A'..='Z' {
                    let n = if round == 0 { c.to_string() } else { format!("{c}{round}") };
                    if c != 'O' && !self.taken.contains(&n) {
                        self.taken.insert(n.clone());
                        return n;
                    }
                }
            }
        }
        for i in 1.. {
            let n = format!("{prefix}{i}");
            if !self.taken.contains(&n) {
                self.taken.insert(n.clone());
                return n;
            }
        }
        unreachable!()
    }

    /// Keeps `name` for entity e when it is usable and free.
    fn keep(&mut self, e: usize, name: &str) -> bool {
        if valid_ident(name) && !RESERVED.contains(&name) && !self.taken.contains(name) {
            self.taken.insert(name.to_string());
            self.names[e] = name.to_string();
            true
        } else {
            false
        }
    }

    /// Names every point except the base handles and helper-like points,
    /// which are named later only when not folded into a line.
    fn name_points(&mut self) {
        let d = self.d;
        self.taken.extend([XAXIS.to_string(), YAXIS.to_string()]);
        let origin = d.ent(d.origin).map(|x| x.name.clone()).unwrap_or_default();
        if !self.keep(d.origin, &origin) {
            self.names[d.origin] = self.fresh("", true);
        }
        self.names[d.xaxis] = XAXIS.into();
        self.names[d.yaxis] = YAXIS.into();
        let points: Vec<usize> = self.live.iter().copied().filter(|&e| d.is_point(e) && !self.base(e) && !self.helper_like(e)).collect();
        // the names that can stay first, so that a new one takes no name in use
        let mut unnamed = Vec::new();
        for &e in &points {
            let n = d.ent(e).unwrap().name.clone();
            if !self.keep(e, &n) {
                unnamed.push(e);
            }
        }
        for e in unnamed {
            self.names[e] = self.fresh("", true);
        }
    }

    /// Gives names to the helper points left unfolded and to the curves,
    /// except lines that are written by their points.
    fn name_rest(&mut self) {
        let d = self.d;
        for e in self.live.clone() {
            if self.helper_like(e) && !self.helpers.contains(&e) {
                self.names[e] = self.fresh("", true);
            }
        }
        // names the file keeps for curves
        for e in self.live.clone() {
            if !d.is_point(e) && !self.base(e) {
                let n = d.ent(e).unwrap().name.clone();
                if !n.is_empty() {
                    self.keep(e, &n);
                }
            }
        }
        // a line goes by its points when they are named and run together,
        // and no other line has the same ends
        let mut ends: HashMap<(usize, usize), usize> = HashMap::new();
        for &e in &self.live {
            if let Some(Ent::Line { a, b, .. }) = d.kind(e) {
                *ends.entry((*a.min(b), *a.max(b))).or_default() += 1;
            }
        }
        for e in self.live.clone() {
            let Some(Ent::Line { a, b, .. }) = d.kind(e).cloned() else { continue };
            if self.base(e) || !self.names[e].is_empty() || self.defs.contains_key(&e) {
                continue;
            }
            let (na, nb) = (&self.names[a], &self.names[b]);
            if a != b && joinable(na) && joinable(nb) && ends[&(a.min(b), a.max(b))] == 1 && !self.taken.contains(&format!("{na}{nb}")) && !self.taken.contains(&format!("{nb}{na}")) {
                self.by_points.insert(e);
            }
        }
        for e in self.live.clone() {
            if d.is_point(e) || self.base(e) || !self.names[e].is_empty() || self.by_points.contains(&e) {
                continue;
            }
            let prefix = match d.kind(e) {
                Some(Ent::Line { .. }) => "l",
                Some(Ent::Circle { .. }) => "k",
                Some(Ent::Arc { .. }) => "w",
                Some(Ent::Conic { kind: ConicKind::Ellipse, .. }) => "e",
                Some(Ent::Conic { kind: ConicKind::Hyperbola, .. }) => "h",
                Some(Ent::Conic { kind: ConicKind::Parabola, .. }) => "p",
                _ => "g",
            };
            self.names[e] = self.fresh(prefix, false);
        }
    }

    // ------------------------------------------------------------ folding

    /// Every reference to entity e: by entities, constraints and dimensions.
    fn refs_to(&self, e: usize) -> (Vec<usize>, Vec<usize>, usize) {
        let d = self.d;
        let ents = self.live.iter().copied().filter(|&x| d.ent_points(x).contains(&e)).collect();
        let cons = (0..d.cons.len()).filter(|&i| d.cons[i].as_ref().is_some_and(|c| Doc::con_ents(c).contains(&e))).collect();
        let dims = d.dims.iter().flatten().filter(|x| x.kind.ents().contains(&e)).count();
        (ents, cons, dims)
    }

    /// True when helper point q belongs to line l only: no other entity or
    /// dimension refers to it, and no constraint but `k` and directions taken
    /// along l (`offset(P, l)`).
    fn helper_of(&self, q: usize, l: usize, k: usize) -> bool {
        if !self.helper_like(q) {
            return false;
        }
        let (a, b) = self.d.line_pts(l);
        let (ents, cons, dims) = self.refs_to(q);
        ents == [l]
            && dims == 0
            && cons.iter().all(|&i| i == k || matches!(self.d.cons[i], Some(Con::Offset { q: qq, a: x, b: y, .. }) if qq != q && ((x, y) == (a, b) || (x, y) == (b, a))))
    }

    /// True when the direction from point a to point b can be written: by
    /// the names of the points run together, or by a line through them.
    fn dir_ok(&self, a: usize, b: usize) -> bool {
        let named = |p: usize| !self.helper_like(p) && joinable(&self.names[p]);
        (named(a) && named(b) && a != b) || self.line_between(a, b).is_some()
    }

    /// A line from a to b (false) or from b to a (true).
    fn line_between(&self, a: usize, b: usize) -> Option<(usize, bool)> {
        self.live.iter().find_map(|&l| match self.d.kind(l) {
            Some(Ent::Line { a: x, b: y, .. }) if (*x, *y) == (a, b) => Some((l, false)),
            Some(Ent::Line { a: x, b: y, .. }) if (*x, *y) == (b, a) => Some((l, true)),
            _ => None,
        })
    }

    /// Construction lines: through a helper point that a constraint places.
    fn fold_lines(&mut self) {
        let d = self.d;
        for l in self.live.clone() {
            let Some(Ent::Line { a, b, infinite: true }) = d.kind(l).cloned() else { continue };
            if self.base(l) {
                continue;
            }
            let older = |es: &[usize]| es.iter().all(|&e| e < l && d.ent(e).is_some());
            for (k, c) in d.cons.iter().enumerate() {
                let Some(c) = c else { continue };
                if self.folded.contains(&k) {
                    continue;
                }
                let helpers: Vec<usize> = match *c {
                    Con::Offset { q, p, a: x, b: y, .. } if p == a && q == b && older(&[p, x, y]) && self.dir_ok(x, y) && (x, y) != (a, b) => vec![q],
                    Con::Bisect { q, x, l1, l2, .. } if x == a && q == b && older(&[x, l1, l2]) && d.is_line(l1) && d.is_line(l2) => vec![q],
                    Con::Asymptote { q, c, .. } if q == b && older(&[c]) && matches!(d.kind(c), Some(Ent::Conic { c: ctr, .. }) if *ctr == a) => vec![q],
                    Con::Directrix { a: p, b: q, f, c } if p == a && q == b && older(&[f, c]) && d.is_conic(c) => vec![p, q],
                    _ => continue,
                };
                if helpers.iter().all(|&q| self.helper_of(q, l, k)) {
                    self.helpers.extend(helpers);
                    self.folded.insert(k);
                    self.defs.insert(l, Def::Line(k));
                    break;
                }
            }
        }
    }

    /// Points made by a constraint on things older than they are.
    fn fold_points(&mut self) {
        let d = self.d;
        for p in self.live.clone() {
            if !d.is_point(p) || self.base(p) || self.helpers.contains(&p) {
                continue;
            }
            let older = |es: &[usize]| es.iter().all(|&e| e < p && d.ent(e).is_some() && !self.helpers.contains(&e));
            let free: Vec<(usize, &Con)> = d.cons.iter().enumerate().filter_map(|(k, c)| c.as_ref().map(|c| (k, c))).filter(|(k, _)| !self.folded.contains(k)).collect();
            let rank = |c: &Con| -> Option<u8> {
                match *c {
                    Con::MidpointOf(m, a, b) if m == p && older(&[a, b]) => Some(0),
                    Con::Midpoint(m, l) if m == p && older(&[l]) => Some(1),
                    Con::Symmetric(a, b, l) if (b == p && older(&[a, l])) || (a == p && older(&[b, l])) => Some(2),
                    Con::TangentAt { p: s, t, c } if t == p && older(&[s, c]) => Some(3),
                    Con::Focus { p: f, c } if f == p && older(&[c]) => Some(4),
                    _ => None,
                }
            };
            let best = free.iter().filter_map(|(k, c)| rank(c).map(|r| (r, *k))).min();
            let def = match best {
                Some((_, k)) => vec![k],
                None => free.iter().filter(|(_, c)| matches!(**c, Con::OnCurve(q, cv) if q == p && older(&[cv]))).take(2).map(|(k, _)| *k).collect(),
            };
            if !def.is_empty() {
                self.folded.extend(def.iter().copied());
                self.defs.insert(p, Def::Point(def));
            }
        }
    }

    // ------------------------------------------------------------ references

    fn pt(&self, e: usize) -> &str {
        &self.names[e]
    }

    /// A line or curve, undirected.
    fn curve(&self, e: usize) -> String {
        if self.by_points.contains(&e) {
            let (a, b) = self.d.line_pts(e);
            format!("{}{}", self.names[a], self.names[b])
        } else {
            self.names[e].clone()
        }
    }

    /// Line l, from its second point to its first when `rev`.
    fn dline(&self, l: usize, rev: bool) -> String {
        if self.by_points.contains(&l) {
            let (a, b) = self.d.line_pts(l);
            let (a, b) = if rev { (b, a) } else { (a, b) };
            format!("{}{}", self.names[a], self.names[b])
        } else if rev {
            format!("-{}", self.names[l])
        } else {
            self.names[l].clone()
        }
    }

    /// The direction from point a to point b.
    fn dir(&self, a: usize, b: usize) -> String {
        let named = |p: usize| !self.helpers.contains(&p) && joinable(&self.names[p]);
        if named(a) && named(b) {
            return format!("{}{}", self.names[a], self.names[b]);
        }
        let (l, rev) = self.line_between(a, b).expect("direction without a line");
        self.dline(l, rev)
    }

    /// Length of a line or radius of a circle, for equal sizes.
    fn size(&self, e: usize) -> String {
        if self.d.is_line(e) {
            if self.by_points.contains(&e) { format!("|{}|", self.curve(e)) } else { format!("len({})", self.curve(e)) }
        } else {
            format!("r({})", self.curve(e))
        }
    }

    fn flags(&self, e: usize) -> String {
        let x = self.d.ent(e).unwrap();
        match (x.construction, x.hidden) {
            (false, false) => String::new(),
            (true, false) => " [aux]".into(),
            (false, true) => " [hidden]".into(),
            (true, true) => " [aux, hidden]".into(),
        }
    }

    // ------------------------------------------------------------ statements

    fn statements(&mut self) -> Vec<Stmt> {
        let d = self.d;
        let polys = self.polygons();
        let in_poly: HashMap<usize, usize> = polys.iter().enumerate().flat_map(|(i, p)| p.iter().map(move |&l| (l, i))).collect();
        let mut out: Vec<Stmt> = Vec::new();
        let mut done: HashSet<usize> = HashSet::new();
        for e in self.live.clone() {
            if self.base(e) || self.helpers.contains(&e) || done.contains(&e) {
                continue;
            }
            if let Some(&i) = in_poly.get(&e) {
                let lines = &polys[i];
                done.extend(lines.iter().copied());
                let pts: Vec<usize> = lines.iter().map(|&l| d.line_pts(l).0).collect();
                let names: String = pts.iter().map(|&p| self.names[p].as_str()).collect();
                let word = if pts.len() == 3 { "triangle" } else { "polygon" };
                out.push(Stmt::new(lines.clone(), pts, format!("{word} {names}{}", self.flags(e)), false, true));
                continue;
            }
            let fl = self.flags(e);
            let n = self.names[e].clone();
            let s = match (d.kind(e).unwrap().clone(), self.defs.get(&e).cloned()) {
                (Ent::Point { .. }, None) => {
                    let mut s = Stmt::new(vec![e], vec![], format!("point {n}{fl}"), false, false);
                    s.plain_point = fl.is_empty();
                    s
                }
                (Ent::Point { .. }, Some(Def::Point(ks))) => self.point_stmt(e, &ks),
                (Ent::Point { .. }, Some(Def::Line(_))) => unreachable!(),
                (Ent::Line { .. }, Some(Def::Line(k))) => self.line_stmt(e, k),
                (Ent::Line { a, b, infinite }, _) => {
                    let word = if infinite { "line" } else { "segment" };
                    let (na, nb) = (&self.names[a], &self.names[b]);
                    let pts = if joinable(na) && joinable(nb) { format!("{na}{nb}") } else { format!("{na} {nb}") };
                    let text = if self.by_points.contains(&e) { format!("{word} {pts}{fl}") } else { format!("{n} = {word} {pts}{fl}") };
                    Stmt::new(vec![e], vec![a, b], text, false, true)
                }
                (Ent::Circle { c, .. }, _) => Stmt::new(vec![e], vec![c], format!("{n} = circle({}){fl}", self.pt(c)), false, false),
                (Ent::Arc { c, a, b }, _) => Stmt::new(vec![e], vec![c, a, b], format!("{n} = arc({}, {}, {}){fl}", self.pt(c), self.pt(a), self.pt(b)), false, false),
                (Ent::Conic { kind, c, .. }, _) => {
                    let word = match kind {
                        ConicKind::Ellipse => "ellipse",
                        ConicKind::Hyperbola => "hyperbola",
                        ConicKind::Parabola => "parabola",
                    };
                    Stmt::new(vec![e], vec![c], format!("{n} = {word}({}){fl}", self.pt(c)), false, false)
                }
                (Ent::Curve { src }, _) => Stmt::new(vec![e], vec![], format!("{n} = curve({src}){fl}"), false, false),
            };
            out.push(s);
        }
        self.merge_points(out)
    }

    /// Runs of three or more consecutive segments closing a polygon, with
    /// no names or constraints of their own making and the same flags.
    fn polygons(&self) -> Vec<Vec<usize>> {
        let d = self.d;
        let plain = |l: usize| {
            matches!(d.kind(l), Some(Ent::Line { a, b, infinite: false }) if a != b) && self.by_points.contains(&l) && !self.defs.contains_key(&l)
        };
        let flags = |l: usize| d.ent(l).map(|x| (x.construction, x.hidden));
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.live.len() {
            let mut best = 0;
            let mut j = i;
            while j < self.live.len() && plain(self.live[j]) && flags(self.live[j]) == flags(self.live[i]) {
                let l = self.live[j];
                if j > i && d.line_pts(self.live[j - 1]).1 != d.line_pts(l).0 {
                    break;
                }
                let n = j - i + 1;
                let pts: HashSet<usize> = self.live[i..=j].iter().map(|&l| d.line_pts(l).0).collect();
                if pts.len() != n {
                    break;
                }
                if n >= 3 && d.line_pts(l).1 == d.line_pts(self.live[i]).0 {
                    best = n;
                }
                j += 1;
            }
            if best > 0 {
                out.push(self.live[i..i + best].to_vec());
                i += best;
            } else {
                i += 1;
            }
        }
        out
    }

    fn point_stmt(&self, p: usize, ks: &[usize]) -> Stmt {
        let d = self.d;
        let n = self.pt(p);
        let fl = self.flags(p);
        let c = d.cons[ks[0]].as_ref().unwrap();
        let (text, args) = match (ks.len(), c) {
            (2, _) => {
                let cv: Vec<usize> = ks.iter().map(|&k| if let Some(Con::OnCurve(_, c)) = d.cons[k] { c } else { unreachable!() }).collect();
                (format!("{n} = meet({}, {})", self.curve(cv[0]), self.curve(cv[1])), cv)
            }
            (_, Con::OnCurve(_, c)) => {
                let s = Stmt::new(vec![p], vec![*c], format!("point {n} on {}{fl}", self.curve(*c)), true, false);
                return s;
            }
            (_, Con::MidpointOf(_, a, b)) => (format!("{n} = mid({}, {})", self.pt(*a), self.pt(*b)), vec![*a, *b]),
            (_, Con::Midpoint(_, l)) => (format!("{n} = mid({})", self.curve(*l)), vec![*l]),
            (_, Con::Symmetric(a, b, l)) => {
                let src = if *b == p { *a } else { *b };
                (format!("{n} = reflect({}, {})", self.pt(src), self.curve(*l)), vec![src, *l])
            }
            (_, Con::TangentAt { p: s, c, .. }) => (format!("{n} = tangent({}, {})", self.pt(*s), self.curve(*c)), vec![*s, *c]),
            (_, Con::Focus { c, .. }) => (format!("{n} = focus({})", self.curve(*c)), vec![*c]),
            _ => unreachable!(),
        };
        Stmt::new(vec![p], args, format!("{text}{fl}"), true, false)
    }

    fn line_stmt(&self, l: usize, k: usize) -> Stmt {
        let d = self.d;
        let n = self.names[l].clone();
        let fl = self.flags(l);
        let (mut ents, text, args) = match *d.cons[k].as_ref().unwrap() {
            Con::Offset { q, p, a, b, rot } => {
                let f = if rot { "perp" } else { "par" };
                (vec![q], format!("{n} = {f}({}, {})", self.dir(a, b), self.pt(p)), vec![p, a, b])
            }
            Con::Bisect { q, x, l1, l2, s1, s2 } => {
                let shared = d.shared_point(l1, l2) == Some(x);
                let at = if shared { String::new() } else { format!(", {}", self.pt(x)) };
                (vec![q], format!("{n} = bisect({}, {}{at})", self.dline(l1, s1 < 0.0), self.dline(l2, s2 < 0.0)), vec![x, l1, l2])
            }
            Con::Asymptote { q, c, s } => (vec![q], format!("{n} = asymptote({}, {})", self.curve(c), if s < 0.0 { "-1" } else { "1" }), vec![c]),
            Con::Directrix { a, b, f, c } => (vec![a, b], format!("{n} = directrix({}, {})", self.curve(c), self.pt(f)), vec![f, c]),
            _ => unreachable!(),
        };
        // the helper points come before the line
        ents.push(l);
        Stmt::new(ents, args, format!("{text}{fl}"), true, false)
    }

    /// Lets the first segment, line or polygon that uses a free point
    /// declare it, instead of a `point` statement of its own.
    fn merge_points(&self, mut stmts: Vec<Stmt>) -> Vec<Stmt> {
        let mut first_user: HashMap<usize, usize> = HashMap::new();
        for (i, s) in stmts.iter().enumerate() {
            for &a in &s.args {
                first_user.entry(a).or_insert(i);
            }
        }
        let mut gone = HashSet::new();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for i in 0..stmts.len() {
            if !stmts[i].plain_point {
                continue;
            }
            let p = stmts[i].ents[0];
            if let Some(&u) = first_user.get(&p) {
                if stmts[u].shape && u > i {
                    // the points come in the order the statement names them
                    let s = &mut stmts[u];
                    let rank = |e: &usize| s.args.iter().position(|a| a == e);
                    let k = s.ents.iter().take_while(|&e| self.d.is_point(*e) && rank(e) < rank(&p)).count();
                    s.ents.insert(k, p);
                    merged.push((u, p));
                    gone.insert(i);
                }
            }
        }
        for (u, p) in merged {
            stmts[u].args.retain(|&a| a != p);
        }
        stmts.into_iter().enumerate().filter(|(i, _)| !gone.contains(i)).map(|(_, s)| s).collect()
    }

    // ------------------------------------------------------------ relations

    fn relation(&self, c: &Con) -> String {
        let d = self.d;
        let p = |e: usize| self.pt(e).to_string();
        let cv = |e: usize| self.curve(e);
        match *c {
            Con::Coincident(a, b) => format!("{} == {}", p(a), p(b)),
            Con::OnCurve(a, b) => format!("{} on {}", p(a), cv(b)),
            Con::Midpoint(a, l) => format!("{} == mid({})", p(a), cv(l)),
            Con::Horizontal(a, b) => format!("horiz {} {}", p(a), p(b)),
            Con::Vertical(a, b) => format!("vert {} {}", p(a), p(b)),
            Con::Parallel(a, b) => format!("{} ∥ {}", cv(a), cv(b)),
            Con::Perpendicular(a, b) => format!("{} ⟂ {}", cv(a), cv(b)),
            Con::Collinear(a, b) => format!("{} coll {}", cv(a), cv(b)),
            Con::Equal(a, b) => format!("{} == {}", self.size(a), self.size(b)),
            Con::Concentric(a, b) => format!("{} conc {}", cv(a), cv(b)),
            Con::Tangent { a, b, s, aux } => {
                let mut t = format!("{} tan {}", cv(a), cv(b));
                if s < 0.0 && d.is_circlelike(a) && d.is_circlelike(b) {
                    t.push_str(" inside");
                }
                if let Some((x, y)) = aux {
                    t.push_str(&format!(" near ({}, {})", exact(d.params[x].v), exact(d.params[y].v)));
                }
                t
            }
            Con::Symmetric(a, b, l) => format!("{} == reflect({}, {})", p(b), p(a), cv(l)),
            Con::MidpointOf(m, a, b) => format!("{} == mid({}, {})", p(m), p(a), p(b)),
            Con::Offset { q, p: o, a, b, rot } => format!("{} == offset({}, {}, {}{})", p(q), p(o), p(a), p(b), if rot { ", 90°" } else { "" }),
            Con::Bisect { q, x, l1, l2, s1, s2 } => format!("{} == bisect({}, {}, {})", p(q), self.dline(l1, s1 < 0.0), self.dline(l2, s2 < 0.0), p(x)),
            Con::TangentAt { p: s, t, c } => format!("{} == tangent({}, {})", p(t), p(s), cv(c)),
            Con::Focus { p: f, c } => format!("{} == focus({})", p(f), cv(c)),
            Con::Directrix { a, b, f, c } => format!("{} {} == directrix({}, {})", p(a), p(b), cv(c), p(f)),
            Con::DirectrixOf { l, f, c } => format!("{} == directrix({}, {})", cv(l), cv(c), p(f)),
            Con::Asymptote { q, c, s } => format!("{} == asymptote({}, {})", p(q), cv(c), if s < 0.0 { "-1" } else { "1" }),
        }
    }

    /// What dimension `x` measures.
    fn measure(&self, x: &Dim) -> String {
        let d = self.d;
        let p = |e: usize| self.pt(e).to_string();
        match x.kind {
            DimKind::Distance(a, b) => format!("dist({}, {})", p(a), p(b)),
            DimKind::HDist(a, b) => format!("dx({}, {})", p(a), p(b)),
            DimKind::VDist(a, b) => format!("dy({}, {})", p(a), p(b)),
            DimKind::PointLine(a, l) => format!("dist({}, {})", p(a), self.curve(l)),
            DimKind::Length(l) => self.size(l),
            DimKind::Radius(c) => format!("r({})", self.curve(c)),
            DimKind::Diameter(c) => format!("diam({})", self.curve(c)),
            DimKind::ConicA(c) => format!("a({})", self.curve(c)),
            DimKind::ConicB(c) => format!("b({})", self.curve(c)),
            DimKind::CoordX(a) => format!("x({})", p(a)),
            DimKind::CoordY(a) => format!("y({})", p(a)),
            DimKind::Angle(l1, l2) => {
                // ∠PVQ between the rays from a shared end V, when that is the
                // angle measured: the supplement when exactly one line runs
                // toward V
                let ((a1, b1), (a2, b2)) = (d.line_pts(l1), d.line_pts(l2));
                let v = [a1, b1].into_iter().find(|&v| v == a2 || v == b2);
                if let Some(v) = v.filter(|_| self.by_points.contains(&l1) && self.by_points.contains(&l2)) {
                    let (r1, r2) = (a1 != v, a2 != v);
                    if (r1 != r2) == x.supp {
                        let other = |a: usize, b: usize| if a == v { b } else { a };
                        return format!("∠{}{}{}", p(other(a1, b1)), p(v), p(other(a2, b2)));
                    }
                }
                format!("angle({}, {})", self.dline(l1, false), self.dline(l2, x.supp))
            }
        }
    }

    fn dim_text(&self, x: &Dim) -> String {
        let mut t = format!("{}: {}", x.name, self.measure(x));
        if x.driving {
            let v = if !x.expr.is_empty() {
                x.expr.clone()
            } else if x.kind.is_angle() {
                format!("{}°", degrees(self.d.params[x.param].v))
            } else {
                exact(self.d.params[x.param].v)
            };
            t.push_str(&format!(" = {v}"));
        }
        if x.numeric {
            t.push_str(" [numeric]");
        }
        t
    }

    fn var_text(&self, v: &Var) -> String {
        let p = &self.d.params[v.param];
        let mut t = if !v.expr.is_empty() {
            format!("{} = {}", v.name, v.expr)
        } else if p.locked {
            format!("{} = {}", v.name, exact(p.v))
        } else {
            format!("{} ~ {}", v.name, exact(p.v))
        };
        if v.numeric {
            t.push_str(" [numeric]");
        }
        t
    }

    // ------------------------------------------------------------ output

    fn emit(&self, mut stmts: Vec<Stmt>) -> String {
        let d = self.d;
        // order: each statement after the ones declaring what it uses, else
        // by the age of its own entity
        let mut decl: HashMap<usize, usize> = HashMap::new();
        for (i, s) in stmts.iter().enumerate() {
            for &e in &s.ents {
                decl.insert(e, i);
            }
        }
        let n = stmts.len();
        let mut indeg = vec![0; n];
        let mut next: Vec<Vec<usize>> = vec![vec![]; n];
        for (i, s) in stmts.iter().enumerate() {
            let mut from: Vec<usize> = s.args.iter().filter_map(|a| decl.get(a).copied()).filter(|&j| j != i).collect();
            from.sort();
            from.dedup();
            for j in from {
                next[j].push(i);
                indeg[i] += 1;
            }
        }
        let key = |i: usize| *stmts[i].ents.iter().max().unwrap();
        let mut heap: BinaryHeap<Reverse<(usize, usize)>> = (0..n).filter(|&i| indeg[i] == 0).map(|i| Reverse((key(i), i))).collect();
        let mut order = Vec::with_capacity(n);
        while let Some(Reverse((_, i))) = heap.pop() {
            order.push(i);
            for &j in &next[i] {
                indeg[j] -= 1;
                if indeg[j] == 0 {
                    heap.push(Reverse((key(j), j)));
                }
            }
        }
        // a cycle cannot come from a document the interfaces built; keep
        // whatever is left in the order of its entities
        let mut rest: Vec<usize> = (0..n).filter(|i| !order.contains(i)).collect();
        rest.sort_by_key(|&i| key(i));
        order.extend(rest);
        // a statement using something constructed is constructed too
        for &i in &order {
            let made = stmts[i].made || stmts[i].args.iter().any(|a| decl.get(a).is_some_and(|&j| stmts[j].made));
            stmts[i].made = made;
        }
        let final_order: Vec<usize> = order.iter().copied().filter(|&i| !stmts[i].made).chain(order.iter().copied().filter(|&i| stmts[i].made)).collect();
        let mut pos = vec![0; n];
        for (k, &i) in final_order.iter().enumerate() {
            pos[i] = k;
        }
        // relations and driving dimensions go under the last statement they use
        // (and the dimensions are kept in the order they come in, for `where`)
        let mut top: Vec<String> = Vec::new();
        let (mut top_dims, mut find_dims) = (Vec::new(), Vec::new());
        let mut under_dims: Vec<Vec<usize>> = vec![vec![]; n];
        let mut attach = |stmts: &mut Vec<Stmt>, ents: Vec<usize>, text: String, dim: Option<usize>| match ents.iter().filter_map(|e| decl.get(e)).max_by_key(|&&i| pos[i]) {
            Some(&i) => {
                stmts[i].under.push(text);
                under_dims[i].extend(dim);
            }
            None => {
                top.push(text);
                top_dims.extend(dim);
            }
        };
        for (k, c) in d.cons.iter().enumerate() {
            if let Some(c) = c.as_ref().filter(|_| !self.folded.contains(&k)) {
                attach(&mut stmts, Doc::con_ents(c), self.relation(c), None);
            }
        }
        let mut find = Vec::new();
        for (i, x) in d.dims.iter().enumerate() {
            let Some(x) = x else { continue };
            if x.driving {
                attach(&mut stmts, x.kind.ents(), self.dim_text(x), Some(i));
            } else {
                find.push(self.dim_text(x));
                find_dims.push(i);
            }
        }
        let given_first = final_order.iter().filter(|&&i| !stmts[i].made).chain(final_order.iter().filter(|&&i| stmts[i].made));
        let dim_order: Vec<usize> = top_dims.into_iter().chain(given_first.flat_map(|&i| under_dims[i].clone())).chain(find_dims).collect();
        let mut given = Vec::new();
        for v in d.vars.iter().flatten() {
            if v.expr.is_empty() {
                given.push(self.var_text(v));
            } else {
                find.push(self.var_text(v));
            }
        }
        given.extend(top);

        let mut out = format!("gxp {}\n", super::VERSION);
        if self.names[d.origin] != "O" {
            out.push_str(&format!("origin {}\n", self.names[d.origin]));
        }
        for (e, n) in [(d.xaxis, XAXIS), (d.yaxis, YAXIS)] {
            let x = d.ent(e).unwrap();
            if (x.construction, x.hidden) != (true, false) {
                let f = self.flags(e);
                out.push_str(&format!("{n} {}\n", if f.is_empty() { "[]" } else { f.trim() }));
            }
        }
        let section = |out: &mut String, title: &str, lines: Vec<String>| {
            if !lines.is_empty() {
                out.push_str(&format!("\n{title}\n"));
                for l in lines {
                    out.push_str(&format!("  {l}\n"));
                }
            }
        };
        let block = |which: bool| -> Vec<String> {
            let mut lines: Vec<String> = Vec::new();
            let mut run: Vec<&str> = Vec::new();
            let flush = |lines: &mut Vec<String>, run: &mut Vec<&str>| {
                if !run.is_empty() {
                    lines.push(format!("point {}", run.join(" ")));
                    run.clear();
                }
            };
            for &i in final_order.iter().filter(|&&i| stmts[i].made == which) {
                let s = &stmts[i];
                if s.plain_point && s.under.is_empty() {
                    run.push(self.pt(s.ents[0]));
                    continue;
                }
                flush(&mut lines, &mut run);
                lines.push(s.text.clone());
                lines.extend(s.under.iter().map(|u| format!("  {u}")));
            }
            flush(&mut lines, &mut run);
            lines
        };
        given.extend(block(false));
        section(&mut out, "given", given);
        section(&mut out, "construct", block(true));
        section(&mut out, "find", find);
        section(&mut out, "where", self.solution(&final_order, &stmts, &dim_order));
        out
    }

    /// The `where` lines: positions of the points, radii and shapes of the
    /// curves, places of the dimension labels.
    fn solution(&self, order: &[usize], stmts: &[Stmt], dims: &[usize]) -> Vec<String> {
        let d = self.d;
        let mut out = Vec::new();
        for &i in order {
            for &e in &stmts[i].ents {
                if self.helpers.contains(&e) {
                    continue;
                }
                match *d.kind(e).unwrap() {
                    Ent::Point { x, y } => {
                        let s = format!("{} ({}, {})", self.pt(e), exact(d.params[x].v), exact(d.params[y].v));
                        out.push(s + &self.fixed(&[("x", x), ("y", y)]));
                    }
                    Ent::Circle { r, .. } => {
                        let s = format!("{} r {}", self.names[e], exact(d.params[r].v));
                        out.push(s + &self.fixed(&[("r", r)]));
                    }
                    Ent::Conic { kind, a, b, t, .. } => {
                        let mut s = format!("{} a {}", self.names[e], exact(d.params[a].v));
                        let mut vals = vec![("a", a), ("angle", t)];
                        // a parabola's b is the constant 1
                        if kind != ConicKind::Parabola {
                            s.push_str(&format!(" b {}", exact(d.params[b].v)));
                            vals.insert(1, ("b", b));
                        }
                        s.push_str(&format!(" angle {}°", degrees(d.params[t].v)));
                        out.push(s + &self.fixed(&vals));
                    }
                    _ => {}
                }
            }
        }
        for x in dims.iter().filter_map(|&i| d.dims[i].as_ref()) {
            // where a label goes is up to the eye: rounded
            let off = x.off.map(|v| sig(v, 6));
            if off != super::read::DEFAULT_LABEL.map(|v| sig(v, 6)) {
                out.push(format!("{} label ({}, {})", x.name, off[0], off[1]));
            }
        }
        out
    }
}

impl W<'_> {
    /// ` [fixed]` when all of the named parameters are locked, ` [fixed x]`
    /// for some of them.
    fn fixed(&self, vals: &[(&str, usize)]) -> String {
        let locked: Vec<&str> = vals.iter().filter(|(_, p)| self.d.params[*p].locked).map(|(n, _)| *n).collect();
        if locked.is_empty() {
            String::new()
        } else if locked.len() == vals.len() {
            " [fixed]".into()
        } else {
            format!(" [fixed {}]", locked.join(" "))
        }
    }
}

impl Stmt {
    fn new(ents: Vec<usize>, args: Vec<usize>, text: String, made: bool, shape: bool) -> Stmt {
        Stmt { ents, args, text, made, shape, plain_point: false, under: vec![] }
    }
}
