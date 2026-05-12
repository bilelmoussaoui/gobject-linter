sources = [
  'private_header.h',
  'with_export.h',
  'without_export.h',
]

foreach src : sources
  static_library('missing_export_macro_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
