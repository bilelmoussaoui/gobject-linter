use std::{
    collections::{HashMap, HashSet},
    fs,
};

use globset::GlobSetBuilder;
use gobject_linter::ast_context::AstContext;

fn build_context(files: &[(&str, &str)]) -> (AstContext, tempfile::TempDir) {
    let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
    for (name, content) in files {
        fs::write(temp_dir.path().join(name), content).expect("write fixture");
    }
    let ignore = GlobSetBuilder::new().build().unwrap();
    let ctx = AstContext::build_with_ignore(temp_dir.path(), &ignore, None, None)
        .expect("build AstContext");
    (ctx, temp_dir)
}

#[test]
fn canonical_resolves_typedef_to_tag() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Foo Foo;\n")]);
    assert_eq!(ctx.type_aliases().canonical("Foo"), "_Foo");
}

#[test]
fn canonical_returns_unchanged_for_unknown() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef int MyInt;\n")]);
    assert_eq!(ctx.type_aliases().canonical("UnknownType"), "UnknownType");
}

#[test]
fn typedef_for_tag_and_tag_for_typedef() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Bar Bar;\n")]);
    assert_eq!(ctx.type_aliases().typedef_for_tag("_Bar"), Some("Bar"));
    assert_eq!(ctx.type_aliases().tag_for_typedef("Bar"), Some("_Bar"));
    assert_eq!(ctx.type_aliases().typedef_for_tag("NoSuch"), None);
    assert_eq!(ctx.type_aliases().tag_for_typedef("NoSuch"), None);
}

#[test]
fn is_referenced_direct() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Baz Baz;\n")]);
    let mut refs = HashSet::new();
    refs.insert("Baz".to_string());
    assert!(ctx.type_aliases().is_referenced("Baz", &refs));
}

#[test]
fn is_referenced_via_tag_lookup() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Baz Baz;\n")]);
    let mut refs = HashSet::new();
    refs.insert("Baz".to_string());
    // Looking up "_Baz" should find "Baz" via tag_to_typedef
    assert!(ctx.type_aliases().is_referenced("_Baz", &refs));
}

#[test]
fn is_referenced_via_typedef_lookup() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Baz Baz;\n")]);
    let mut refs = HashSet::new();
    refs.insert("_Baz".to_string());
    // Looking up "Baz" should find "_Baz" via typedef_to_tag
    assert!(ctx.type_aliases().is_referenced("Baz", &refs));
}

#[test]
fn is_referenced_not_found() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Baz Baz;\n")]);
    let refs = HashSet::new();
    assert!(!ctx.type_aliases().is_referenced("Baz", &refs));
}

#[test]
fn field_is_referenced_direct() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Widget Widget;\n")]);
    let mut qualified = HashMap::new();
    let mut fields = HashSet::new();
    fields.insert("x".to_string());
    qualified.insert("Widget".to_string(), fields);

    assert!(
        ctx.type_aliases()
            .field_is_referenced("Widget", "x", &qualified)
    );
    assert!(
        !ctx.type_aliases()
            .field_is_referenced("Widget", "y", &qualified)
    );
}

#[test]
fn field_is_referenced_via_tag_to_typedef() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Widget Widget;\n")]);
    let mut qualified = HashMap::new();
    let mut fields = HashSet::new();
    fields.insert("x".to_string());
    qualified.insert("Widget".to_string(), fields);

    // Looking up via tag "_Widget" should find field under typedef "Widget"
    assert!(
        ctx.type_aliases()
            .field_is_referenced("_Widget", "x", &qualified)
    );
}

#[test]
fn field_is_referenced_via_typedef_to_tag() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Widget Widget;\n")]);
    let mut qualified = HashMap::new();
    let mut fields = HashSet::new();
    fields.insert("z".to_string());
    qualified.insert("_Widget".to_string(), fields);

    // Looking up via typedef "Widget" should find field under tag "_Widget"
    assert!(
        ctx.type_aliases()
            .field_is_referenced("Widget", "z", &qualified)
    );
}

#[test]
fn insert_qualified_adds_typedef_and_tag() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Item Item;\n")]);
    let mut qualified: HashMap<String, HashSet<String>> = HashMap::new();
    ctx.type_aliases()
        .insert_qualified("Item", "value", &mut qualified);

    assert!(qualified.get("Item").unwrap().contains("value"));
    assert!(qualified.get("_Item").unwrap().contains("value"));
}

#[test]
fn insert_qualified_via_tag_adds_both() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef struct _Item Item;\n")]);
    let mut qualified: HashMap<String, HashSet<String>> = HashMap::new();
    ctx.type_aliases()
        .insert_qualified("_Item", "count", &mut qualified);

    assert!(qualified.get("_Item").unwrap().contains("count"));
    assert!(qualified.get("Item").unwrap().contains("count"));
}

#[test]
fn insert_qualified_no_alias() {
    let (ctx, _tmp) = build_context(&[("test.h", "typedef int MyInt;\n")]);
    let mut qualified: HashMap<String, HashSet<String>> = HashMap::new();
    ctx.type_aliases()
        .insert_qualified("Standalone", "field", &mut qualified);

    assert!(qualified.get("Standalone").unwrap().contains("field"));
    assert_eq!(qualified.len(), 1);
}

#[test]
fn gobject_declare_type_aliases() {
    let (ctx, _tmp) = build_context(&[(
        "test.h",
        "\
#include <glib-object.h>

G_BEGIN_DECLS

#define MY_TYPE_OBJ (my_obj_get_type ())
G_DECLARE_FINAL_TYPE (MyObj, my_obj, MY, OBJ, GObject)

G_END_DECLS
",
    )]);

    assert_eq!(ctx.type_aliases().canonical("MyObj"), "_MyObj");
    assert_eq!(ctx.type_aliases().typedef_for_tag("_MyObj"), Some("MyObj"));
}
