#include <gio/gio.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, MY, FOO, GObject)
struct _Foo { GObject parent_instance; GObject *child; };
G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }
static void foo_class_init (FooClass *klass) { }

static void
foo_dispose (GObject *object)
{
  Foo *self = MY_FOO (object);
  g_clear_object (&self->child);
}

// Valid: chains up using object_class variable

typedef struct _Bar Bar;
G_DECLARE_FINAL_TYPE (Bar, bar, MY, BAR, GObject)
struct _Bar { GObject parent_instance; GObject *child; };
G_DEFINE_TYPE (Bar, bar, G_TYPE_OBJECT)

static void bar_init (Bar *self) { }
static void bar_class_init (BarClass *klass) { }

static void
bar_dispose (GObject *object)
{
  Bar *self = MY_BAR (object);
  GObjectClass *object_class = G_OBJECT_CLASS (bar_parent_class);

  g_clear_object (&self->child);

  object_class->dispose (object);
}

// Valid: chains up using klass variable

typedef struct _Baz Baz;
G_DECLARE_FINAL_TYPE (Baz, baz, BAZ, BAZ, GObject)

struct _Baz {
  GObject parent_instance;
  gpointer data;
};

G_DEFINE_TYPE (Baz, baz, G_TYPE_OBJECT)

static void baz_init (Baz *self) { }
static void baz_class_init (BazClass *klass) { }

static void
baz_finalize (GObject *object)
{
  GObjectClass *klass = G_OBJECT_CLASS (baz_parent_class);

  // Some cleanup
  Baz *self = BAZ_BAZ (object);
  g_free (self->data);

  klass->finalize (object);
}

// Valid: Not a GObject virtual method - GSource has its own finalize

typedef struct {
  GSource parent;
  gchar *callback;
} CallbackSource;

static void
callback_source_finalize (GSource *source)
{
  CallbackSource *callback_source = (CallbackSource *) source;
  g_clear_pointer (&callback_source->callback, g_free);
}

// Valid: Chains up correctly (simplified from real gnome-shell code)

typedef struct _MyDevice MyDevice;
G_DECLARE_FINAL_TYPE (MyDevice, my_device, MY, DEVICE, GObject)

struct _MyDevice {
  GObject parent_instance;
  gpointer impl_state;
  gint slot_base;
};

G_DEFINE_TYPE (MyDevice, my_device, G_TYPE_OBJECT)

static void my_device_init (MyDevice *self) { }
static void my_device_class_init (MyDeviceClass *klass) { }

static void
my_device_dispose (GObject *object)
{
  MyDevice *self = MY_DEVICE (object);
  GObjectClass *object_class =
    G_OBJECT_CLASS (my_device_parent_class);

  g_clear_pointer (&self->impl_state, g_free);

  object_class->dispose (object);
}
