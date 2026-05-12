sources = [
  'functions.c',
  'functions.h',
  'type_since.c',
  'type_since.h',
]

foreach src : sources
  static_library('gi_missing_since_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
