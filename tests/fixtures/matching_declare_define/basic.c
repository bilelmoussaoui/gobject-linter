#include "basic.h"

struct _FooBar { GObject parent_instance; };
G_DEFINE_FINAL_TYPE (FooBar, foo_bar, G_TYPE_OBJECT)
static void foo_bar_init (FooBar *self) { }
static void foo_bar_class_init (FooBarClass *klass) { }
