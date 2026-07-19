#include <helix/runtime.h>
#include <helix/internal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

/* Provided by the re-enabled src/game/main.c. */
extern void main_func(void);

/* 48-byte header + a <=64 MiB cart payload. Keep in sync with rom.rs MAX_CONTAINER_BYTES; this is a
 * pre-filter so a bogus size never drives a huge malloc (HLXRomLoad re-validates every field). */
#define HLX_MAX_CONTAINER_BYTES (48u + 64u * 1024u * 1024u)

/* Install a cart native-image container from $HELIX_ROM, before main_func. Optional: a guest that
 * compiles its data in (e.g. SM64 under NO_SEGMENTED_MEMORY) sets no ROM and uses the identity DMA
 * path unchanged. Fail-closed: if a ROM is requested but can't be loaded/validated, abort rather
 * than boot a guest whose first cart DMA would fail.
 *
 * Source is the $HELIX_ROM env var only (enough for OoT dev bring-up); the fuller
 * config-over-env-over-argv precedence + a wide-char Windows path (CommandLineToArgvW) are deferred
 * until a guest actually needs a cart image (see helix ROADMAP). */
static void helix_load_rom_if_configured(void) {
    const char *path = getenv("HELIX_ROM");
    if (path == NULL || path[0] == '\0') {
        return;
    }
    FILE *f = fopen(path, "rb");
    if (f == NULL) {
        fprintf(stderr, "helix: cannot open HELIX_ROM=%s\n", path);
        exit(1);
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fprintf(stderr, "helix: seek failed on %s\n", path);
        exit(1);
    }
    long n = ftell(f);
    rewind(f);
    if (n <= 0 || (unsigned long) n > HLX_MAX_CONTAINER_BYTES) {
        fprintf(stderr, "helix: bad ROM size (%ld) for %s\n", n, path);
        exit(1);
    }
    uint8_t *buf = (uint8_t *) malloc((size_t) n);
    if (buf == NULL || fread(buf, 1, (size_t) n, f) != (size_t) n) {
        fprintf(stderr, "helix: read failed on %s\n", path);
        exit(1);
    }
    fclose(f);
    HlxStatus st = HLXRomLoad(buf, (size_t) n);
    free(buf); /* HLXRomLoad copied the payload */
    if (st != HLX_OK) {
        fprintf(stderr, "helix: HLXRomLoad rejected %s (status %d)\n", path, (int) st);
        exit(1);
    }
}

static void helix_boot(void) {
    HLXRuntimeInit();             /* main thread: logging, scheduler, winit EventLoop */
    helix_load_rom_if_configured();
    main_func();                  /* osInitialize + spawn idle host thread; returns immediately */
    HLXRunEventLoop();            /* main thread: winit pump until shutdown; returns after teardown */
}

#if (defined(_WIN32) || defined(_WIN64)) && !defined(_MSC_VER)
#include <windows.h>
int WINAPI WinMain(HINSTANCE hInstance, HINSTANCE hPrevInstance, LPSTR pCmdLine, int nCmdShow) {
    (void) hInstance;
    (void) hPrevInstance;
    (void) pCmdLine;
    (void) nCmdShow;
    helix_boot();
    return 0;
}
#else
int main(int argc, char *argv[]) {
    (void) argc;
    (void) argv;
    helix_boot();
    return 0;
}
#endif
