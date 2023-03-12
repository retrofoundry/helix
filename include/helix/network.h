#ifndef HELIX_LIB_NETWORK_H
#define HELIX_LIB_NETWORK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void HLXTCPConnect(char* host, uint16_t port, void (*callback)(const char* data));
void HLXTCPDisconnect(void);

void HLXTCPSendMessage(const char* data);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_NETWORK_H */
