#include <helix/runtime.h>

/* Provided by the re-enabled src/game/main.c. */
extern void main_func(void);

static void helix_boot(void) {
    HLXRuntimeInit();  /* main thread: logging, scheduler, winit EventLoop (window created on first pump) */
    main_func();       /* osInitialize + spawn idle host thread; returns immediately */
    HLXRunEventLoop(); /* main thread: winit pump until shutdown; returns after teardown */
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
