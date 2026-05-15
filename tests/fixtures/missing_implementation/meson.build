sources = [
  'basic.c',
  'basic.h',
  'gettype.c',
  'gettype.h',
]

foreach src : sources
  static_library('missing_implementation_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
