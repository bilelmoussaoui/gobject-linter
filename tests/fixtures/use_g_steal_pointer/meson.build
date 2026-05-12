sources = [
  'basic.c',
  'basic.fixed.c',
  'if_condition_removal.c',
  'if_condition_removal.fixed.c',
  'no_match.c',
]

foreach src : sources
  static_library('use_g_steal_pointer_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
