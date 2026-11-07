#include <reality64/runtime.h>
#include <reality64/internal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

/* Provided by the re-enabled src/game/main.c. */
extern void main_func(void);

/* Bound the allocation before reading; keep in sync with rom.rs MAX_CONTAINER_BYTES. */
#define R64_MAX_CONTAINER_BYTES (48u + 64u * 1024u * 1024u)

/* Guests with compiled-in assets need no ROM. An explicitly requested ROM must load before boot. */
static void reality64_load_rom_if_configured(void) {
    const char *path = getenv("REALITY64_ROM");
    if (path == NULL || path[0] == '\0') {
        return;
    }
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        fprintf(stderr, "reality64: cannot open REALITY64_ROM=%s\n", path);
        exit(1);
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fprintf(stderr, "reality64: seek failed on %s\n", path);
        exit(1);
    }
    long n = ftell(f);
    rewind(f);
    if (n <= 0 || (unsigned long) n > R64_MAX_CONTAINER_BYTES) {
        fprintf(stderr, "reality64: bad ROM size (%ld) for %s\n", n, path);
        exit(1);
    }
    uint8_t *buf = (uint8_t *) malloc((size_t) n);
    if (buf == NULL || fread(buf, 1, (size_t) n, f) != (size_t) n) {
        fprintf(stderr, "reality64: read failed on %s\n", path);
        exit(1);
    }
    fclose(f);
    R64Status st = R64RomLoad(buf, (size_t) n);
    free(buf);
    if (st != R64_OK) {
        fprintf(stderr, "reality64: R64RomLoad rejected %s (status %d)\n", path, (int) st);
        exit(1);
    }
}

static void reality64_boot(void) {
    R64RuntimeInit();
    reality64_load_rom_if_configured();
    main_func();
    R64RunEventLoop();
}

#if (defined(_WIN32) || defined(_WIN64)) && !defined(_MSC_VER)
#include <windows.h>
int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR pCmdLine, int nCmdShow) {
    (void) hInstance;
    (void) hPrevInstance;
    (void) pCmdLine;
    (void) nCmdShow;
    reality64_boot();
    return 0;
}
#else
int main(int argc, char *argv[]) {
    (void) argc;
    (void) argv;
    reality64_boot();
    return 0;
}
#endif
