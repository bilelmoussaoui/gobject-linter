#include <gio/gio.h>

void gdm_client_get_greeter(GObject *client,
                            GCancellable *cancellable,
                            GAsyncReadyCallback callback,
                            gpointer user_data)
{
    GTask *task;

    task = g_task_new (G_OBJECT (client),
                       cancellable,
                       callback,
                       user_data);
    g_task_set_source_tag (task, gdm_client_get_greeter);

    g_object_unref (task);
}
