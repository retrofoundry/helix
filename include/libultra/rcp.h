#ifndef HELIX_LIBULTRA_RCP_H
#define HELIX_LIBULTRA_RCP_H

// RCP constants the helix shims need — helix's canonical, guest-agnostic subset of PR/rcp.h.
// Values are fixed N64 hardware constants (identical across all decomp versions).

// VI DAC source clocks (Hz), by TV family — used to realize the AI DAC frequency.
#define VI_NTSC_CLOCK 48681812 /* 48.681812 MHz */
#define VI_PAL_CLOCK  49656530 /* 49.656530 MHz */
#define VI_MPAL_CLOCK 48628316 /* 48.628316 MHz */

// AI status register bits (osAiGetStatus maps helix's internal bits onto these).
#define AI_STATUS_FIFO_FULL 0x80000000 /* bit 31: FIFO full */
#define AI_STATUS_DMA_BUSY  0x40000000 /* bit 30: DMA busy  */

#endif
