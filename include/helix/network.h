#ifndef HELIX_LIB_NETWORK_H
#define HELIX_LIB_NETWORK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void HLX_TCPConnect(char* host, uint16_t port, void (*callback)(const char* data));
void HLX_TCPDisconnect(void);

void HLX_TCPSendMessage(const char* data);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_NETWORK_H */
