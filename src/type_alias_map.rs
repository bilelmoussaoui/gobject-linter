use std::collections::{HashMap, HashSet};

use gobject_ast::Project;

/// Bidirectional mapping between typedef names and struct/union tags, plus
/// GObject-synthesised aliases (`Foo` ↔ `_Foo`).
pub struct TypeAliasMap {
    typedef_to_tag: HashMap<String, String>,
    tag_to_typedef: HashMap<String, String>,
}

impl TypeAliasMap {
    pub fn build(project: &Project) -> Self {
        let mut typedef_to_tag: HashMap<String, String> = HashMap::new();
        let mut tag_to_typedef: HashMap<String, String> = HashMap::new();

        for (_path, file) in project.files.iter().map(|(p, f)| (p.as_path(), f)) {
            for (name, target) in file.iter_typedef_pairs() {
                typedef_to_tag.insert(name.to_owned(), target.base_type.clone());
                if target.is_struct || target.is_union {
                    tag_to_typedef.insert(target.base_type.clone(), name.to_owned());
                }
            }
            // G_DECLARE_FINAL_TYPE etc. expand to `typedef struct _Foo Foo` at
            // compile time.
            for gt in file.iter_all_gobject_types() {
                let tag = format!("_{}", gt.type_name);
                typedef_to_tag
                    .entry(gt.type_name.clone())
                    .or_insert_with(|| tag.clone());
                tag_to_typedef
                    .entry(tag)
                    .or_insert_with(|| gt.type_name.clone());
            }
        }

        Self {
            typedef_to_tag,
            tag_to_typedef,
        }
    }

    /// Resolve a typedef name to its underlying struct tag, or return the name
    /// unchanged if it is already a tag or has no known alias.
    pub fn canonical<'a>(&'a self, name: &'a str) -> &'a str {
        self.typedef_to_tag
            .get(name)
            .map(|s| s.as_str())
            .unwrap_or(name)
    }

    /// Return the typedef alias for a struct tag, if one exists.
    pub fn typedef_for_tag<'a>(&'a self, tag: &'a str) -> Option<&'a str> {
        self.tag_to_typedef.get(tag).map(|s| s.as_str())
    }

    /// Return the struct tag for a typedef name, if one exists.
    pub fn tag_for_typedef<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        self.typedef_to_tag.get(name).map(|s| s.as_str())
    }

    /// True if `name` or any of its aliases appears in `refs`.
    pub fn is_referenced(&self, name: &str, refs: &HashSet<String>) -> bool {
        refs.contains(name)
            || self
                .typedef_to_tag
                .get(name)
                .is_some_and(|t| refs.contains(t))
            || self
                .tag_to_typedef
                .get(name)
                .is_some_and(|a| refs.contains(a))
    }

    /// True if `(struct_name, field_name)` or any typedef/tag alias of
    /// `struct_name` paired with `field_name` appears in `qualified`.
    pub fn field_is_referenced(
        &self,
        struct_name: &str,
        field_name: &str,
        qualified: &HashSet<(String, String)>,
    ) -> bool {
        let has = |s: &str| qualified.contains(&(s.to_owned(), field_name.to_owned()));
        has(struct_name)
            || self.typedef_to_tag.get(struct_name).is_some_and(|t| has(t))
            || self.tag_to_typedef.get(struct_name).is_some_and(|a| has(a))
    }

    /// Insert `(type_name, field_name)` into `qualified` under every alias of
    /// `type_name` so that lookups via either the tag or the typedef succeed.
    pub fn insert_qualified(
        &self,
        type_name: &str,
        field_name: &str,
        qualified: &mut HashSet<(String, String)>,
    ) {
        qualified.insert((type_name.to_owned(), field_name.to_owned()));
        if let Some(alias) = self.tag_to_typedef.get(type_name) {
            qualified.insert((alias.clone(), field_name.to_owned()));
        }
        if let Some(tag) = self.typedef_to_tag.get(type_name) {
            qualified.insert((tag.clone(), field_name.to_owned()));
        }
    }
}
