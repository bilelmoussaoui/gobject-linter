#include "gettype.h"

G_DEFINE_TYPE (GtkMagic, gtk_magic, G_TYPE_OBJECT)

static void
gtk_magic_class_init (GtkMagicClass *class)
{
}

static void
gtk_magic_init (GtkMagic *self)
{
}

GObject *
gtk_magic_new (void)
{
  return g_object_new (GTK_TYPE_MAGIC, NULL);
}
