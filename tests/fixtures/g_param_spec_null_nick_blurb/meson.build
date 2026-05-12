sources = [
  'basic.c',
  'basic.fixed.c',
  'with_gettext.c',
  'with_gettext.fixed.c',
]

foreach src : sources
  static_library('g_param_spec_null_nick_blurb_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach

