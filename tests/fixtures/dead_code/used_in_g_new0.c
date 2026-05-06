#include <glib-object.h>

typedef struct {
    guint32 uncompressed_crc;
    guint32 uncompressed_size;
    guint32 compressed_size;
} FuZipFirmwareWriteItem;

static void
foo(GPtrArray *imgs)
{
    g_autofree FuZipFirmwareWriteItem *items = NULL;
    items = g_new0(FuZipFirmwareWriteItem, imgs->len);
    items[0].uncompressed_crc = 0;
    items[0].uncompressed_size = 0;
    items[0].compressed_size = 0;
    g_debug("crc=%u size=%u compressed=%u",
            items[0].uncompressed_crc,
            items[0].uncompressed_size,
            items[0].compressed_size);
}
