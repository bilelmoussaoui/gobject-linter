#include <glib-object.h>

typedef struct _FooObject {
  GObject parent;
} FooObject;

typedef struct _FooObjectClass {
  GObjectClass parent_class;
} FooObjectClass;

typedef struct {
  GTypeInterface parent;
  void (*encode)(gpointer);
} FooCodecInterface;

GType foo_codec_get_type (void);
#define FOO_TYPE_CODEC (foo_codec_get_type ())

static void foo_object_codec_iface_init (FooCodecInterface *iface);
static gpointer foo_object_encode = NULL;

G_DEFINE_TYPE_EXTENDED(FooObject, foo_object, G_TYPE_OBJECT, 0,
                       G_IMPLEMENT_INTERFACE(FOO_TYPE_CODEC, foo_object_codec_iface_init))

static void
foo_object_codec_iface_init(FooCodecInterface *iface)
{
    iface->encode = foo_object_encode;
}

static void
foo_object_init(FooObject *self)
{
}

static void
foo_object_class_init(FooObjectClass *klass)
{
}
