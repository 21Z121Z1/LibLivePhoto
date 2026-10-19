/* External acceptance consumer: compiled against the actual delivered header.
 * Dynamic loading avoids requiring a compiler-specific Windows import library.
 * Media is immutable; an owned test directory receives only the LLP1 response.
 */
#include "liblivephoto.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#ifdef _WIN32
#include <windows.h>
#define LOAD(path) LoadLibraryA(path)
#define SYMBOL(handle, name) GetProcAddress(handle, name)
#define CLOSE(handle) FreeLibrary(handle)
#else
#include <dlfcn.h>
#define LOAD(path) dlopen(path, RTLD_NOW | RTLD_LOCAL)
#define SYMBOL(handle, name) dlsym(handle, name)
#define CLOSE(handle) dlclose(handle)
#endif
typedef uint32_t (*Version)(void);
typedef LLPResponse *(*Process)(const uint8_t *,size_t,const uint8_t *,size_t,const uint8_t *,size_t);
typedef const uint8_t *(*Bytes)(const LLPResponse *);
typedef size_t (*Length)(const LLPResponse *);
typedef void (*FreeResponse)(LLPResponse *);

static FILE *open_file(const char *path, const char *mode) {
#ifdef _WIN32
    FILE *stream = NULL;
    if (fopen_s(&stream,path,mode)!=0) return NULL;
    return stream;
#else
    return fopen(path,mode);
#endif
}

static uint8_t *read_input(const char *path, size_t *length) {
    *length = 0;
    if (strcmp(path,"-")==0) return NULL;
    FILE *input = open_file(path,"rb");
    if (!input || fseek(input,0,SEEK_END)!=0) exit(3);
    long end = ftell(input);
    if (end<0 || end>128*1024*1024 || fseek(input,0,SEEK_SET)!=0) exit(3);
    uint8_t *data = (uint8_t *)malloc((size_t)end+1);
    if (!data || fread(data,1,(size_t)end,input)!=(size_t)end || fclose(input)!=0) exit(3);
    *length = (size_t)end;
    return data;
}
int main(int argc, char **argv) {
    if (argc!=6) return 2;
#ifdef _WIN32
    HMODULE library = LOAD(argv[1]);
#else
    void *library = LOAD(argv[1]);
#endif
    if (!library) return 4;
    Version version = (Version)SYMBOL(library,"llp_api_version");
    Process process = (Process)SYMBOL(library,"llp_process");
    Bytes bytes = (Bytes)SYMBOL(library,"llp_response_data");
    Length length = (Length)SYMBOL(library,"llp_response_len");
    FreeResponse release = (FreeResponse)SYMBOL(library,"llp_response_free");
    if (!version || !process || !bytes || !length || !release || version()!=1) return 4;
    size_t rn,pn,mn;
    uint8_t *request=read_input(argv[2],&rn), *primary=read_input(argv[3],&pn), *movie=read_input(argv[4],&mn);
    LLPResponse *response = process(request,rn,primary,pn,movie,mn);
    if (!response) return 5;
    size_t n = length(response);
    const uint8_t *data = bytes(response);
    if (!data || n<12 || n>320*1024*1024) return 5;
    FILE *output = open_file(argv[5],"wb");
    if (!output || fwrite(data,1,n,output)!=n || fclose(output)!=0) return 3;
    release(response);
    free(request); free(primary); free(movie);
    CLOSE(library);
    return 0;
}
