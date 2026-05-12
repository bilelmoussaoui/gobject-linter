sources = [
  'iface_init.c',
  'unused_enum_value.c',
  'unused_field.c',
  'unused_static.c',
  'used_enum_values.c',
  'used_fields.c',
  'used_in_g_new0.c',
  'used_in_macro_named_array.c',
  'used_static.c',
]

foreach src : sources
  static_library('dead_code_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
