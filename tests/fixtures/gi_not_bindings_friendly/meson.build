sources = [
  'bindings.h',
]

foreach src : sources
  static_library('gi_not_bindings_friendly_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
