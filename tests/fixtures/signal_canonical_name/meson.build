sources = [
  'correct_hyphens.c',
  'correct_hyphens.fixed.c',
  'detailed_signals.c',
  'detailed_signals.fixed.c',
  'signal_connect.c',
  'signal_connect.fixed.c',
  'signal_lookup_and_group.c',
  'signal_lookup_and_group.fixed.c',
  'signal_new.c',
  'signal_new.fixed.c',
]

foreach src : sources
  static_library('signal_canonical_name_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
