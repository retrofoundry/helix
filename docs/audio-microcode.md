# Audio microcode

reality64 runs a game's audio command lists itself instead of emulating the RSP. Each handler is
written for one microcode revision and reproduces it exactly: the same output samples, the same
state in memory, the same DMEM contents after every task.

| Selector | Microcode | Data section |
| --- | --- | --- |
| `R64_RSP_AUDIO_ACMD` (5) | classic `aspMain`, Super Mario 64 (US) | 704 bytes, SHA-256 `79bb2a3e8555019bc4b229f0c218f1e329ca5711bfa55c7c7f992facf2e12c83` |

## What the port provides

Register the microcode's text address with `R64RspRegister`, as for graphics.

Link the real `aspMain` data section and point `OSTask.t.ucode_data` at it, with
`ucode_data_size` set to its length. The handler reads its tables from there, the way the RSP
loads them at the start of each task, so reality64 itself contains no microcode. Generate the data
at build time from the game's own sources or ROM; don't commit it. A missing or different data
section aborts with a message naming the revision reality64 expected.

Commands are reality64's native `Acmd` packets: two 64-bit words, with full host pointers wherever
the console command carries an RDRAM address.

Keep data in the representation the game's CPU code already uses: PCM, codebooks, loop history
and voice state as host-order 16-bit values, compressed ADPCM bytes as they come from the ROM.

Keep every audio buffer at the same alignment modulo 8 it had on the console. Like the RSP's
DMA, the handler rounds addresses down to a multiple of 8 and lengths up, so allocations must
also cover those rounded extents.

Tasks run synchronously on the calling guest thread and keep their state between tasks. A loop
or state buffer the game hands to one task must stay alive while later tasks can use it.

## How it was checked

The handlers were checked against the microcode itself, running on a real N64 and in an
interpreter that matches the console byte for byte. Synthetic tasks cover every command's
boundaries; real command lists captured from the game cover ordinary play. Byte-level operations
at odd addresses on 16-bit data are the one place the native representation can differ from the
console, and no game we know of issues them.
