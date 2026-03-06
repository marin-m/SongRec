Source: https://johnvansickle.com/ffmpeg/

Commands run:

```bash
for arch in amd64 arm64; do
    wget https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-${arch}-static.tar.xz
    tar tvf ffmpeg-release-${arch}-static.tar.xz
    tar xvf ffmpeg-release-${arch}-static.tar.xz ffmpeg-7.0.2-${arch}-static/ffmpeg
    mv ffmpeg-7.0.2-${arch}-static/ffmpeg ffmpeg-7.0.2-${arch}
    rm -rf ffmpeg-7.0.2-${arch}-static/ ffmpeg-release-${arch}-static.tar.xz
done
```
