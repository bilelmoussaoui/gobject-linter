sources = [
  'all_installed.c',
  'define_property_name.c',
  'missing_property.c',
  'multiple_classes_missing_property.c',
  'multiple_classes_same_file.c',
]

foreach src : sources
  static_library('property_enum_coverage_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
