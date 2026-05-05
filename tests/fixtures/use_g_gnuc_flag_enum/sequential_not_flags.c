// Should not be flagged: sequential integers are not bit flags
typedef enum {
  FORBID = 0,
  ALLOW  = 1,
  IGNORE = 2
} NotifyResult;
