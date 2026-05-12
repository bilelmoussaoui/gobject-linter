sources = [
  'checked_but_leaked.c',
  'compound_statement.c',
  'double_pointer.c',
  'early_return.c',
  'freed.c',
  'g_assert_terminates.c',
  'g_clear_error.c',
  'g_error_terminates.c',
  'goto_cleanup.c',
  'goto_leak.c',
  'g_steal_pointer.c',
  'g_task_return_error.c',
  'leak.c',
  'multiple_calls.c',
  'multiple_errors.c',
  'nested_if.c',
  'not_initialized.c',
  'propagated.c',
  'propagate_in_else.c',
  'terminate_with_error.c',
  'unused.c',
]

foreach src : sources
  static_library('g_error_leak_' + src.underscorify(),
    src,
    dependencies: fixture_deps,
    c_args: fixture_c_args,
  )
endforeach
