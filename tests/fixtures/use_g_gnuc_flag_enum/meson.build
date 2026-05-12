sources = [
  'already_has_attribute.c',
  'already_has_attribute_with_includes.c',
  'bit_shift_flags.c',
  'bit_shift_flags.fixed.c',
  'hex_flags.c',
  'hex_flags.fixed.c',
  'not_a_flags_enum.c',
  'property_enum_not_flags.c',
  'sequential_not_flags.c',
]

foreach src : sources
  static_library('use_g_gnuc_flag_enum_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
