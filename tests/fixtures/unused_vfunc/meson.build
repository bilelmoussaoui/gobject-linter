sources = [
  'unused/unused.c',
  'used/used.c',
]

foreach src : sources
  static_library('unused_vfunc_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
