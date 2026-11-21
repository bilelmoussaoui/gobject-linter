sources = [
  'basic.c',
  'basic.fixed.c',
  'negative.c',
]

foreach src : sources
  static_library('use_g_strconcat_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
