sources = [
  'basic.c',
  'basic.fixed.c',
  'different_variable.c',
  'different_variable.fixed.c',
  'for_loop.c',
  'issue_26.c',
  'pointer_comparison.c',
]

foreach src : sources
  static_library('unnecessary_null_check_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
