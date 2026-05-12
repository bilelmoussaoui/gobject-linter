sources = [
  'already_glib.c',
  'pointer_locals.c',
  'pointer_locals.fixed.c',
  'prefer_glib_body.c',
  'prefer_glib_body.fixed.c',
  'prefer_glib.c',
  'prefer_glib.fixed.c',
]

foreach src : sources
  static_library('type_style_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
