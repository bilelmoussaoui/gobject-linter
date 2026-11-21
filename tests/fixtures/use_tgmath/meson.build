sources = [
  'basic.c',
  'basic.fixed.c',
  'no_include.c',
  'tgmath.c',
]

foreach src : sources
  static_library('use_tgmath_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
