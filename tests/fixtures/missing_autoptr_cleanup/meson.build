sources = [
  'has_cleanup.c',
  'missing_cleanup.c',
  'old_style_with_cleanup.c',
  'with_code_variant.c',
  'with_export_attr.c',
]

foreach src : sources
  static_library('missing_autoptr_cleanup_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
