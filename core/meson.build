project('gpu-screen-recorder', 'c', 'cpp', version : '6.1.4', default_options : ['c_std=gnu11', 'cpp_std=c++17', 'warning_level=2'])

is_windows = build_machine.system() == 'windows'
is_linux = build_machine.system() == 'linux'

add_project_arguments('-Wshadow', language : 'c')
if get_option('buildtype') == 'debug'
    add_project_arguments('-g3', language : 'c')
elif get_option('buildtype') == 'release'
    add_project_arguments('-DNDEBUG', language : 'c')
endif

if is_windows
    add_project_arguments('-DSOURIS_WINDOWS', language : ['c', 'cpp'])
endif

src = [
    'src/capture/capture.c',
    'src/encoder/encoder.c',
    'src/encoder/video/video.c',
    'src/encoder/video/nvenc.c',
    'src/encoder/video/vaapi.c',
    'src/encoder/video/vulkan.c',
    'src/encoder/video/software.c',
    'src/codec_query/nvenc.c',
    'src/codec_query/vaapi.c',
    'src/codec_query/vulkan.c',
    'src/replay_buffer/replay_buffer.c',
    'src/replay_buffer/replay_buffer_ram.c',
    'src/replay_buffer/replay_buffer_disk.c',
    'src/log.c',
    'src/json.c',
    'src/ffmpeg_utils.c',
    'src/recorder/audio_codec.c',
    'src/recorder/video_codec.c',
    'src/recorder/codec_select.c',
    'src/recorder/windowing.c',
    'src/recorder/capture_source.c',
    'src/recorder/capture_setup.c',
    'src/recorder/audio_input.c',
    'src/recorder/audio_capture.c',
    'src/recorder/recording_clock.c',
    'src/recorder/muxer.c',
    'src/recorder/replay_save.c',
    'src/recorder/screenshot.c',
    'src/recorder/recorder.c',
    'src/cli/commands.c',
    'src/cli/ipc.c',
    'src/egl.c',
    'src/cuda.c',
    'src/window_texture.c',
    'src/shader.c',
    'src/color_conversion.c',
    'src/utils.c',
    'src/library_loader.c',
    'src/cursor.c',
    'src/damage.c',
    'src/image_writer.c',
    'src/args_parser.c',
    'src/defs.c',
    'src/plugins.c',
    'src/souris_gain.c',
    'src/souris_gsr.c',
    # Platform-neutral SourisGSR shims (working stubs on Linux, real
    # implementations live in src/capture/win/ and are added below).
    'src/capture/win_capture_stub.c',
    'src/souris_win_audio_stub.c',
    'src/encoder/video/souris_win_encode.c',
    'src/cli/main.c',
]

if is_linux
    src += [
        'kms/client/kms_client.c',
        'src/capture/nvfbc.c',
        'src/capture/xcomposite.c',
        'src/capture/ximage.c',
        'src/capture/kms.c',
        'src/capture/v4l2.c',
        'src/window/window.c',
        'src/window/x11.c',
        'src/window/wayland.c',
        'src/kde_night_light.c',
        'src/wayland_host_bridge.c',
        'src/dbus.c',
        'src/sound.c',
    ]
    subdir('protocol')
    src += protocol_src
endif

if is_windows
    # Real Windows backends: WGC (default, window+monitor) with DXGI monitor
    # fallback, WASAPI audio (loopback/mic/per-app). See docs/WINDOWS_PORT.md.
    src += [
        'src/capture/win/wgc.cpp',
        'src/capture/win/dxgi.cpp',
        'src/souris_win_audio_wasapi.cpp',
    ]
endif

cc = meson.get_compiler('c')
m_dep = cc.find_library('m', required : true)

# Overrides the libav* dependencies below with a statically linked ffmpeg built from source.
if get_option('ffmpeg_static') == true
    subproject('ffmpeg')
    add_project_arguments('-DGSR_FFMPEG_STATIC', language : 'c')
endif

dep = [
    m_dep,
    dependency('threads'),
    dependency('libavcodec'),
    dependency('libavformat'),
    dependency('libavutil'),
    dependency('libswresample'),
    dependency('libavfilter'),
]

if is_linux
    dep += [
        dependency('x11'),
        dependency('xcomposite'),
        dependency('xrandr'),
        dependency('xfixes'),
        dependency('xdamage'),
        dependency('libpulse'),
        dependency('libva'),
        dependency('libva-drm'),
        dependency('libdrm'),
        dependency('wayland-egl'),
        dependency('wayland-client'),
        dependency('dbus-1'),
        dependency('libcap'),
    ]
endif

if is_windows
    # System libraries: no pkg-config needed (SDK + vcpkg for FFmpeg).
    win_cpp = meson.get_compiler('cpp')
    dep += [
        win_cpp.find_library('d3d11'),
        win_cpp.find_library('dxgi'),
        win_cpp.find_library('ole32'),
        win_cpp.find_library('avrt'),
    ]
endif

uses_pipewire = false

# PipeWire backends only exist on Linux.
if is_linux and get_option('portal') == true
    src += [
        'src/capture/portal.c',
        'src/pipewire_video.c',
    ]
    add_project_arguments('-DGSR_PORTAL', language : 'c')
    uses_pipewire = true
endif

if is_linux and get_option('app_audio') == true
    src += [
        'src/pipewire_audio.c',
    ]
    add_project_arguments('-DGSR_APP_AUDIO', language : 'c')
    uses_pipewire = true
endif

if uses_pipewire == true
    dep += [
        dependency('libpipewire-0.3'),
        dependency('libspa-0.2'),
    ]
endif

add_project_arguments('-DGSR_VERSION="' + meson.project_version() + '"', language: 'c')

if is_linux
    executable('gsr-kms-server', 'kms/server/kms_server.c', dependencies : dependency('libdrm'), c_args : '-fstack-protector-all', install : true)
endif
executable('gsr-cli', ['tools/gsr-cli/main.c', 'src/json.c', 'src/log.c'], install : true)
executable('gpu-screen-recorder', src, dependencies : dep, install : true)

install_headers('plugin/plugin.h', install_dir : 'include/gsr')
if is_linux
    install_man('gpu-screen-recorder.1', 'gsr-cli.1', 'gsr-kms-server.1')
    install_subdir('scripts', install_dir: 'share/gpu-screen-recorder')
endif

if is_linux and get_option('systemd') == true
    install_data(files('extra/gpu-screen-recorder.service'), install_dir : 'lib/systemd/user')
endif

if is_linux and get_option('capabilities') == true
    meson.add_install_script('extra/meson_post_install.sh')
endif

if is_linux and get_option('nvidia_suspend_fix') == true
    install_data(files('extra/gsr-nvidia.conf'), install_dir : 'lib/modprobe.d')
endif

if get_option('plugin_examples') == true and is_linux
    shared_library('triangle', 'plugin/examples/hello_triangle/triangle.c',
        dependencies: [dependency('gl'), m_dep],
        name_prefix : '',
        name_suffix: 'so')
endif
