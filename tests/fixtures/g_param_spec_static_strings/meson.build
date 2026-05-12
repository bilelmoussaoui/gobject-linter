sources = [
  'basic.c',
  'basic.fixed.c',
  'cast_with_static_strings.c',
  'issue_29.c',
  'issue_29.fixed.c',
]

foreach src : sources
  static_library('g_param_spec_static_strings_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
