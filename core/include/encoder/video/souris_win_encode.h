#ifndef SOURIS_WIN_ENCODE_H
#define SOURIS_WIN_ENCODE_H

/* SourisGSR — encode HW en Windows.
   Prioridad: NVENC > AMF > QuickSync > MediaFoundation (fallback SDR).
   En Linux este header solo documenta el mapping; la selección real vive
   en FFmpeg (h264_nvenc/hevc_nvenc/av1_nvenc, h264_amf/hevc_amf,
   h264_qsv/hevc_qsv). En Windows el backend D3D11 alimenta hwupload. */

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

/* Nombre del encoder FFmpeg para un codec dado ("h264"/"hevc"/"av1"). */
const char* souris_win_ffmpeg_encoder_name(souris_win_encoder enc, const char *codec);

#ifdef __cplusplus
}
#endif

#endif /* SOURIS_WIN_ENCODE_H */
