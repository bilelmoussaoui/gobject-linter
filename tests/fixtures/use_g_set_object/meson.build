sources = [
  'basic.c',
  'basic.fixed.c',
  'field_access.c',
  'field_access.fixed.c',
  'unref.c',
  'unref.fixed.c',
]

foreach src : sources
  static_library('use_g_set_object_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
