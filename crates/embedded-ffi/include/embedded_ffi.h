typedef struct SeBuf {
  uint8_t* ptr;
  size_t len;
} SeBuf;

typedef struct Handle Handle;

void se_buf_free(SeBuf buf);
SeBuf se_version(void);
Handle* se_open_mem(const char* ns, const char* db);
Handle* se_open_surrealkv(const char* path, const char* ns, const char* db);
SeBuf se_query(Handle* handle, const char* surql, const uint8_t* vars_cbor, size_t vars_len);
SeBuf se_query_take(Handle* handle, const char* surql, const uint8_t* vars_cbor, size_t vars_len, size_t index);
void se_close(Handle* handle);
