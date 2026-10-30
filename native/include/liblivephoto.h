#ifndef LIBLIVEPHOTO_NATIVE_H
#define LIBLIVEPHOTO_NATIVE_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
/* Core operation protocol version, not an Apple/Google ABI version. */
uint32_t llp_api_version(void);
typedef struct LLPResponse LLPResponse;
/* Inputs are immutable readable spans for this call. NULL is allowed only for
 * zero length. Response bytes use LLP1 (see interface crate/docs), owned by Rust.
 * No callback, pointer or source span is retained after this synchronous call. */
LLPResponse *llp_process(const uint8_t *request, size_t request_len,
                        const uint8_t *primary, size_t primary_len,
                        const uint8_t *movie, size_t movie_len);
/* A handle must be live, obtained from llp_process, and freed once. Do not use
 * response_data after free. No cross-allocator free/realloc is permitted. */
const uint8_t *llp_response_data(const LLPResponse *response);
size_t llp_response_len(const LLPResponse *response);
void llp_response_free(LLPResponse *response);
#ifdef __cplusplus
}
#endif
#endif
