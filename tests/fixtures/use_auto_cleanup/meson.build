sources = [
  'already_autoptr.c',
  'autofree_already_autofree.c',
  'autofree_already_autofree.fixed.c',
  'autofree_basic.c',
  'autofree_struct.c',
  'autolist_primitive_list_g_free.c',
  'autolist_use_g_autolist.c',
  'basic.c',
  'error_basic.c',
  'goto_basic.c',
  'plain_struct.c',
]

foreach src : sources
  static_library('use_auto_cleanup_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
