#ifndef HELIX_LIB_NETWORK_H
#define HELIX_LIB_NETWORK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void* HLXTCPCreate(void);
void HLXTCPConnect(void* stream, char* host, uint16_t port, void (*on_message_callback)(const char* data));
void HLXTCPDisconnect(void* stream);

void HLXTCPSendMessage(void* stream, const char* data);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_NETWORK_H */
