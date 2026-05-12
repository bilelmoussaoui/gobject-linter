#include <gio/gio.h>

typedef struct {
  GError *error;
} MyData;

/* GError ** pointing at an existing GError* field — already has a non-NULL
 * initializer, so we must NOT insert = NULL (would produce invalid C:
 * GError **error = NULL = &d->error). */

static void
my_func (MyData *d)
{
  GError **error = &d->error;

  do_something (error);
}

/* Same with a plain GError* assigned from a field — already initialized. */

static void
my_func2 (MyData *d)
{
  GError *error = d->error;

  do_something (&error);
}

/* First use is a direct assignment  */

static void
direct_assignment (void)
{
  GError *error;

  error = g_error_new (G_IO_ERROR, G_IO_ERROR_FAILED, "oops");
  do_something (&error);
}

/* Same but with other declarations between. */

static void
assignment_after_other_decls (void)
{
  GError *error;
  int ret;
  char *name;

  error = g_error_new_literal (G_IO_ERROR, G_IO_ERROR_FAILED, "oops");
  do_something (&error);
}
