#include <glib-object.h>

#define GTK_TYPE_MAGIC            (gtk_magic_get_type ())
#define GTK_MAGIC(obj)            (G_TYPE_CHECK_INSTANCE_CAST ((obj), GTK_TYPE_MAGIC, GtkMagic))
#define GTK_MAGIC_CLASS(klass)    (G_TYPE_CHECK_CLASS_CAST ((klass), GTK_TYPE_MAGIC, GtkMagicClass))
#define GTK_IS_MAGIC(obj)         (G_TYPE_CHECK_INSTANCE_TYPE ((obj), GTK_TYPE_MAGIC))
#define GTK_IS_MAGIC_CLASS(klass) (G_TYPE_CHECK_CLASS_TYPE ((klass), GTK_TYPE_MAGIC))
#define GTK_MAGIC_GET_CLASS(obj)  (G_TYPE_INSTANCE_GET_CLASS ((obj), GTK_TYPE_MAGIC, GtkMagicClass))


typedef struct _GtkMagic         GtkMagic;
typedef struct _GtkMagicClass    GtkMagicClass;

struct _GtkMagic
{
  GObject parent_instance;
};

struct _GtkMagicClass
{
  GObjectClass parent_class;
};

GType    gtk_magic_get_type (void) G_GNUC_CONST;
GObject *gtk_magic_new      (void);

