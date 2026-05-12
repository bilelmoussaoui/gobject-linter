sources = [
  'already_pragma_once.h',
  'define_with_value.h',
  'traditional_guard.fixed.h',
  'traditional_guard.h',
  'with_comment.fixed.h',
  'with_comment.h',
]

foreach src : sources
  static_library('use_pragma_once_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
