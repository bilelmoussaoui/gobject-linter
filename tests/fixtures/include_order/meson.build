sources = [
  'already_correct.c',
  'basic.c',
  'basic.fixed.c',
  'before_preprocessor.c',
  'before_preprocessor.fixed.c',
  'meta-test-monitor.c',
  'meta-test-monitor.fixed.c',
  'preserve_spacing.c',
  'preserve_spacing.fixed.c',
  'with_conditionals.c',
  'with_conditionals.fixed.c',
]

foreach src : sources
  static_library('include_order_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
