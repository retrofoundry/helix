// Representative SM64 note-processing slice — exercises ADPCM decode, DMEM move,
// volume setup, gained mixing, and interleave, then saves a genuinely non-zero mix
// result. aResample/aEnvMixer are intentionally omitted: the sm64-port resample
// kernel yields all-zero output for this synthetic input, which would make the
// parity comparison vacuous. Included by both dispatch TUs.
aSegment(cmd++, 0, 0);
aClearBuffer(cmd++, 0x0000, 0x0400);
aClearBuffer(cmd++, 0x0500, 0x00a0);
aSetBuffer(cmd++, 0, 0x0000, 0x0000, 0x0090);
aLoadBuffer(cmd++, s_adpcm_src);
aLoadADPCM(cmd++, 32, s_book);
aSetLoop(cmd++, &s_loop_state);
aSetBuffer(cmd++, 0, 0x0000, 0x0100, 0x0090);
aADPCMdec(cmd++, A_INIT, s_adpcm_state);
aDMEMMove(cmd++, 0x0100, 0x0200, 0x0090);
aSetVolume(cmd++, A_VOL | A_LEFT, 0x4000, 0, 0);
aSetVolume(cmd++, A_VOL | A_RIGHT, 0x4000, 0, 0);
aSetVolume32(cmd++, A_RATE | A_LEFT, 0x7fff, 0x00010000);
aSetVolume32(cmd++, A_RATE | A_RIGHT, 0x7fff, 0x00010000);
aSetBuffer(cmd++, 0, 0x0000, 0x0000, 0x0090);
aMix(cmd++, 0, 0x7fff, 0x0100, 0x0500);
aMix(cmd++, 0, 0x4000, 0x0200, 0x0500);
aInterleave(cmd++, 0x0100, 0x0200);
aSetBuffer(cmd++, 0, 0x0000, 0x0500, 0x0090);
aSaveBuffer(cmd++, out_pcm);
