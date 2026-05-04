#include <stdint.h>

typedef struct {
  uint16_t id;
} UsbProduct;

typedef struct {
  uint16_t id;
} UsbVendor;

typedef struct {
  int rule_type;

  union {
    int device_class;
    UsbProduct product;
    UsbVendor vendor;
  } d;
} XdpUsbRule;
