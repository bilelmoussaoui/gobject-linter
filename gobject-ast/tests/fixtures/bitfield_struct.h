typedef struct {
  unsigned flags : 1;
  unsigned count : 4;
  unsigned padding : 27;
  int normal_field;
} MyBitStruct;
