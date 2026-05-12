sources = [
  'basic.c',
  'basic.fixed.c',
  'i_macro.c',
]

foreach src : sources
  static_library('property_canonical_name_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
