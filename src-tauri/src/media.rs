use anyhow::{bail, Context, Result};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Decode outside WebKit/GStreamer. Output is transient; originals are never modified.
pub fn first_frame(source: &Path) -> Result<Vec<u8>> {
    let mut child = Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-v",
            "error",
            "-protocol_whitelist",
            "file,pipe",
            "-i",
        ])
        .arg(source)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-vf",
            "scale=320:-2",
            "-threads",
            "1",
            "-f",
            "image2pipe",
            "-c:v",
            "mjpeg",
            "pipe:1",
        ])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .spawn()
        .context("无法提取视频封面，请确认已安装 FFmpeg")?;
    let output = child.stdout.take().context("无法读取封面")?;
    let reader = std::thread::spawn(move || {
        let mut data = Vec::new();
        output
            .take(8 * 1024 * 1024)
            .read_to_end(&mut data)
            .map(|_| data)
    });
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            result => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = reader.join();
                bail!("视频封面提取超时或失败：{result:?}");
            }
        }
    };
    let data = reader
        .join()
        .map_err(|_| anyhow::anyhow!("封面读取失败"))??;
    if !status.success() || data.is_empty() {
        bail!("无法解码此视频的首帧");
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_is_jpeg_and_invalid_input_fails_without_modifying_original() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("video.mkv");
        let status = Command::new("ffmpeg")
            .args([
                "-nostdin",
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=green:s=64x48:d=0.1",
                "-c:v",
                "ffv1",
            ])
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
        let original = std::fs::read(&path).unwrap();
        let frame = first_frame(&path).unwrap();
        let image = image::load_from_memory(&frame).unwrap();
        assert_eq!(image.width(), 320);
        assert_eq!(std::fs::read(&path).unwrap(), original);
        let invalid = tmp.path().join("bad.mp4");
        std::fs::write(&invalid, "invalid").unwrap();
        assert!(first_frame(&invalid).is_err());
    }
}
