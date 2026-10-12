#ifndef SOURIS_WIN_ENCODE_H
#define SOURIS_WIN_ENCODE_H

/* SourisGSR — HW encoding on Windows.
   Priority: NVENC > AMF > QuickSync > MediaFoundation (SDR fallback).
   On Linux this header only documents the mapping; the real selection lives
   in FFmpeg (h264_nvenc/hevc_nvenc/av1_nvenc, h264_amf/hevc_amf,
   h264_qsv/hevc_qsv). On Windows the D3D11 backend feeds hwupload. */

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    SOURIS_WIN_ENCODER_AUTO = 0,
    SOURIS_WIN_ENCODER_NVENC,
    SOURIS_WIN_ENCODER_AMF,
    SOURIS_WIN_ENCODER_QSV,
    SOURIS_WIN_ENCODER_MF,
} souris_win_encoder;

/* FFmpeg encoder name for a given codec ("h264"/"hevc"/"av1"). */
const char* souris_win_ffmpeg_encoder_name(souris_win_encoder enc, const char *codec);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_WIN_ENCODE_H */
