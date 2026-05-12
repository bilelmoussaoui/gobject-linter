sources = [
  'basic.c',
  'basic.fixed.c',
]

foreach src : sources
  static_library('use_explicit_default_flags_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
