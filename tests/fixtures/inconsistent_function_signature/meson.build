sources = [
  'mismatch.c',
  'mismatch.h',
  'no_mismatch.c',
  'no_mismatch.h',
  'param_mismatch.c',
  'param_mismatch.h',
  'static_mismatch.c',
]

foreach src : sources
  static_library('inconsistent_function_signature_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
