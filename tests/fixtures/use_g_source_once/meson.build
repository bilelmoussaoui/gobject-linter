sources = [
  'basic.c',
  'basic.fixed.c',
  'with_cast.c',
  'with_cast.fixed.c',
]

foreach src : sources
  static_library('use_g_source_once_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
