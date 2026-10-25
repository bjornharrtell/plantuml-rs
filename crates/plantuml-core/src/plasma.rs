//! Tree of qualified names (`Plasma`, `Quark`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub const MAGIC_SEPARATOR: &str = "\u{1}";

struct PlasmaInner<D> {
    separator: RefCell<String>,
    quarks: RefCell<Vec<Quark<D>>>,
    stats: RefCell<HashMap<String, (Quark<D>, usize)>>,
}

struct QuarkInner<D> {
    plasma: Rc<PlasmaInner<D>>,
    parent: Option<Quark<D>>,
    name: String,
    qualified_name: String,
    data: RefCell<Option<D>>,
    children: RefCell<Vec<(String, Quark<D>)>>,
}

/// A node of the namespace tree.
pub struct Quark<D>(Rc<QuarkInner<D>>);

impl<D> Clone for Quark<D> {
    fn clone(&self) -> Self {
        Quark(self.0.clone())
    }
}

impl<D> PartialEq for Quark<D> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl<D> std::fmt::Debug for Quark<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Quark({})", self.0.qualified_name)
    }
}

/// All the quarks of a diagram.
pub struct Plasma<D> {
    inner: Rc<PlasmaInner<D>>,
    root: Quark<D>,
}

impl<D> Clone for Plasma<D> {
    fn clone(&self) -> Self {
        Plasma {
            inner: self.inner.clone(),
            root: self.root.clone(),
        }
    }
}

impl<D: Clone> Default for Plasma<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Clone> Plasma<D> {
    pub fn new() -> Self {
        let inner = Rc::new(PlasmaInner {
            separator: RefCell::new(MAGIC_SEPARATOR.to_string()),
            quarks: RefCell::new(Vec::new()),
            stats: RefCell::new(HashMap::new()),
        });
        let root = Quark::create(&inner, None, "");
        Plasma { inner, root }
    }

    pub fn root(&self) -> Quark<D> {
        self.root.clone()
    }

    pub fn separator(&self) -> String {
        self.inner.separator.borrow().clone()
    }

    pub fn set_separator(&self, separator: Option<&str>) {
        *self.inner.separator.borrow_mut() = separator.unwrap_or(MAGIC_SEPARATOR).to_string();
    }

    pub fn has_separator(&self) -> bool {
        *self.inner.separator.borrow() != MAGIC_SEPARATOR
    }

    pub fn quarks(&self) -> Vec<Quark<D>> {
        self.inner.quarks.borrow().clone()
    }

    pub fn first_with_name(&self, name: &str) -> Option<Quark<D>> {
        self.inner.stats.borrow().get(name).map(|e| e.0.clone())
    }

    pub fn count_by_name(&self, name: &str) -> usize {
        self.inner.stats.borrow().get(name).map_or(0, |e| e.1)
    }
}

impl<D: Clone> Quark<D> {
    fn create(plasma: &Rc<PlasmaInner<D>>, parent: Option<&Quark<D>>, name: &str) -> Quark<D> {
        let qualified_name = match parent {
            Some(p) if p.0.parent.is_some() => {
                format!(
                    "{}{}{}",
                    p.0.qualified_name,
                    plasma.separator.borrow(),
                    name
                )
            }
            _ => name.to_string(),
        };
        let q = Quark(Rc::new(QuarkInner {
            plasma: plasma.clone(),
            parent: parent.cloned(),
            name: name.to_string(),
            qualified_name,
            data: RefCell::new(None),
            children: RefCell::new(Vec::new()),
        }));
        plasma.quarks.borrow_mut().push(q.clone());
        let mut stats = plasma.stats.borrow_mut();
        match stats.get_mut(name) {
            Some(e) => e.1 += 1,
            None => {
                stats.insert(name.to_string(), (q.clone(), 1));
            }
        }
        q
    }

    pub fn parent(&self) -> Option<Quark<D>> {
        self.0.parent.clone()
    }

    pub fn name(&self) -> &str {
        &self.0.name
    }

    pub fn qualified_name(&self) -> &str {
        &self.0.qualified_name
    }

    /// Names joined with dots (`toStringPoint`).
    pub fn to_string_point(&self) -> String {
        match &self.0.parent {
            Some(p) if p.0.parent.is_some() => format!("{}.{}", p.to_string_point(), self.0.name),
            _ => self.0.name.clone(),
        }
    }

    pub fn is_root(&self) -> bool {
        self.0.parent.is_none()
    }

    pub fn data(&self) -> Option<D> {
        self.0.data.borrow().clone()
    }

    /// Sets the data; a quark holds its data once.
    pub fn set_data(&self, data: D) {
        let mut slot = self.0.data.borrow_mut();
        assert!(slot.is_none(), "quark already has data");
        *slot = Some(data);
    }

    pub fn child_if_exists(&self, name: &str) -> Option<Quark<D>> {
        self.0
            .children
            .borrow()
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, q)| q.clone())
    }

    /// The descendant at a separator-joined path, created when missing.
    pub fn child(&self, full: &str) -> Quark<D> {
        let plasma_sep = self.0.plasma.separator.borrow().clone();
        if plasma_sep == MAGIC_SEPARATOR {
            return self.direct_child(full);
        }
        let mut full = clean(full, &plasma_sep);
        let mut current = self.clone();
        loop {
            match full.find(&plasma_sep) {
                None => return current.direct_child(&full),
                Some(idx) => {
                    current = current.direct_child(&full[..idx]);
                    full = clean(&full[idx + plasma_sep.len()..], &plasma_sep);
                }
            }
        }
    }

    fn direct_child(&self, name: &str) -> Quark<D> {
        if let Some(q) = self.child_if_exists(name) {
            return q;
        }
        let q = Quark::create(&self.0.plasma, Some(self), name);
        self.0
            .children
            .borrow_mut()
            .push((name.to_string(), q.clone()));
        q
    }

    pub fn children(&self) -> Vec<Quark<D>> {
        self.0
            .children
            .borrow()
            .iter()
            .map(|(_, q)| q.clone())
            .collect()
    }

    pub fn count_children(&self) -> usize {
        self.0.children.borrow().len()
    }
}

fn clean(full: &str, sep: &str) -> String {
    let mut s = full;
    while s.starts_with(sep) {
        s = &s[sep.len()..];
    }
    while s.ends_with(sep) {
        s = &s[..s.len() - sep.len()];
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree() {
        let p: Plasma<i32> = Plasma::new();
        p.set_separator(Some("."));
        let q = p.root().child("a.b.c");
        assert_eq!(q.qualified_name(), "a.b.c");
        assert_eq!(q.parent().unwrap().name(), "b");
        assert_eq!(p.root().child("a").count_children(), 1);
        q.set_data(3);
        assert_eq!(q.data(), Some(3));
        assert_eq!(p.count_by_name("a"), 1);
        assert!(p.root().is_root());
        assert_eq!(q.to_string_point(), "a.b.c");
    }

    #[test]
    fn no_separator() {
        let p: Plasma<()> = Plasma::new();
        let q = p.root().child("a.b");
        assert_eq!(q.name(), "a.b");
        assert!(p.root().child_if_exists("a.b").is_some());
    }
}
