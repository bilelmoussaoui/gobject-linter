sources = [
  'basic.c',
  'basic.fixed.c',
]

foreach src : sources
  static_library('use_g_value_set_static_string_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
