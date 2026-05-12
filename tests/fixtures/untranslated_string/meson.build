sources = []

if gtk4_dep.found()
  sources += ['basic.c']
endif

if adwaita_dep.found()
  sources += ['adwaita.c', 'empty_string.c']
endif

foreach src : sources
  static_library('untranslated_string_' + src.underscorify(),
    src,
    dependencies: fixture_deps + [gtk4_dep, adwaita_dep],
    c_args: fixture_c_args,
  )
endforeach
