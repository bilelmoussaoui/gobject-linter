sources = [
  'bare_usage.c',
  'bare_usage.fixed.c',
  'multiple_same_line.c',
  'multiple_same_line.fixed.c',
  'variable_assignment.c',
]

foreach src : sources
  static_library('strcmp_explicit_comparison_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
