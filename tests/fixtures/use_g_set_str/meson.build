sources = [
  'basic.c',
  'basic.fixed.c',
  'issue_23.c',
  'issue_23.fixed.c',
  'issue_24.c',
  'issue_24.fixed.c',
]

foreach src : sources
  static_library('use_g_set_str_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
