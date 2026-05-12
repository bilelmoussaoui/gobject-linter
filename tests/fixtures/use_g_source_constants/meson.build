sources = [
  'basic.c',
  'basic.fixed.c',
  'multiple_same_line.c',
  'multiple_same_line.fixed.c',
]

foreach src : sources
  static_library('use_g_source_constants_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
