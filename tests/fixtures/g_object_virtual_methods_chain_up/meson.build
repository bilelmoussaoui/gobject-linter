sources = [
  'basic.c',
]

foreach src : sources
  static_library('g_object_virtual_methods_chain_up' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
