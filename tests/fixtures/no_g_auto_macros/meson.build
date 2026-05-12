sources = [
  'has_auto_macros.c',
  'no_violations.c',
]

foreach src : sources
  static_library('no_g_auto_macros_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
