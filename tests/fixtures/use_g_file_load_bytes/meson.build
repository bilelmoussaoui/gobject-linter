sources = [
  'basic.c',
]

foreach src : sources
  static_library('use_g_file_load_bytes_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
