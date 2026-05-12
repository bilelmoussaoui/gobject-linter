sources = [
  'basic.c',
  'basic.h',
  'mismatch.c',
  'mismatch.h',
]

foreach src : sources
  static_library('matching_declare_define_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
