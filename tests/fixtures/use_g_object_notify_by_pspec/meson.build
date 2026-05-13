sources = [
  'basic.c',
  'different_object.c',
  'different_object.fixed.c',
  'two_classes.c',
  'two_classes.fixed.c',
  'with_props.c',
  'with_props.fixed.c',
  'with_override.c',
  'with_override.fixed.c',
]

foreach src : sources
  static_library('use_g_object_notify_by_pspec_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
