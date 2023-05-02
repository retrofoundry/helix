#ifndef HELIX_LIB_NETWORK_H
#define HELIX_LIB_NETWORK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void* TCPCreate(void);
void* TCPFree(void* stream);

void TCPConnect(void* stream, char* host, uint16_t port, void (*on_message_callback)(const char* data));
void TCPDisconnect(void* stream);

void TCPSendMessage(void* stream, const char* data);

#ifdef __cplusplus
}
#endif

#endif /* HELIX_LIB_NETWORK_H */
