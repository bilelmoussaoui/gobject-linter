sources = [
  'basic.c',
  'basic.fixed.c',
  'separate_assignment.c',
  'separate_assignment.fixed.c',
]

foreach src : sources
  static_library('use_g_new_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
