typedef struct _MyObject MyObject;

typedef void (*MyCallback)(MyObject *obj, gpointer user_data);

typedef gboolean (*MyPredicate)(const gchar *name, guint index);

typedef const gchar *(*MyGetNameFunc)(MyObject *obj);
