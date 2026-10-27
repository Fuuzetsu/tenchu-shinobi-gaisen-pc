#include <dlfcn.h>
#include <stdio.h>

int main(void) {
    const char *libraries[] = {
        "libvulkan.so.1", "libGL.so.1", "libEGL.so.1", "libX11.so.6",
        "libwayland-client.so.0", "libxkbcommon.so.0", "libpulse.so.0", "libdbus-1.so.3",
    };
    for (unsigned i = 0; i < sizeof(libraries) / sizeof(libraries[0]); ++i) {
        if (!dlopen(libraries[i], RTLD_NOW | RTLD_LOCAL)) {
            fprintf(stderr, "%s: %s\n", libraries[i], dlerror());
            return 1;
        }
        printf("Loaded %s\n", libraries[i]);
    }
    return 0;
}
