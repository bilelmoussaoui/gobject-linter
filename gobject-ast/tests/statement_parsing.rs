use std::path::PathBuf;

use gobject_ast::{
    Expression, ExpressionStmt, Parser, Statement,
    model::top_level::{TypeDefItem, TypedefTarget},
};

fn parse_fixture(name: &str) -> gobject_ast::Project {
    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);

    let mut parser = Parser::new().unwrap();
    parser.parse_file(&fixture_path).unwrap()
}

#[test]
fn test_parse_call_expressions() {
    let project = parse_fixture("call_expressions.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("call_expressions.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    let func = file
        .iter_function_definitions()
        .next()
        .expect("Should find a function");
    assert_eq!(func.name, "test_function");

    // Check we have statements parsed
    assert!(
        !func.body_statements.is_empty(),
        "Should have parsed body statements"
    );

    // Count call expressions
    let mut call_count = 0;
    for stmt in &func.body_statements {
        if let Statement::Expression(ExpressionStmt {
            expr: Expression::Call(_),
            ..
        }) = stmt
        {
            call_count += 1;
        }
    }

    // We should find at least the function calls (not counting the variable
    // declaration)
    assert!(
        call_count >= 2,
        "Should find at least 2 call expressions (g_task_set_source_tag, g_object_unref), found {}",
        call_count
    );
}

#[test]
fn test_parse_assignments() {
    let project = parse_fixture("assignments.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("assignments.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");
    let func = file
        .iter_function_definitions()
        .next()
        .expect("Should find a function");

    // Count assignments
    let mut assignment_count = 0;
    for stmt in &func.body_statements {
        if let Statement::Expression(ExpressionStmt {
            expr: Expression::Assignment(_),
            ..
        }) = stmt
        {
            assignment_count += 1;
        }
    }

    assert!(
        assignment_count >= 1,
        "Should find at least 1 assignment expression, found {}",
        assignment_count
    );
}

#[test]
fn test_parse_return_statement() {
    let project = parse_fixture("return_statement.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("return_statement.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");
    let func = file
        .iter_function_definitions()
        .next()
        .expect("Should find a function");

    // Should have a return statement
    assert!(!func.body_statements.is_empty(), "Should have statements");

    let has_return = func
        .body_statements
        .iter()
        .any(|stmt| matches!(stmt, Statement::Return(_)));

    assert!(has_return, "Should find return statement");
}

#[test]
fn test_parse_goto_statement() {
    let project = parse_fixture("goto_statement.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("goto_statement.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    let func = file
        .iter_function_definitions()
        .next()
        .expect("Should find a function");

    let has_goto = func.body_statements.iter().any(|s| {
        let mut found = false;
        s.walk(&mut |stmt| {
            if matches!(stmt, Statement::Goto(_)) {
                found = true;
            }
        });
        found
    });

    assert!(has_goto, "Should find goto statement");
}

#[test]
fn test_anonymous_union_field_types_collected() {
    let project = parse_fixture("anonymous_union.h");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("anonymous_union.h");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    let xdp_rule = file.iter_all_items().find_map(|item| match item {
        gobject_ast::model::top_level::TopLevelItem::TypeDefinition(
            td @ TypeDefItem::Typedef { name, .. },
        ) if name == "XdpUsbRule" => Some(td),
        _ => None,
    });

    let TypeDefItem::Typedef { struct_fields, .. } =
        xdp_rule.expect("XdpUsbRule typedef not found")
    else {
        panic!("XdpUsbRule should be a Typedef");
    };

    assert_eq!(
        struct_fields.len(),
        2,
        "XdpUsbRule should have 2 top-level fields"
    );

    let rule_type = &struct_fields[0];
    assert_eq!(rule_type.field_name.as_deref(), Some("rule_type"));
    assert_eq!(rule_type.field_type.base_type, "int");
    assert!(rule_type.inner_fields.is_empty());

    // The anonymous union `union { ... } d` is stored as field `d` with
    // inner_fields.
    let union_d = &struct_fields[1];
    assert_eq!(union_d.field_name.as_deref(), Some("d"));
    assert!(
        union_d.field_type.base_type.is_empty(),
        "anonymous union has no base type"
    );

    let inner: Vec<(&str, &str)> = union_d
        .inner_fields
        .iter()
        .map(|f| {
            (
                f.field_name.as_deref().unwrap_or(""),
                f.field_type.base_type.as_str(),
            )
        })
        .collect();
    assert_eq!(
        inner,
        vec![
            ("device_class", "int"),
            ("product", "UsbProduct"),
            ("vendor", "UsbVendor")
        ],
        "anonymous union inner fields mismatch"
    );
}

#[test]
fn test_statement_order() {
    let project = parse_fixture("statement_order.c");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("statement_order.c");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");
    let func = file
        .iter_function_definitions()
        .next()
        .expect("Should find a function");

    // Verify order: should have declaration/call first, then call
    assert!(
        func.body_statements.len() >= 2,
        "Should have at least 2 statements, found {}",
        func.body_statements.len()
    );

    // Second statement should be a call to g_bytes_unref
    let mut found_pattern = false;
    for i in 0..func.body_statements.len() - 1 {
        if let Statement::Expression(ExpressionStmt {
            expr: Expression::Call(call2),
            ..
        }) = &func.body_statements[i + 1]
            && call2.is_function("g_bytes_unref")
        {
            found_pattern = true;
        }
    }

    assert!(
        found_pattern,
        "Should find consecutive g_bytes_get_data and g_bytes_unref calls in order"
    );
}

#[test]
fn test_callback_typedef_parsing() {
    let project = parse_fixture("typedef_callback.h");

    let fixture_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("typedef_callback.h");

    let file = project
        .get_file(&fixture_path)
        .expect("File should be parsed");

    let typedefs: Vec<&TypeDefItem> = file
        .iter_all_items()
        .filter_map(|item| match item {
            gobject_ast::model::top_level::TopLevelItem::TypeDefinition(td) => Some(td),
            _ => None,
        })
        .collect();

    // Plain type alias: `typedef struct _MyObject MyObject`
    let plain = typedefs
        .iter()
        .find(|td| matches!(td, TypeDefItem::Typedef { name, .. } if name == "MyObject"))
        .expect("MyObject typedef not found");
    let TypeDefItem::Typedef { target, .. } = plain else {
        panic!("expected Typedef");
    };
    assert!(
        matches!(target, TypedefTarget::Type(_)),
        "MyObject should be a plain type alias"
    );
    let TypedefTarget::Type(ti) = target else {
        unreachable!()
    };
    assert_eq!(ti.base_type, "_MyObject");
    assert!(ti.is_struct);

    // `typedef void (*MyCallback)(MyObject *obj, gpointer user_data)`
    let cb = typedefs
        .iter()
        .find(|td| matches!(td, TypeDefItem::Typedef { name, .. } if name == "MyCallback"))
        .expect("MyCallback typedef not found");
    let TypeDefItem::Typedef { target, .. } = cb else {
        panic!("expected Typedef");
    };
    let TypedefTarget::Callback {
        return_type,
        parameters,
    } = target
    else {
        panic!(
            "MyCallback should be TypedefTarget::Callback, got {:?}",
            target
        );
    };
    assert_eq!(return_type.base_type, "void");
    assert_eq!(return_type.pointer_depth, 0);
    assert_eq!(parameters.len(), 2);
    assert_eq!(parameters[0].type_info.base_type, "MyObject");
    assert_eq!(parameters[1].type_info.base_type, "gpointer");

    // `typedef gboolean (*MyPredicate)(const gchar *name, guint index)`
    let pred = typedefs
        .iter()
        .find(|td| matches!(td, TypeDefItem::Typedef { name, .. } if name == "MyPredicate"))
        .expect("MyPredicate typedef not found");
    let TypeDefItem::Typedef { target, .. } = pred else {
        panic!("expected Typedef");
    };
    let TypedefTarget::Callback {
        return_type,
        parameters,
    } = target
    else {
        panic!("MyPredicate should be TypedefTarget::Callback");
    };
    assert_eq!(return_type.base_type, "gboolean");
    assert_eq!(parameters.len(), 2);

    // `typedef const gchar *(*MyGetNameFunc)(MyObject *obj)`
    let getter = typedefs
        .iter()
        .find(|td| matches!(td, TypeDefItem::Typedef { name, .. } if name == "MyGetNameFunc"))
        .expect("MyGetNameFunc typedef not found");
    let TypeDefItem::Typedef { target, .. } = getter else {
        panic!("expected Typedef");
    };
    let TypedefTarget::Callback {
        return_type,
        parameters,
    } = target
    else {
        panic!("MyGetNameFunc should be TypedefTarget::Callback");
    };
    assert_eq!(return_type.base_type, "gchar");
    assert_eq!(return_type.pointer_depth, 1);
    assert!(return_type.is_const);
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0].type_info.base_type, "MyObject");
}
