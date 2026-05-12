sources = [
  'all_handled.c',
  'all_handled.fixed.c',
  'casted_auto_fixable.c',
  'casted_auto_fixable.fixed.c',
  'casted_needs_implementation.c',
  'delegation_inner_switch.c',
  'exhaustive_with_cast.c',
  'exhaustive_with_cast.fixed.c',
  'missing_in_getter.c',
  'multiple_missing_cases.c',
  'multiple_missing_cases.fixed.c',
  'read_only_properties.c',
  'read_only_properties.fixed.c',
  'write_only_properties.c',
  'write_only_properties.fixed.c',
]

foreach src : sources
  static_library('property_switch_exhaustiveness_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
