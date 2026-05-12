sources = [
  'basic.c',
  'basic.fixed.c',
  'multiline.c',
  'multiline.fixed.c',
]

foreach src : sources
  static_library('g_task_source_tag_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
