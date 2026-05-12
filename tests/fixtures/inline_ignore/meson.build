sources = [
  'test.c',
  'test_wildcards.c',
]

foreach src : sources
  static_library('inline_ignore_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
