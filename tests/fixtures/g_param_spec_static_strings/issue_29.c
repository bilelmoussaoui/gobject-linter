#include <glib-object.h>

typedef struct _Foo Foo;
G_DECLARE_FINAL_TYPE (Foo, foo, FOO, FOO, GObject)

struct _Foo {
  GObject parent_instance;
};

enum { PROP_0, PROP_A, PROP_B, PROP_C, PROP_D, PROP_E, PROP_F, PROP_G, PROP_H, PROP_I, PROP_J };

G_DEFINE_TYPE (Foo, foo, G_TYPE_OBJECT)

static void foo_init (Foo *self) { }

static void
foo_class_init (FooClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  /* --- nick=NULL, blurb=NULL --- */

  /* No static flag: should suggest G_PARAM_STATIC_NAME (not STATIC_STRINGS) */
  g_object_class_install_property (object_class, PROP_A,
    g_param_spec_string ("prop-a", NULL, NULL, NULL, G_PARAM_READWRITE));

  /* Already has STATIC_NAME: no violation */
  g_object_class_install_property (object_class, PROP_B,
    g_param_spec_string ("prop-b", NULL, NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Has STATIC_STRINGS (superset): no violation even though nick/blurb NULL */
  g_object_class_install_property (object_class, PROP_C,
    g_param_spec_string ("prop-c", NULL, NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS));

  /* --- nick=literal, blurb=NULL --- */

  /* No static flag: should suggest STATIC_NAME | STATIC_NICK */
  g_object_class_install_property (object_class, PROP_D,
    g_param_spec_string ("prop-d", "Prop D", NULL, NULL, G_PARAM_READWRITE));

  /* Has STATIC_NAME only (NICK missing): should add STATIC_NICK */
  g_object_class_install_property (object_class, PROP_E,
    g_param_spec_string ("prop-e", "Prop E", NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME));

  /* Has both STATIC_NAME | STATIC_NICK: no violation */
  g_object_class_install_property (object_class, PROP_F,
    g_param_spec_string ("prop-f", "Prop F", NULL, NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME | G_PARAM_STATIC_NICK));

  /* --- nick=NULL, blurb=literal --- */

  /* No static flag: should suggest STATIC_NAME | STATIC_BLURB */
  g_object_class_install_property (object_class, PROP_G,
    g_param_spec_string ("prop-g", NULL, "A blurb", NULL, G_PARAM_READWRITE));

  /* --- nick=literal, blurb=literal --- */

  /* No static flag: should suggest STATIC_STRINGS */
  g_object_class_install_property (object_class, PROP_H,
    g_param_spec_string ("prop-h", "Prop H", "A blurb", NULL, G_PARAM_READWRITE));

  /* Has all three individual flags: no violation */
  g_object_class_install_property (object_class, PROP_I,
    g_param_spec_string ("prop-i", "Prop I", "A blurb", NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_NAME | G_PARAM_STATIC_NICK | G_PARAM_STATIC_BLURB));

  /* Has STATIC_STRINGS: no violation */
  g_object_class_install_property (object_class, PROP_J,
    g_param_spec_string ("prop-j", "Prop J", "A blurb", NULL,
                         G_PARAM_READWRITE | G_PARAM_STATIC_STRINGS));
}
