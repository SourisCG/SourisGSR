#include "../../../include/encoder/video/souris_win_encode.h"
#include <string.h>
#include <stdbool.h>

const char* souris_win_ffmpeg_encoder_name(souris_win_encoder enc, const char *codec) {
    const bool hevc = codec && strcmp(codec, "hevc") == 0;
    const bool av1 = codec && strcmp(codec, "av1") == 0;
    switch(enc) {
        case SOURIS_WIN_ENCODER_NVENC:
            if(av1) return "av1_nvenc";
            if(hevc) return "hevc_nvenc";
            return "h264_nvenc";
        case SOURIS_WIN_ENCODER_AMF:
            if(hevc) return "hevc_amf";
            return "h264_amf";
        case SOURIS_WIN_ENCODER_QSV:
            if(hevc) return "hevc_qsv";
            if(av1) return "av1_qsv";
            return "h264_qsv";
        case SOURIS_WIN_ENCODER_MF:
            return "h264_mf";
        case SOURIS_WIN_ENCODER_AUTO:
        default:
            return NULL;
    }
}
