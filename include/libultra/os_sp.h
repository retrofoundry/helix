#ifndef _OS_SP_H_
#define _OS_SP_H_

#ifdef _LANGUAGE_C_PLUS_PLUS
extern "C" {
#endif

#include <libultra/ultratypes.h>

/* MUST stay ABI-identical to include/PR/sptask.h so ultra/rcp.rs reads OSTask_t.type / .data_ptr
   at the same host offsets. */

#define M_GFXTASK 1
#define M_AUDTASK 2

typedef struct {
    /*0x00*/ u32 type;
    /*0x04*/ u32 flags;
    /*0x08*/ u64 *ucode_boot;
    /*0x0C*/ u32 ucode_boot_size;
    /*0x10*/ u64 *ucode;
    /*0x14*/ u32 ucode_size;
    /*0x18*/ u64 *ucode_data;
    /*0x1C*/ u32 ucode_data_size;
    /*0x20*/ u64 *dram_stack;
    /*0x24*/ u32 dram_stack_size;
    /*0x28*/ u64 *output_buff;
    /*0x2C*/ u64 *output_buff_size;
    /*0x30*/ u64 *data_ptr;
    /*0x34*/ u32 data_size;
    /*0x38*/ u64 *yield_data_ptr;
    /*0x3C*/ u32 yield_data_size;
} OSTask_t;

typedef union {
    OSTask_t t;
    long long int force_structure_alignment;
} OSTask;

typedef u32 OSYieldResult;

void osSpTaskLoad(OSTask *task);
void osSpTaskStartGo(OSTask *task);
void osSpTaskYield(void);
OSYieldResult osSpTaskYielded(OSTask *task);

#ifdef _LANGUAGE_C_PLUS_PLUS
}
#endif

#endif /* !_OS_SP_H_ */
