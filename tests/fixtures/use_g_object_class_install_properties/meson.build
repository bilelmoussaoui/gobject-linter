sources = [
  'basic.c',
  'no_typedef.c',
  'no_typedef.fixed.c',
  'signals_and_props_enum.c',
  'signals_and_props_enum.fixed.c',
  'with_enum.c',
  'with_enum.fixed.c',
  'with_variable.c',
  'with_variable.fixed.c',
]

foreach src : sources
  static_library('use_g_object_class_install_properties_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
