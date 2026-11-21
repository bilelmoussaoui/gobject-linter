sources = [
  'basic.c',
  'basic.fixed.c',
  'date_time.c',
  'glib_version.c',
  'issue_23.c',
  'issue_23.fixed.c',
  'issue_24.c',
  'issue_24.fixed.c',
  'strv.c',
  'take.c',
]

# These use setters introduced in GLib 2.90
if glib_dep.version().version_compare('>=2.90')
  sources += [
    'date_time.fixed.c',
    'strv.fixed.c',
    'take.fixed.c',
  ]
endif

foreach src : sources
  static_library('use_g_set_str_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
