Detects user-visible string literals that are not wrapped in a translation
macro such as `_()`, `N_()`, `gettext()`, `g_dgettext()`, or
`g_dpgettext2()`.

## Why?

- **Internationalization**: user-visible strings in widget constructors and
  setters are displayed directly in the UI. Untranslated literals make the
  application appear only partially localized
- **Consistency**: if most strings are translated, a forgotten one stands out
  to translators and causes incomplete `.po` files
- **Early detection**: catching untranslated strings at lint time is cheaper
  than discovering them through translator bug reports

## What is checked

- **GTK widget functions**: `gtk_label_new`, `gtk_button_new_with_label`,
  `gtk_window_set_title`, `gtk_entry_set_placeholder_text`,
  `gtk_message_dialog_new`, and other functions that accept user-visible text
- **Libadwaita widget functions**: `adw_toast_new`, `adw_status_page_set_title`,
  `adw_message_dialog_new`, `adw_preferences_row_set_title`,
  `adw_action_row_set_subtitle`, and similar
- Strings that are empty, contain no alphabetic characters, or are simple format
  specifiers (e.g. `"%s"`) are skipped

## Examples

**Bad** (bare string literal):
```c
gtk_window_set_title (GTK_WINDOW (window), "Preferences");
gtk_button_new_with_label ("Apply");
```

**Good** (wrapped in translation macro):
```c
gtk_window_set_title (GTK_WINDOW (window), _("Preferences"));
gtk_button_new_with_label (_("Apply"));
```

## Notes

This rule is most useful for applications and libraries that ship translations.
Libraries that intentionally use untranslated English strings (e.g. with
`g_param_spec_null_nick_blurb` setting nick/blurb to NULL) can leave this rule
disabled.
