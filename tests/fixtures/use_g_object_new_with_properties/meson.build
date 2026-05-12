sources = [
  'basic.c',
]

foreach src : sources
  static_library('use_g_object_new_with_properties_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
