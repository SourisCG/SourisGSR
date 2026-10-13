# Getting Started

## Quick record (Wayland)

```sh
sgsr -w portal -f 60 -a default_output -o video.mp4
```

Stop with `Ctrl-C` or `pkill -SIGINT -f "^sgsr"`.

## List devices

```sh
sgsr --list-capture-options
sgsr --list-audio-devices
sgsr --list-application-audio
sgsr --info
```

## Replay

```sh
sgsr -w portal -f 60 -r 60 -c mp4 -o ~/Videos
pkill -SIGUSR1 -f "^sgsr" # save replay
```
