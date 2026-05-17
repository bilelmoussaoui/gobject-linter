sources = [
  'basic.c',
  'basic.fixed.c',
  'consecutive.c',
  'consecutive.fixed.c',
  'handle_id_basic.c',
  'handle_id_issue_25.c',
  'handle_id_issue_25.fixed.c',
  'handle_id_multi_stmt.c',
  'handle_id_multi_stmt.fixed.c',
  'if_statements.c',
  'if_statements.fixed.c',
  'list_basic.c',
  'list_basic.fixed.c',
  'signal_handler_basic.c',
  'signal_handler_basic.fixed.c',
  'weak_pointer_basic.c',
  'weak_pointer_basic.fixed.c',
]

foreach src : sources
  static_library('use_clear_functions_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
