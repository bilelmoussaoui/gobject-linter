use std::path::Path;

use gobject_ast::{
    Parser,
    model::{
        DeclareKind, DefineKind, GObjectTypeKind, GType, ParamFlag, Parameter, Project,
        PropertyType, SignalFlag, TypeDefItem,
    },
};

fn parse_fixture(fixture_name: &str) -> Project {
    let fixture_path = Path::new("tests/fixtures/gobject").join(fixture_name);
    let mut parser = Parser::new().expect("Failed to create parser");
    parser.parse_file(&fixture_path).expect("Failed to parse")
}

#[test]
fn test_g_declare_final_type() {
    let project = parse_fixture("declare_final.h");
    let file = project.files.values().next().expect("No files parsed");

    let gobject_types = file.iter_all_gobject_types().collect::<Vec<_>>();
    assert_eq!(gobject_types.len(), 1);
    let gobj = gobject_types[0];

    assert_eq!(gobj.type_name, "MyWidget");
    assert_eq!(
        gobj.type_macro,
        Some(GType::Identifier("MY_TYPE_WIDGET".to_owned()))
    );

    match &gobj.kind {
        GObjectTypeKind::Declare {
            kind,
            module_prefix,
            type_prefix,
        } => {
            assert_eq!(*kind, DeclareKind::Final);
            assert_eq!(gobj.function_prefix, "my_widget");
            assert_eq!(module_prefix, "MY");
            assert_eq!(type_prefix, "WIDGET");
            assert_eq!(gobj.parent_type.as_deref(), Some("GtkWidget"));
        }
        _ => panic!("Expected Declare(Final), got {:?}", gobj.kind),
    }
}

#[test]
fn test_g_declare_derivable_type() {
    let project = parse_fixture("declare_derivable.h");
    let file = project.files.values().next().expect("No files parsed");
    let gobjects = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(gobjects.len(), 1);
    let gobj = gobjects[0];

    assert_eq!(gobj.type_name, "MyObject");
    assert_eq!(
        gobj.type_macro,
        Some(GType::Identifier("MY_TYPE_OBJECT".to_owned()))
    );

    match &gobj.kind {
        GObjectTypeKind::Declare {
            kind,
            module_prefix,
            type_prefix,
        } => {
            assert_eq!(*kind, DeclareKind::Derivable);
            assert_eq!(gobj.function_prefix, "my_object");
            assert_eq!(module_prefix, "MY");
            assert_eq!(type_prefix, "OBJECT");
            assert_eq!(gobj.parent_type.as_deref(), Some("GObject"));
        }
        _ => panic!("Expected Declare(Derivable), got {:?}", gobj.kind),
    }
}

#[test]
fn test_g_declare_interface() {
    let project = parse_fixture("declare_interface.h");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);
    let gobj = &types[0];

    assert_eq!(gobj.type_name, "MyInterface");
    assert_eq!(
        gobj.type_macro,
        Some(GType::Identifier("MY_TYPE_INTERFACE".to_owned()))
    );

    match &gobj.kind {
        GObjectTypeKind::Declare {
            kind,
            module_prefix,
            type_prefix,
        } => {
            assert_eq!(*kind, DeclareKind::Interface);
            assert_eq!(gobj.function_prefix, "my_interface");
            assert_eq!(module_prefix, "MY");
            assert_eq!(type_prefix, "INTERFACE");
            assert_eq!(gobj.parent_type.as_deref(), Some("GObject"));
        }
        _ => panic!("Expected Declare(Interface), got {:?}", gobj.kind),
    }
}

