use std::io::{BufRead, BufReader, Read};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;

pub struct Shell {
    pub input: String,
    pub lines: Vec<String>,
    child: Option<Child>,
    rx: Option<Receiver<String>>,
    cwd: PathBuf,
}

impl Shell {
    pub fn new(cwd: PathBuf) -> Self {
        Self {
            input: String::new(),
            lines: Vec::new(),
            child: None,
            rx: None,
            cwd,
        }
    }

    pub fn running(&self) -> bool {
        self.child.is_some()
    }

    pub fn start(&mut self) -> anyhow::Result<()> {
        if self.running() || self.input.trim().is_empty() {
            return Ok(());
        }
        self.poll();
        if self.rx.is_some() {
            anyhow::bail!("Previous output is still being collected");
        }
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut cmd = Command::new(&shell);
        let name = std::path::Path::new(&shell)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if matches!(name, "nu" | "nushell" | "fish") {
            cmd.args(["--login", "-c", &self.input]);
        } else if matches!(name, "pwsh" | "powershell") {
            cmd.args(["-Login", "-Command", &self.input]);
        } else {
            cmd.args(["-lc", &self.input]);
        }
        cmd.current_dir(&self.cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            cmd.process_group(0);
        }
        let mut child = cmd.spawn()?;
        let (tx, rx) = mpsc::sync_channel(256);
        for stream in [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut reader = BufReader::new(stream);
                loop {
                    let mut bytes = Vec::new();
                    match reader.by_ref().take(8192).read_until(b'\n', &mut bytes) {
                        Ok(0) => break,
                        Ok(_) => {
                            while matches!(bytes.last(), Some(b'\n' | b'\r')) {
                                bytes.pop();
                            }
                            if tx
                                .send(String::from_utf8_lossy(&bytes).into_owned())
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(e) => {
                            let _ = tx.send(format!("read failed: {e}"));
                            break;
                        }
                    }
                }
            });
        }
        drop(tx);
        self.lines.push(format!("$ {}", self.input));
        self.input.clear();
        self.child = Some(child);
        self.rx = Some(rx);
        Ok(())
    }

    pub fn poll(&mut self) {
        if let Some(rx) = &self.rx {
            for _ in 0..256 {
                match rx.try_recv() {
                    Ok(line) => self.lines.push(line),
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.rx = None;
                        break;
                    }
                    Err(mpsc::TryRecvError::Empty) => break,
                }
            }
        }
        if self.lines.len() > 1000 {
            self.lines.drain(..self.lines.len() - 1000);
        }
        if let Some(child) = &mut self.child {
            match child.try_wait() {
                Ok(Some(status)) => {
                    #[cfg(unix)]
                    kill_group(child.id());
                    self.lines.push(format!("exit {status}"));
                    self.child = None;
                }
                Ok(None) => {}
                Err(e) => {
                    self.lines.push(format!("wait failed: {e}"));
                    self.cancel();
                }
            }
        }
    }

    pub fn cancel(&mut self) {
        if let Some(mut child) = self.child.take() {
            #[cfg(unix)]
            {
                kill_group(child.id());
            }
            let _ = child.kill();
            let _ = child.wait();
            self.lines.push("cancelled".into());
        }
    }
}
impl Drop for Shell {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(unix)]
fn kill_group(id: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", "--", &format!("-{id}")])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[cfg(unix)]
    #[test]
    fn command_runs_in_caller_directory_and_can_be_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        let mut shell = Shell::new(dir.path().to_path_buf());
        shell.input = "pwd; sleep 30".into();
        shell.start().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline
            && !shell
                .lines
                .iter()
                .any(|s| s == &dir.path().display().to_string())
        {
            shell.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(shell
            .lines
            .iter()
            .any(|s| s == &dir.path().display().to_string()));
        shell.cancel();
        assert!(!shell.running());
    }
    #[cfg(unix)]
    #[test]
    fn natural_exit_keeps_output() {
        let mut shell = Shell::new(std::env::current_dir().unwrap());
        shell.input = "printf 'finished\n'".into();
        shell.start().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline
            && (shell.running() || !shell.lines.iter().any(|s| s == "finished"))
        {
            shell.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!shell.running());
        assert!(shell.lines.iter().any(|s| s == "finished"));
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn cancel_stops_descendant_process() {
        let mut shell = Shell::new(std::env::current_dir().unwrap());
        shell.input = "sleep 30 & echo $!; wait".into();
        shell.start().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let pid = loop {
            shell.poll();
            if let Some(pid) = shell.lines.iter().find_map(|line| line.parse::<u32>().ok()) {
                break pid;
            }
            assert!(Instant::now() < deadline, "did not see descendant pid");
            std::thread::sleep(Duration::from_millis(10));
        };
        shell.cancel();
        let stat = format!("/proc/{pid}/stat");
        while Instant::now() < deadline {
            match std::fs::read_to_string(&stat) {
                Err(_) => return,
                Ok(line) if line.split_whitespace().nth(2) == Some("Z") => return,
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        panic!("descendant {pid} is still running");
    }
}
