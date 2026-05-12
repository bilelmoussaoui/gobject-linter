sources = [
  'basic.c',
  'basic.fixed.c',
  'no_match.c',
]

foreach src : sources
  static_library('g_error_init_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