#[test]
fn test_g_define_type() {
    let project = parse_fixture("define_type.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);
    let gobj = &types[0];

    assert_eq!(gobj.type_name, "MyWidget");
    assert_eq!(gobj.type_macro, None);

    match &gobj.kind {
        GObjectTypeKind::Define(kind) => {
            assert_eq!(*kind, DefineKind::Type);
            assert_eq!(gobj.function_prefix, "my_widget");
            assert_eq!(gobj.parent_type.as_deref(), Some("GTK_TYPE_WIDGET"));
        }
        _ => panic!("Expected Define(Type), got {:?}", gobj.kind),
    }
}

#[test]
fn test_g_define_type_with_private() {
    let project = parse_fixture("define_type_with_private.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);
    let gobj = &types[0];

    assert_eq!(gobj.type_name, "MyObject");

    match &gobj.kind {
        GObjectTypeKind::Define(kind) => {
            assert_eq!(*kind, DefineKind::TypeWithPrivate);
            assert_eq!(gobj.function_prefix, "my_object");
            assert_eq!(gobj.parent_type.as_deref(), Some("G_TYPE_OBJECT"));
        }
        _ => panic!("Expected Define(TypeWithPrivate), got {:?}", gobj.kind),
    }
}

#[test]
fn test_class_struct_with_vfuncs() {
    let project = parse_fixture("class_with_vfuncs.h");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);
    let gobj = &types[0];
    assert_eq!(gobj.type_name, "MyObject");

    let cs = file
        .find_class_struct_for(gobj)
        .expect("No class struct parsed");
    let TypeDefItem::Struct { name, vfuncs, .. } = cs else {
        panic!("Expected Struct variant");
    };

    assert_eq!(name, "_MyObjectClass");
    assert!(
        vfuncs.len() >= 2,
        "Expected at least 2 vfuncs, got {}",
        vfuncs.len()
    );

    let vfunc_names: Vec<_> = vfuncs.iter().map(|v| &v.name).collect();
    assert!(
        vfunc_names.contains(&&"do_something".to_string()),
        "Missing vfunc 'do_something'"
    );
    assert!(
        vfunc_names.contains(&&"get_value".to_string()),
        "Missing vfunc 'get_value'"
    );
}

#[test]
fn test_multiple_gobject_types_in_one_file() {
    let project = parse_fixture("multiple_types.h");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert!(
        types.len() >= 2,
        "Expected at least 2 GObject types, got {}",
        types.len()
    );

    let type_names: Vec<_> = types.iter().map(|g| &g.type_name).collect();
    assert!(type_names.contains(&&"MyWidget".to_string()));
    assert!(type_names.contains(&&"MyInterface".to_string()));
}

#[test]
fn test_vfunc_parameters_and_return_types() {
    let project = parse_fixture("class_with_vfuncs.h");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    let gobj = &types[0];
    let cs = file
        .find_class_struct_for(gobj)
        .expect("No class struct parsed");
    let TypeDefItem::Struct { vfuncs, .. } = cs else {
        panic!("Expected Struct variant");
    };

    // Find the do_something vfunc
    let do_something = vfuncs
        .iter()
        .find(|v| v.name == "do_something")
        .expect("Missing do_something vfunc");

    // Check return type
    assert_eq!(do_something.return_type.base_type, "void");

    // Check parameters
    assert_eq!(do_something.parameters.len(), 2);
    let Parameter::Regular {
        type_info: ds_p0,
        name: ds_n0,
        ..
    } = &do_something.parameters[0]
    else {
        panic!("expected Regular")
    };
    let Parameter::Regular {
        type_info: ds_p1,
        name: ds_n1,
        ..
    } = &do_something.parameters[1]
    else {
        panic!("expected Regular")
    };
    assert_eq!(ds_p0.base_type, "MyObject");
    assert_eq!(ds_p0.pointer_depth, 1);
    assert_eq!(ds_n0.as_deref(), Some("self"));
    assert_eq!(ds_p1.base_type, "int");
    assert_eq!(ds_n1.as_deref(), Some("value"));

    // Find the get_value vfunc
    let get_value = vfuncs
        .iter()
        .find(|v| v.name == "get_value")
        .expect("Missing get_value vfunc");

    // Check return type
    assert_eq!(get_value.return_type.base_type, "int");

    // Check parameters
    assert_eq!(get_value.parameters.len(), 1);
    let Parameter::Regular {
        type_info: gv_p0, ..
    } = &get_value.parameters[0]
    else {
        panic!("expected Regular")
    };
    assert_eq!(gv_p0.base_type, "MyObject");
    assert_eq!(gv_p0.pointer_depth, 1);
}

#[test]
fn test_property_extraction() {
    let project = parse_fixture("properties.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    // Check that we parse the GObject type definition
    assert_eq!(types.len(), 1);
    let gobject_type = &types[0];

    assert_eq!(
        gobject_type.class_init_function_name(),
        "my_object_class_init"
    );

    let properties = &gobject_type.properties;

    // Should have extracted 2 properties: name and value
    assert!(
        properties.len() >= 2,
        "Expected at least 2 properties, got {}",
        properties.len()
    );

    // Find the "name" property
    let name_prop = properties.iter().find(|a| a.property().name == "name");
    assert!(name_prop.is_some(), "Property 'name' not found");
    let name_prop = name_prop.unwrap().property();

    assert!(matches!(name_prop.property_type, PropertyType::String));
    assert_eq!(name_prop.nick, Some("Name".to_string()));
    assert_eq!(name_prop.blurb, Some("The object name".to_string()));
    assert!(name_prop.flags.contains(&ParamFlag::ReadWrite));

    // Find the "value" property
    let value_prop = properties.iter().find(|a| a.property().name == "value");
    assert!(value_prop.is_some(), "Property 'value' not found");
    let value_prop = value_prop.unwrap().property();

    match &value_prop.property_type {
        PropertyType::Int { min, max, default } => {
            assert_eq!(*min, 0);
            assert_eq!(*max, 100);
            assert_eq!(*default, 0);
        }
        _ => panic!("Expected Int property type"),
    }
    assert_eq!(value_prop.nick, Some("Value".to_string()));
    assert!(value_prop.flags.contains(&ParamFlag::ReadWrite));
}

#[test]
fn test_property_installation() {
    let project = parse_fixture("properties.c");
    let file = project.files.values().next().expect("No files parsed");

    // Find class_init function
    let class_init = file
        .iter_function_definitions()
        .find(|f| f.name == "my_object_class_init")
        .expect("No class_init found");

    // Check that it calls g_param_spec_* functions
    let param_spec_calls = class_init.find_calls(&["g_param_spec_string", "g_param_spec_int"]);
    assert!(
        param_spec_calls.len() >= 2,
        "Expected at least 2 g_param_spec calls, got {}",
        param_spec_calls.len()
    );

    // Check that it calls g_object_class_install_properties
    let install_calls = class_init.find_calls(&["g_object_class_install_properties"]);
    assert!(
        !install_calls.is_empty(),
        "Expected g_object_class_install_properties call"
    );
}

#[test]
fn test_signals_enum() {
    let project = parse_fixture("signals.c");
    let file = project.files.values().next().expect("No files parsed");

    // Check that we have the signals enum
    let signal_enum = file
        .iter_all_enums()
        .find(|e| e.values.iter().any(|v| v.name == "SIGNAL_CHANGED"));
    assert!(signal_enum.is_some(), "Signal enum not found");

    let signal_enum = signal_enum.unwrap();
    assert!(
        signal_enum
            .values
            .iter()
            .any(|v| v.name == "SIGNAL_CHANGED")
    );
    assert!(
        signal_enum
            .values
            .iter()
            .any(|v| v.name == "SIGNAL_ACTIVATED")
    );
    assert!(signal_enum.values.iter().any(|v| v.name == "N_SIGNALS"));
}

#[test]
fn test_signal_creation() {
    let project = parse_fixture("signals.c");
    let file = project.files.values().next().expect("No files parsed");

    let class_init = file
        .iter_function_definitions()
        .find(|f| f.name == "my_object_class_init")
        .expect("No class_init found");

    // Check that it calls g_signal_new
    let signal_new_calls = class_init.find_calls(&["g_signal_new"]);
    assert!(
        signal_new_calls.len() >= 2,
        "Expected at least 2 g_signal_new calls, got {}",
        signal_new_calls.len()
    );
}

#[test]
fn test_signal_extraction() {
    let project = parse_fixture("signals.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    // Get the GObject type
    let gobject_type = &types[0];
    assert_eq!(gobject_type.type_name, "MyObject");

    // Find the class_init function
    let class_init_name = gobject_type.class_init_function_name();
    let class_init = file
        .iter_function_definitions()
        .find(|f| f.name == class_init_name)
        .expect("No class_init found");

    // Extract signals using the helper
    let signals = gobject_type.extract_signals(class_init);

    assert_eq!(signals.len(), 2, "Expected 2 signals");

    // Check first signal
    let changed = &signals[0];
    assert_eq!(changed.name, "changed");
    assert!(changed.itype.is_some());
    assert_eq!(changed.flags.len(), 1);
    assert_eq!(changed.flags[0], SignalFlag::RunLast);
    assert!(matches!(changed.return_type, Some(GType::None)));
    assert_eq!(changed.n_params, Some(0));
    assert_eq!(changed.param_types.len(), 0);

    // Check second signal
    let activated = &signals[1];
    assert_eq!(activated.name, "activated");
    assert_eq!(activated.flags.len(), 1);
    assert!(activated.flags.contains(&SignalFlag::RunFirst));
    assert!(matches!(activated.return_type, Some(GType::None)));
    assert_eq!(activated.n_params, Some(1));
    assert_eq!(
        activated.param_types,
        vec![GType::Identifier("G_TYPE_INT".to_string())]
    );
}

#[test]
fn test_interface_implementation() {
    let project = parse_fixture("interface_impl.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    // Should have a G_DEFINE_TYPE_WITH_CODE macro
    assert_eq!(types.len(), 1);
    let gobj = &types[0];

    // Check that it detected the interface implementation
    assert_eq!(
        gobj.interfaces.len(),
        1,
        "Expected 1 interface implementation"
    );
    assert_eq!(
        gobj.interfaces[0].interface_type,
        GType::Identifier("MY_TYPE_INTERFACE".to_owned())
    );
    assert_eq!(
        gobj.interfaces[0].init_function.as_deref(),
        Some("my_interface_init")
    );

    // Should have the interface init function (definition)
    let has_iface_init_def = file
        .iter_function_definitions()
        .any(|f| f.name == "my_interface_init");

    assert!(has_iface_init_def, "No interface init definition found");
}

#[test]
fn test_interface_impl_multiple() {
    let project = parse_fixture("multi_interface.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);
    let gobj = &types[0];

    // Check type kind
    assert!(matches!(
        gobj.kind,
        GObjectTypeKind::Define(DefineKind::TypeWithCode)
    ));

    // Check that it has private data
    assert!(
        gobj.has_private,
        "Expected has_private to be true with G_ADD_PRIVATE"
    );

    // Check multiple interfaces
    assert_eq!(
        gobj.interfaces.len(),
        2,
        "Expected 2 interface implementations"
    );

    assert_eq!(
        gobj.interfaces[0].interface_type,
        GType::Identifier("GTK_TYPE_EDITABLE".to_owned())
    );
    assert_eq!(
        gobj.interfaces[0].init_function.as_deref(),
        Some("my_editable_init")
    );

    assert_eq!(
        gobj.interfaces[1].interface_type,
        GType::Identifier("GTK_TYPE_SCROLLABLE".to_owned())
    );
    assert_eq!(
        gobj.interfaces[1].init_function.as_deref(),
        Some("my_scrollable_init")
    );
}

#[test]
fn test_boxed_type() {
    let project = parse_fixture("boxed_types.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    // Check that we found the boxed type definition
    // G_DEFINE_BOXED_TYPE should be parsed
    assert!(!types.is_empty(), "No GObject types found");

    // Should have copy and free functions
    assert!(
        file.iter_all_function_names()
            .any(|name| name == "my_struct_copy")
    );
    assert!(
        file.iter_all_function_names()
            .any(|name| name == "my_struct_free")
    );
}

#[test]
fn test_define_quark() {
    let project = parse_fixture("define_quark.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);

    let gobj = &types[0];
    assert_eq!(gobj.function_prefix, "my_error");

    let GObjectTypeKind::DefineQuark {
        quark_name,
        func_prefix,
    } = &gobj.kind
    else {
        panic!("Expected DefineQuark kind, got {:?}", gobj.kind);
    };
    assert_eq!(quark_name, "my-error");
    assert_eq!(func_prefix, "my_error");

    assert_eq!(
        gobj.kind.quark_function_name().as_deref(),
        Some("my_error_quark")
    );
}

#[test]
fn test_define_enum_type() {
    let project = parse_fixture("define_enum.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 2);

    // G_DEFINE_ENUM_TYPE
    let enum_type = &types[0];
    assert_eq!(enum_type.type_name, "GtkOrientation");
    assert_eq!(enum_type.function_prefix, "gtk_orientation");
    assert!(enum_type.parent_type.is_none());
    assert!(enum_type.class_struct_name().is_none());

    let GObjectTypeKind::DefineEnum { values } = &enum_type.kind else {
        panic!("Expected DefineEnum kind, got {:?}", enum_type.kind);
    };
    assert_eq!(values.len(), 2);
    assert_eq!(values[0].name, "GTK_ORIENTATION_HORIZONTAL");
    assert_eq!(values[0].nick, "horizontal");
    assert_eq!(values[1].name, "GTK_ORIENTATION_VERTICAL");
    assert_eq!(values[1].nick, "vertical");

    // G_DEFINE_FLAGS_TYPE
    let flags_type = &types[1];
    assert_eq!(flags_type.type_name, "GSettingsBindFlags");
    assert_eq!(flags_type.function_prefix, "g_settings_bind_flags");
    assert!(flags_type.parent_type.is_none());
    assert!(flags_type.class_struct_name().is_none());

    let GObjectTypeKind::DefineFlags { values } = &flags_type.kind else {
        panic!("Expected DefineFlags kind, got {:?}", flags_type.kind);
    };
    assert_eq!(values.len(), 6);
    assert_eq!(values[0].name, "G_SETTINGS_BIND_DEFAULT");
    assert_eq!(values[0].nick, "default");
    assert_eq!(values[5].name, "G_SETTINGS_BIND_INVERT_BOOLEAN");
    assert_eq!(values[5].nick, "invert-boolean");
}

#[test]
fn test_gtk_doc_comments() {
    let project = parse_fixture("annotations.h");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    // Should have the declared type
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].type_name, "MyObject");

    // Should have all the documented functions
    assert!(
        file.iter_all_function_names()
            .any(|name| name == "my_object_new")
    );
    assert!(
        file.iter_all_function_names()
            .any(|name| name == "my_object_set_name")
    );
    assert!(
        file.iter_all_function_names()
            .any(|name| name == "my_object_get_children")
    );
    assert!(
        file.iter_all_function_names()
            .any(|name| name == "my_object_process")
    );
}

#[test]
fn test_custom_param_spec() {
    let project = parse_fixture("custom_param_spec.c");
    let file = project.files.values().next().expect("No files parsed");
    let types = file.iter_all_gobject_types().collect::<Vec<_>>();

    assert_eq!(types.len(), 1);
    let gobject_type = &types[0];

    let properties = &gobject_type.properties;

    // Should have extracted the custom color property
    assert_eq!(properties.len(), 1, "Expected 1 property");

    let color_prop = properties[0].property();
    assert_eq!(color_prop.name, "color");
    assert_eq!(color_prop.nick, Some("Color".to_string()));
    assert_eq!(color_prop.blurb, Some("The object color".to_string()));

    // Custom param specs should be captured as Unknown
    match &color_prop.property_type {
        PropertyType::Unknown { spec_function } => {
            assert_eq!(spec_function, "cogl_param_spec_color");
        }
        _ => panic!(
            "Expected Unknown property type, got {:?}",
            color_prop.property_type
        ),
    }

    assert!(color_prop.flags.contains(&ParamFlag::ReadWrite));
    assert!(color_prop.flags.contains(&ParamFlag::StaticStrings));
}
