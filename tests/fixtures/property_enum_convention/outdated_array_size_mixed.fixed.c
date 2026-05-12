#include <glib-object.h>

typedef struct _Widget Widget;
G_DECLARE_FINAL_TYPE (Widget, widget, MY, WIDGET, GObject)
struct _Widget { GObject parent_instance; };
G_DEFINE_TYPE (Widget, widget, G_TYPE_OBJECT)
static void widget_init (Widget *self) { }


/* First object: Modern pattern with correct array size */
typedef enum {
  WIDGET_PROP_WIDTH = 1,
  WIDGET_PROP_HEIGHT,
  WIDGET_PROP_COLOR
} WidgetProperty;

static GParamSpec *widget_props[WIDGET_PROP_COLOR + 1] = { NULL, };

static void
widget_class_init (WidgetClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  widget_props[WIDGET_PROP_WIDTH] = g_param_spec_int ("width", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  widget_props[WIDGET_PROP_HEIGHT] = g_param_spec_int ("height", NULL, NULL, 0, 100, 0, G_PARAM_READWRITE);
  widget_props[WIDGET_PROP_COLOR] = g_param_spec_string ("color", NULL, NULL, NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (widget_props), widget_props);
}

typedef struct _Document Document;
G_DECLARE_FINAL_TYPE (Document, document, MY, DOCUMENT, GObject)
struct _Document { GObject parent_instance; };
G_DEFINE_TYPE (Document, document, G_TYPE_OBJECT)
static void document_init (Document *self) { }

/* Second object: Outdated array size - PROP_DESCRIPTION was added but array still uses PROP_TITLE + 1 */
typedef enum {
  DOC_PROP_NAME = 1,
  DOC_PROP_TITLE,
  DOC_PROP_DESCRIPTION
} DocumentProperty;

static GParamSpec *doc_props[DOC_PROP_DESCRIPTION + 1] = { NULL, };

static void
document_class_init (DocumentClass *klass)
{
  GObjectClass *object_class = G_OBJECT_CLASS (klass);

  doc_props[DOC_PROP_NAME] = g_param_spec_string ("name", NULL, NULL, NULL, G_PARAM_READWRITE);
  doc_props[DOC_PROP_TITLE] = g_param_spec_string ("title", NULL, NULL, NULL, G_PARAM_READWRITE);
  doc_props[DOC_PROP_DESCRIPTION] = g_param_spec_string ("description", NULL, NULL, NULL, G_PARAM_READWRITE);

  g_object_class_install_properties (object_class, G_N_ELEMENTS (doc_props), doc_props);
}
