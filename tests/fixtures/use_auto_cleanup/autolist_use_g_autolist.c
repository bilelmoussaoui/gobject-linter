static void
test_glist_cleanup (void)
{
  GList *connections;

  connections = g_dbus_interface_skeleton_get_connections (skeleton);

  for (GList *l = connections; l != NULL; l = l->next)
    {
      GDBusConnection *connection = l->data;
      g_signal_emit (skeleton, signal_id, 0, connection);
    }

  g_list_free_full (connections, g_object_unref);
}

static void
test_slist_cleanup (void)
{
  GSList *items = NULL;

  items = get_some_items ();

  for (GSList *l = items; l != NULL; l = l->next)
    {
      Item *item = l->data;
      process_item (item);
    }

  g_slist_free_full (items, g_object_unref);
}

static GList *
test_returned_list (void)
{
  GList *list = NULL;

  list = build_list ();

  g_list_free_full (list, g_object_unref);

  return list;
}

static void
test_variant_cleanup (void)
{
  GList *variants;

  variants = get_variants ();

  for (GList *l = variants; l != NULL; l = l->next)
    {
      GVariant *variant = l->data;
      process_variant (variant);
    }

  g_list_free_full (variants, g_variant_unref);
}

static void
test_already_auto (void)
{
  g_autolist(GObject) connections = NULL;

  connections = g_dbus_interface_skeleton_get_connections (skeleton);

  for (GList *l = connections; l != NULL; l = l->next)
    {
      GDBusConnection *connection = l->data;
      g_signal_emit (skeleton, signal_id, 0, connection);
    }
}

static void
test_no_cleanup (void)
{
  GList *connections;

  connections = g_dbus_interface_skeleton_get_connections (skeleton);

  for (GList *l = connections; l != NULL; l = l->next)
    {
      GDBusConnection *connection = l->data;
      g_signal_emit (skeleton, signal_id, 0, connection);
    }

  // Intentionally not freed - should not trigger
}
