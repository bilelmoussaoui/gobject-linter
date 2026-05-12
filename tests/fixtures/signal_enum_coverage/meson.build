sources = [
  'all_installed.c',
  'missing_signal.c',
]

foreach src : sources
  static_library('signal_enum_coverage_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
