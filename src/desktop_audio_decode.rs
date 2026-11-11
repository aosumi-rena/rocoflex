use std::{env, io::Read, path::{Path, PathBuf}, process::{Child, Command, Stdio},
    sync::atomic::{AtomicBool, Ordering}, thread::{self, JoinHandle}, time::Duration};

const MAX_SOURCE_BYTES: u64 = 128 * 1024 * 1024;
const MAX_DECODED_BYTES: u64 = 48_000 * 300 * 4;

struct DecoderChild(Child);

impl DecoderChild {
    fn stop(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let _ = self.0.kill();
        self.0.wait()
    }
}

impl Drop for DecoderChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) { let _ = self.stop(); }
    }
}

fn join_reader<T>(reader: JoinHandle<Result<T, String>>, name: &str) -> Result<T, String> {
    reader.join().map_err(|_| format!("The FFmpeg {name} reader stopped unexpectedly."))?
}

pub(super) fn detect_ffmpeg() -> Option<PathBuf> {
    let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    let mut folders = Vec::new();
    if let Ok(executable) = env::current_exe()
        && let Some(folder) = executable.parent()
    {
        folders.push(folder.to_path_buf());
        folders.push(folder.join("ffmpeg").join("bin"));
    }
    if let Some(paths) = env::var_os("PATH") { folders.extend(env::split_paths(&paths)); }
    if cfg!(windows) {
        if let Some(program_files) = env::var_os("ProgramFiles") {
            folders.push(PathBuf::from(program_files).join("ffmpeg").join("bin"));
        }
    }
    folders.into_iter().map(|folder| folder.join(name)).find(|path| path.is_file())
}

pub(super) fn decode(path: &Path, executable: &Path, cancelled: &AtomicBool) -> Result<Vec<f32>, String> {
    if cancelled.load(Ordering::Relaxed) { return Err("Audio import cancelled.".into()); }
    let metadata = path.metadata().map_err(|error| format!("Could not open audio: {error}"))?;
    if !metadata.is_file() { return Err("Choose a regular audio file.".into()); }
    if metadata.len() > MAX_SOURCE_BYTES { return Err("The audio file exceeds the 128 MiB import limit.".into()); }
    let mut command = Command::new(executable);
    command.args(["-nostdin", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(path).args(["-map", "0:a:0", "-vn", "-sn", "-dn", "-af", "pan=mono|c0=c0",
            "-ar", "48000", "-c:a", "pcm_f32le", "-f", "f32le", "pipe:1"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = DecoderChild(command.spawn().map_err(|error| format!("Could not start FFmpeg: {error}"))?);
    let stdout = child.0.stdout.take().ok_or("FFmpeg did not provide an audio pipe.")?;
    let mut stderr = child.0.stderr.take().ok_or("FFmpeg did not provide a diagnostic pipe.")?;

    let diagnostic = match std::thread::Builder::new().name("ffmpeg-diagnostics".into()).spawn(move || {
        let mut retained = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            match stderr.read(&mut chunk) {
                Ok(0) => break,
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(format!("Could not read FFmpeg diagnostics: {error}")),
                Ok(count) => {
                    let keep = count.min(4096usize.saturating_sub(retained.len()));
                    retained.extend_from_slice(&chunk[..keep]);
                }
            }
        }
        Ok(retained)
    }) {
        Ok(handle) => handle,
        Err(error) => {
            let _ = child.stop();
            return Err(format!("Could not read FFmpeg diagnostics: {error}"));
        }
    };
    let output = match thread::Builder::new().name("ffmpeg-audio".into()).spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(MAX_DECODED_BYTES + 4).read_to_end(&mut bytes)
            .map_err(|error| format!("Could not read decoded audio: {error}"))?;
        if bytes.len() as u64 > MAX_DECODED_BYTES {
            return Err("Choose audio at most 300 seconds long.".into());
        }
        Ok(bytes)
    }) {
        Ok(handle) => handle,
        Err(error) => {
            let _ = child.stop();
            let _ = join_reader(diagnostic, "diagnostic");
            return Err(format!("Could not start the FFmpeg audio reader: {error}"));
        }
    };

    let mut output = Some(output);
    let mut decoded = None;
    let status = loop {
        if cancelled.load(Ordering::Relaxed) { break child.stop(); }
        if output.as_ref().is_some_and(JoinHandle::is_finished) {
            decoded = Some(join_reader(output.take().expect("finished audio reader"), "audio"));
            if decoded.as_ref().is_some_and(Result::is_err) { break child.stop(); }
        }
        match child.0.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None) => thread::sleep(Duration::from_millis(25)),
            Err(error) => {
                let _ = child.stop();
                break Err(error);
            }
        }
    };
    let decoded = decoded.unwrap_or_else(|| join_reader(output.expect("pending audio reader"), "audio"));
    let diagnostic = join_reader(diagnostic, "diagnostic");
    if cancelled.load(Ordering::Relaxed) { return Err("Audio import cancelled.".into()); }
    let bytes = decoded?;
    let diagnostic = diagnostic?;
    if !status.map_err(|error| format!("FFmpeg did not finish: {error}"))?.success() {
        let detail = String::from_utf8_lossy(&diagnostic);
        return Err(if detail.trim().is_empty() { "FFmpeg could not decode this audio file.".into() }
            else { format!("FFmpeg could not decode this audio file:\n{}", detail.trim()) });
    }
    if bytes.len() % 4 != 0 { return Err("FFmpeg returned incomplete audio samples.".into()); }
    Ok(bytes.chunks_exact(4).map(|word|
        f32::from_le_bytes([word[0], word[1], word[2], word[3]])).collect())
}
