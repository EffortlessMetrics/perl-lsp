//! Linux managed-build storage supervision for the existing gate watchdog.
//!
//! This is polling, not a quota. Other writers, command-local routing overrides,
//! escaped process groups and filesystem failures can defeat a reserve. The
//! preallocated record is independent of the normal receipt's later allocation.

use color_eyre::eyre::{Context, Result, bail, eyre};
use serde::Serialize;
use std::collections::{BTreeMap, VecDeque};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::unix::{
    fs::{MetadataExt, OpenOptionsExt},
    process::CommandExt,
};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const LOG_LIMIT: usize = 4 * 1024 * 1024;
const TRUNCATED: &[u8] = b"[gate log truncated; retaining bounded output tail]\n";
const INCOMPLETE: &[u8] = b"[gate output capture incomplete after process-group settlement]\n";
const REASON_CHARS: usize = 4096;
const PROBE_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug)]
pub(super) struct Execution {
    pub output: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub supervision_error: bool,
}

#[derive(Debug)]
struct StorageFailure(String);

impl std::fmt::Display for StorageFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for StorageFailure {}

fn storage_failure(error: color_eyre::Report) -> color_eyre::Report {
    StorageFailure(format!("{error:#}")).into()
}

#[derive(Debug, Serialize)]
struct Destination {
    path: PathBuf,
    build: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct Policy {
    destinations: Vec<Destination>,
    min_free_gb: u64,
    max_used_pct: u64,
}

impl Policy {
    pub(super) fn from_environment(log: &Path, receipt: &Path) -> Result<Option<Self>> {
        Self::from_lookup(log, receipt, |key| std::env::var_os(key))
    }

    fn from_lookup(
        log: &Path,
        receipt: &Path,
        env: impl Fn(&str) -> Option<OsString>,
    ) -> Result<Option<Self>> {
        let Some(devplane) = env("DEVPLANE") else { return Ok(None) };
        let required_path = |key: &str, value: Option<OsString>| -> Result<PathBuf> {
            match value {
                Some(value) if !value.is_empty() => Ok(PathBuf::from(value)),
                _ => bail!("Managed gate requires a nonempty {key} routing path"),
            }
        };
        let number = |key: &str, default: u64| -> Result<u64> {
            match env(key).filter(|v| !v.is_empty()) {
                Some(value) => value
                    .to_str()
                    .ok_or_else(|| eyre!("Non-Unicode {key}"))?
                    .parse()
                    .with_context(|| format!("Invalid {key}")),
                None => Ok(default),
            }
        };
        let mut destinations =
            vec![Destination { path: required_path("DEVPLANE", Some(devplane))?, build: true }];
        for key in ["CARGO_HOME", "CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"] {
            destinations.push(Destination { path: required_path(key, env(key))?, build: true });
        }
        // sccache may be a daemon outside our group, but its configured output
        // volume still consumes space. We never signal that shared daemon.
        if let Some(path) = env("SCCACHE_DIR") {
            destinations
                .push(Destination { path: required_path("SCCACHE_DIR", Some(path))?, build: true });
        }
        let temporary = env("TMPDIR").filter(|v| !v.is_empty()).unwrap_or_else(|| "/tmp".into());
        for path in [PathBuf::from(temporary), log.to_owned(), receipt.to_owned()] {
            destinations.push(Destination { path, build: false });
        }
        Ok(Some(Self {
            destinations,
            min_free_gb: number("MIN_FREE_GB", 40)?,
            max_used_pct: number("MAX_USED_PCT", 85)?,
        }))
    }

    fn check(&self) -> Result<()> {
        // Resolve again on each sample: a newly created destination can be on a
        // different mount than its previously existing ancestor. The stricter
        // build predicate wins when several roles share a device.
        let mut devices: BTreeMap<u64, (PathBuf, Space, bool)> = BTreeMap::new();
        for destination in &self.destinations {
            let (ancestor, _) = resolve_storage_path(&destination.path)?;
            let (device, space) = probe(&ancestor)?;
            devices.entry(device).and_modify(|entry| entry.2 |= destination.build).or_insert((
                ancestor,
                space,
                destination.build,
            ));
        }
        for (_, (path, space, build)) in devices {
            if space.denied(self.min_free_gb, self.max_used_pct, build) {
                bail!(
                    "Storage admission lost: {} used={}% free={}G build_floor={build}",
                    path.display(),
                    space.used_pct,
                    space.available_kb / 1024 / 1024
                );
            }
        }
        Ok(())
    }
}

fn resolve_storage_path(path: &Path) -> Result<(PathBuf, PathBuf)> {
    let absolute =
        if path.is_absolute() { path.to_owned() } else { std::env::current_dir()?.join(path) };
    for ancestor in absolute.ancestors() {
        match fs::metadata(ancestor) {
            Ok(_) => {
                let resolved = fs::canonicalize(ancestor)?;
                let destination = resolved.join(absolute.strip_prefix(ancestor)?);
                return Ok((resolved, destination));
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                // A dangling symlink could route future output to another
                // filesystem. Refuse it rather than monitoring its lexical
                // parent's mount and reserving evidence on the wrong device.
                match fs::symlink_metadata(ancestor) {
                    Ok(metadata) if metadata.file_type().is_symlink() => {
                        bail!("Unresolved storage symlink: {}", ancestor.display())
                    }
                    Err(error) if error.kind() != io::ErrorKind::NotFound => {
                        return Err(error.into());
                    }
                    _ => {}
                }
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Cannot resolve storage path {}", path.display()));
            }
        }
    }
    bail!("No existing ancestor for {}", path.display())
}

#[derive(Debug)]
struct Space {
    available_kb: u128,
    used_pct: u128,
}

impl Space {
    fn from_blocks(fragment: u128, blocks: u128, free: u128, available: u128) -> Result<Self> {
        if fragment == 0 || blocks == 0 || free > blocks || available > free {
            bail!("Invalid filesystem capacity counters");
        }
        let used = blocks - free;
        let denominator = used.checked_add(available).ok_or_else(|| eyre!("Capacity overflow"))?;
        if denominator == 0 {
            bail!("Filesystem has no usable capacity");
        }
        let available_kb = available
            .checked_mul(fragment)
            .ok_or_else(|| eyre!("Capacity overflow"))?
            .div_ceil(1024);
        let used_pct =
            used.checked_mul(100).ok_or_else(|| eyre!("Capacity overflow"))?.div_ceil(denominator);
        Ok(Self { available_kb, used_pct })
    }

    fn denied(&self, min_free_gb: u64, max_used_pct: u64, build: bool) -> bool {
        self.used_pct >= u128::from(max_used_pct)
            || (build && self.available_kb / 1024 / 1024 < u128::from(min_free_gb))
    }
}

fn probe(path: &Path) -> Result<(u64, Space)> {
    // Hold the queried inode so statvfs and the device identity refer to the
    // same filesystem even if a path is replaced between the two calls.
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(path)
        .with_context(|| format!("Cannot probe {}", path.display()))?;
    let metadata = file.metadata()?;
    if !metadata.is_dir() && !metadata.is_file() {
        bail!("Unsupported storage destination type: {}", path.display());
    }
    let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: fd remains open; fstatvfs initializes stats on success.
    if unsafe { libc::fstatvfs(file.as_raw_fd(), stats.as_mut_ptr()) } != 0 {
        return Err(io::Error::last_os_error()).context("Filesystem capacity probe failed");
    }
    // SAFETY: successful fstatvfs initialized the value above.
    let stats = unsafe { stats.assume_init() };
    let space = Space::from_blocks(
        stats.f_frsize as u128,
        stats.f_blocks as u128,
        stats.f_bfree as u128,
        stats.f_bavail as u128,
    )?;
    Ok((metadata.dev(), space))
}

#[derive(Serialize)]
struct EmergencyRecord<'a> {
    schema: &'static str,
    status: &'static str,
    command: &'a str,
    log_path: &'a Path,
    receipt_path: &'a Path,
    policy: &'a Policy,
    reason: String,
}

struct Emergency {
    file: File,
    path: PathBuf,
    capacity: usize,
}

impl Emergency {
    fn create(record: &EmergencyRecord<'_>) -> Result<Self> {
        Self::create_with_device(record, |path| {
            let (ancestor, _) = resolve_storage_path(path)?;
            Ok(fs::metadata(ancestor)?.dev())
        })
    }

    fn create_with_device(
        record: &EmergencyRecord<'_>,
        device: impl Fn(&Path) -> Result<u64>,
    ) -> Result<Self> {
        let (_, resolved_receipt) = resolve_storage_path(record.receipt_path)?;
        let parent = resolved_receipt
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).context("Cannot create emergency receipt directory")?;
        let directory = File::open(parent).context("Cannot open emergency receipt directory")?;
        let mut temporary =
            tempfile::Builder::new().prefix("gate-storage-").suffix(".json").tempfile_in(parent)?;
        // A receipt can be an individually bind-mounted file. Its canonical
        // parent may be on a different filesystem, where a sibling reserve
        // would provide the wrong protection. Re-resolve the requested route
        // after opening the private file and refuse mismatched identities.
        if temporary.as_file().metadata()?.dev() != device(record.receipt_path)? {
            bail!("Cannot reserve evidence on the receipt destination filesystem");
        }
        // Known fields are serialized, so quoting/Unicode paths count correctly.
        // A reason is bounded by characters; JSON needs at most six bytes per
        // character. Status text fits the additional fixed allowance.
        let capacity = serde_json::to_vec(record)?
            .len()
            .checked_add(REASON_CHARS * 6 + 512)
            .ok_or_else(|| eyre!("Emergency receipt size overflow"))?;
        let length = libc::off_t::try_from(capacity)?;
        // SAFETY: live private fd, nonnegative checked length. Unlike set_len,
        // posix_fallocate reserves physical blocks or reports an error.
        let allocation = unsafe { libc::posix_fallocate(temporary.as_raw_fd(), 0, length) };
        if allocation != 0 {
            return Err(io::Error::from_raw_os_error(allocation))
                .context("Cannot reserve emergency receipt");
        }
        write_record(temporary.as_file_mut(), capacity, record)?;
        let (file, path) = temporary.keep()?;
        directory.sync_all().with_context(|| {
            format!(
                "Cannot sync emergency receipt directory; incomplete record retained at {}",
                path.display()
            )
        })?;
        Ok(Self { file, path, capacity })
    }

    fn record(&mut self, record: &EmergencyRecord<'_>) -> Result<()> {
        write_record(&mut self.file, self.capacity, record)
    }
}

fn write_record(file: &mut File, capacity: usize, record: &EmergencyRecord<'_>) -> Result<()> {
    let mut bytes = serde_json::to_vec(record)?;
    if bytes.len() > capacity {
        bail!("Emergency receipt exceeded its preallocated capacity");
    }
    bytes.resize(capacity, b' ');
    file.seek(SeekFrom::Start(0))?;
    file.write_all(&bytes)?;
    file.sync_all().context("Cannot sync emergency receipt")
}

struct Tail {
    bytes: VecDeque<u8>,
    truncated: bool,
    incomplete: bool,
}

impl Tail {
    fn new() -> Self {
        Self { bytes: VecDeque::new(), truncated: false, incomplete: false }
    }

    fn append(&mut self, bytes: &[u8]) {
        let capacity = LOG_LIMIT - TRUNCATED.len() - INCOMPLETE.len();
        let discard = self.bytes.len().saturating_add(bytes.len()).saturating_sub(capacity);
        if discard > 0 {
            self.truncated = true;
            self.bytes.drain(..discard.min(self.bytes.len()));
        }
        self.bytes.extend(bytes.iter().skip(bytes.len().saturating_sub(capacity)));
    }

    fn bytes(&self) -> Vec<u8> {
        let mut result = Vec::with_capacity(LOG_LIMIT);
        if self.truncated {
            result.extend_from_slice(TRUNCATED);
        }
        if self.incomplete {
            result.extend_from_slice(INCOMPLETE);
        }
        result.extend(&self.bytes);
        result
    }

    fn save(&self, file: &mut File) -> Result<()> {
        let bytes = self.bytes();
        file.seek(SeekFrom::Start(0))?;
        file.write_all(&bytes)?;
        file.set_len(bytes.len() as u64)?;
        file.sync_all().context("Cannot sync bounded gate log")
    }
}

fn nonblocking(pipe: &impl AsRawFd) -> Result<()> {
    // SAFETY: a live owned pipe descriptor, valid fcntl operations.
    let flags = unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(pipe.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(io::Error::last_os_error()).context("Cannot set nonblocking gate pipe");
    }
    Ok(())
}

enum Drain {
    Eof,
    WouldBlock,
    Budget,
}

fn drain(pipe: &mut impl Read, tail: &mut Tail) -> Result<Drain> {
    let mut bytes = [0u8; 16 * 1024];
    // Per-stream budget: continuous stdout cannot starve stderr, probes or the
    // existing deadline. Remaining bytes are drained on the next iteration.
    for _ in 0..16 {
        match pipe.read(&mut bytes) {
            Ok(0) => return Ok(Drain::Eof),
            Ok(count) => tail.append(&bytes[..count]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                return Ok(Drain::WouldBlock);
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error).context("Cannot drain gate output"),
        }
    }
    Ok(Drain::Budget)
}

fn drain_settled(pipe: &mut impl Read, tail: &mut Tail) -> Result<()> {
    let start = Instant::now();
    for _ in 0..64 {
        match drain(pipe, tail)? {
            Drain::Eof => return Ok(()),
            Drain::WouldBlock => {
                // An open writer may produce more bytes later. Even an empty
                // pipe is not EOF, so do not claim a complete final capture.
                tail.incomplete = true;
                return Ok(());
            }
            Drain::Budget => {}
        }
        if start.elapsed() >= PROBE_INTERVAL {
            break;
        }
    }
    // An escaped writer can keep filling a pipe. Bound settlement readback too,
    // but make any intentionally unread output explicit instead of silently
    // losing a large pipe's final test summary.
    tail.incomplete = true;
    Ok(())
}

fn finish_result<T>(primary: Result<T>, capture: Result<()>, saved: Result<()>) -> Result<T> {
    match primary {
        Ok(value) => {
            capture?;
            saved?;
            Ok(value)
        }
        Err(error) => {
            let storage = error.downcast_ref::<StorageFailure>().is_some();
            let mut reason = format!("{error:#}");
            for (operation, result) in [("cleanup", capture), ("log persistence", saved)] {
                if let Err(error) = result {
                    reason.push_str(&format!("; {operation} also failed: {error:#}"));
                }
            }
            if storage { Err(StorageFailure(reason).into()) } else { Err(eyre!(reason)) }
        }
    }
}

/// Retains the direct child until group settlement, including after leader exit.
/// try_wait would reap it, permitting PID reuse before a later group signal.
struct OwnedChild {
    child: Child,
    settled: bool,
}

impl OwnedChild {
    fn exited(&self) -> Result<bool> {
        // SAFETY: zero-initialized siginfo_t is valid; waitid initializes it on
        // an observed child event. WNOWAIT deliberately leaves our child owned.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let result = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id(),
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if result != 0 {
            return Err(io::Error::last_os_error()).context("Cannot observe owned gate child");
        }
        // SAFETY: si_pid is the documented field for a waitid result.
        Ok(unsafe { info.si_pid() } != 0)
    }

    fn signal(&self, signal: i32) -> Result<()> {
        let pid = i32::try_from(self.child.id())?;
        // SAFETY: this process group was created for our still-unreaped child.
        if unsafe { libc::kill(-pid, signal) } != 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error).context("Cannot signal owned gate group");
            }
        }
        Ok(())
    }

    fn settle(&mut self, grace: bool) -> Result<i32> {
        if grace {
            self.signal(libc::SIGTERM)?;
            std::thread::sleep(Duration::from_secs(1));
        }
        // Signal descendants after a normally exited leader too. Reaping below
        // waits for our direct child; signal delivery to grandchildren can be
        // asynchronous. We do not claim to reap non-child or escaped processes.
        self.signal(libc::SIGKILL)?;
        let status = self.child.wait().context("Cannot reap owned gate child")?;
        self.settled = true;
        Ok(status.code().unwrap_or(-1))
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.settled {
            let _ = self.signal(libc::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

pub(super) fn run(
    command: &mut Command,
    text: &str,
    log: &Path,
    receipt: &Path,
    timeout: Duration,
    policy: &Policy,
) -> Result<Execution> {
    run_with_checks(
        command,
        text,
        log,
        receipt,
        timeout,
        policy,
        Checks { space: &mut || policy.check(), remove_record: &|path| fs::remove_file(path) },
    )
}

struct Checks<'a> {
    space: &'a mut dyn FnMut() -> Result<()>,
    remove_record: &'a dyn Fn(&Path) -> io::Result<()>,
}

#[cfg(test)]
fn run_with_probe(
    command: &mut Command,
    text: &str,
    log: &Path,
    receipt: &Path,
    timeout: Duration,
    policy: &Policy,
    mut check: impl FnMut() -> Result<()>,
) -> Result<Execution> {
    run_with_checks(
        command,
        text,
        log,
        receipt,
        timeout,
        policy,
        Checks { space: &mut check, remove_record: &|path| fs::remove_file(path) },
    )
}

fn run_with_checks(
    command: &mut Command,
    text: &str,
    log: &Path,
    receipt: &Path,
    timeout: Duration,
    policy: &Policy,
    checks: Checks<'_>,
) -> Result<Execution> {
    (checks.space)()?;
    let mut record = EmergencyRecord {
        schema: "gate-storage/1",
        status: "incomplete",
        command: text,
        log_path: log,
        receipt_path: receipt,
        policy,
        reason: "Gate has not completed; this is not a passing receipt".to_owned(),
    };
    let mut emergency = Emergency::create(&record)?;
    let mut tail = Tail::new();
    let execution = (|| -> Result<(i32, bool)> {
        let mut log_file =
            File::create(log).context("Cannot create bounded gate log").map_err(storage_failure)?;
        // Admission follows reserve allocation, before any child can write.
        (checks.space)().map_err(storage_failure)?;
        command.process_group(0).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut owned = OwnedChild {
            child: command.spawn().context("Cannot spawn guarded gate")?,
            settled: false,
        };
        let pipes = (|| -> Result<_> {
            let stdout = owned.child.stdout.take().ok_or_else(|| eyre!("Missing stdout pipe"))?;
            let stderr = owned.child.stderr.take().ok_or_else(|| eyre!("Missing stderr pipe"))?;
            nonblocking(&stdout)?;
            nonblocking(&stderr)?;
            Ok((stdout, stderr))
        })();
        let (mut stdout, mut stderr) = match pipes {
            Ok(pipes) => pipes,
            Err(error) => {
                return finish_result(
                    Err(error),
                    owned
                        .settle(true)
                        .map(|_| ())
                        .context("Owned child settlement after pipe setup failure"),
                    Ok(()),
                );
            }
        };
        let start = Instant::now();
        let mut sampled = start;
        let mut heartbeat = start;
        let result = (|| -> Result<(i32, bool)> {
            loop {
                drain(&mut stdout, &mut tail)?;
                drain(&mut stderr, &mut tail)?;
                if sampled.elapsed() >= PROBE_INTERVAL {
                    (checks.space)().map_err(storage_failure)?;
                    sampled = Instant::now();
                }
                if owned.exited()? {
                    return Ok((owned.settle(false)?, false));
                }
                if start.elapsed() >= timeout {
                    owned.settle(true)?;
                    return Ok((124, true));
                }
                if heartbeat.elapsed() >= Duration::from_secs(30) {
                    tail.save(&mut log_file).map_err(storage_failure)?;
                    println!(
                        "gate command still running elapsed_ms={} storage_guard=active",
                        start.elapsed().as_millis()
                    );
                    heartbeat = Instant::now();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
        })();
        if result.is_err()
            && let Err(settlement) = owned.settle(true)
        {
            return finish_result(
                result,
                Err(settlement).context("Owned child settlement failed"),
                Ok(()),
            );
        }
        // Bounded drain after group signaling never waits indefinitely for EOF.
        let capture = drain_settled(&mut stdout, &mut tail)
            .and_then(|()| drain_settled(&mut stderr, &mut tail));
        if capture.is_err() {
            tail.incomplete = true;
        }
        // Preserve the original resource cause if writing the ordinary log also
        // fails; the emergency record is already allocated on the receipt mount.
        let saved = tail.save(&mut log_file).map_err(storage_failure);
        let outcome = finish_result(result, capture, saved)?;
        (checks.remove_record)(&emergency.path).with_context(|| {
            format!(
                "Gate child completed; cannot remove emergency record {}",
                emergency.path.display()
            )
        })?;
        Ok(outcome)
    })();
    match execution {
        Ok((exit_code, timed_out)) => Ok(Execution {
            output: String::from_utf8_lossy(&tail.bytes()).into_owned(),
            exit_code,
            timed_out: timed_out || exit_code == 124,
            supervision_error: false,
        }),
        Err(error) => {
            let resource_failure = error.downcast_ref::<StorageFailure>().is_some();
            record.status = "error";
            record.reason = format!("{error:#}").chars().take(REASON_CHARS).collect();
            let evidence = match emergency.record(&record) {
                Ok(()) => format!("synced emergency record: {}", emergency.path.display()),
                Err(error) => format!(
                    "emergency record update FAILED: {error:#}; reserved path: {}",
                    emergency.path.display()
                ),
            };
            Ok(Execution {
                output: format!(
                    "{}\nGate supervision error ({}): {}\n{}\n",
                    String::from_utf8_lossy(&tail.bytes()),
                    if resource_failure { "storage" } else { "execution" },
                    record.reason,
                    evidence
                ),
                exit_code: if resource_failure { 75 } else { 70 },
                timed_out: false,
                supervision_error: true,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(path: &Path) -> Policy {
        Policy {
            destinations: vec![Destination { path: path.to_owned(), build: true }],
            min_free_gb: 20,
            max_used_pct: 95,
        }
    }

    fn shell(text: &str) -> Command {
        let mut command = Command::new("bash");
        command.args(["-c", text]);
        command
    }

    fn records(parent: &Path) -> Result<Vec<PathBuf>> {
        let mut records = Vec::new();
        for entry in fs::read_dir(parent)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with("gate-storage-") {
                records.push(entry.path());
            }
        }
        Ok(records)
    }

    #[test]
    fn capacity_matches_df_units_reserved_blocks_and_boundaries() -> Result<()> {
        let gib = 1024 * 1024 * 1024u128;
        let at_floor = Space::from_blocks(512, 100 * gib / 512, 22 * gib / 512, 20 * gib / 512)?;
        assert_eq!(at_floor.available_kb, 20 * 1024 * 1024);
        assert_eq!(at_floor.used_pct, 80); // reserved blocks excluded from denominator
        assert!(!at_floor.denied(20, 95, true));
        // df -Pk rounds a partial KiB upward before the shell's GiB division.
        let rounded = Space::from_blocks(512, 100 * gib / 512, 22 * gib / 512, 20 * gib / 512 - 1)?;
        assert!(!rounded.denied(20, 95, true));
        let below = Space::from_blocks(512, 100 * gib / 512, 22 * gib / 512, 20 * gib / 512 - 2)?;
        assert!(below.denied(20, 95, true));
        assert!(!below.denied(20, 95, false));
        let ceiling = Space::from_blocks(1024, 100, 5, 5)?;
        assert!(ceiling.denied(0, 95, false));
        assert!(Space::from_blocks(0, 100, 5, 5).is_err());
        assert!(Space::from_blocks(4096, 100, 5, 6).is_err());
        assert!(Space::from_blocks(4096, 0, 0, 0).is_err());
        assert!(Space::from_blocks(u128::MAX, 100, 5, 5).is_err());
        Ok(())
    }

    #[test]
    fn route_contract_defaults_missing_values_and_ancestor_errors() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path();
        assert!(Policy::from_lookup(path, path, |_| None)?.is_none());
        assert!(
            Policy::from_lookup(path, path, |key| (key == "DEVPLANE")
                .then(|| path.as_os_str().to_owned()))
            .is_err()
        );
        let env = |key: &str| {
            ["DEVPLANE", "CARGO_HOME", "CARGO_TARGET_DIR", "CARGO_BUILD_BUILD_DIR"]
                .contains(&key)
                .then(|| path.as_os_str().to_owned())
        };
        let policy =
            Policy::from_lookup(path, path, env)?.ok_or_else(|| eyre!("missing policy"))?;
        assert_eq!((policy.min_free_gb, policy.max_used_pct), (40, 85));
        assert_eq!(policy.destinations.len(), 7);
        assert_eq!(resolve_storage_path(&path.join("not-yet/created"))?.0, path);
        let file = path.join("file");
        fs::write(&file, "ordinary file")?;
        assert!(resolve_storage_path(&file.join("child")).is_err());
        assert!(
            Policy::from_lookup(path, path, |key| if key == "MIN_FREE_GB" {
                Some("invalid".into())
            } else {
                env(key)
            })
            .is_err()
        );
        Ok(())
    }

    #[test]
    fn live_probe_and_shared_build_role_remain_strict() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let (device, _) = probe(temp.path())?;
        assert_eq!(device, fs::metadata(temp.path())?.dev());
        // Exact df arithmetic is tested from fixed counters above. Live free
        // space can change between processes, so do not assert that two samples
        // on a shared filesystem have identical available capacity.
        let strict = Policy {
            destinations: vec![
                Destination { path: temp.path().into(), build: false },
                Destination { path: temp.path().join("future"), build: true },
            ],
            min_free_gb: u64::MAX,
            max_used_pct: 100,
        };
        assert!(strict.check().is_err());
        Ok(())
    }

    #[test]
    fn healthy_success_failure_and_deadline_keep_exit_semantics() -> Result<()> {
        let temp = tempfile::tempdir()?;
        for (text, code, timed_out) in
            [("printf good", 0, false), ("printf bad; exit 75", 75, false), ("sleep 60", 124, true)]
        {
            let result = run_with_probe(
                &mut shell(text),
                text,
                &temp.path().join("gate.log"),
                &temp.path().join("receipt.json"),
                Duration::from_millis(100),
                &policy(temp.path()),
                || Ok(()),
            )?;
            assert_eq!(result.exit_code, code);
            assert_eq!(result.timed_out, timed_out);
            assert!(!(result.supervision_error && result.exit_code == 75));
            assert!(records(temp.path())?.is_empty());
        }
        Ok(())
    }

    fn assert_stopped(pid: u32) -> Result<()> {
        for _ in 0..100 {
            match fs::read_to_string(format!("/proc/{pid}/stat")) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Ok(stat)
                    if stat.rsplit_once(") ").and_then(|(_, rest)| rest.chars().next())
                        == Some('Z') =>
                {
                    return Ok(());
                }
                Err(error) => return Err(error.into()),
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        bail!("Owned fixture child {pid} remains running")
    }

    #[test]
    fn low_space_terminates_resistant_group_and_syncs_failure_on_receipt_mount() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let receipt_dir = temp.path().join("custom-receipts");
        fs::create_dir(&receipt_dir)?;
        let pids = temp.path().join("pids");
        let text = "trap '' TERM; sleep 60 & printf '%s %s' \"$$\" \"$!\" > \"$PIDS\"; while :; do printf noisy; done";
        let mut command = shell(text);
        command.env("PIDS", &pids);
        let mut samples = 0;
        let result = run_with_probe(
            &mut command,
            text,
            &temp.path().join("gate.log"),
            &receipt_dir.join("receipt.json"),
            Duration::from_secs(10),
            &policy(temp.path()),
            || {
                samples += 1;
                if pids.exists() {
                    bail!("injected low-space observation");
                }
                Ok(())
            },
        )?;
        assert!(samples >= 3);
        assert!((result.supervision_error && result.exit_code == 75));
        assert!(!result.timed_out);
        assert_eq!(result.exit_code, 75);
        let pids: Vec<u32> = fs::read_to_string(pids)?
            .split_whitespace()
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?;
        assert_eq!(pids.len(), 2);
        for pid in pids {
            assert_stopped(pid)?;
        }
        let records = records(&receipt_dir)?;
        assert_eq!(records.len(), 1);
        let record: serde_json::Value = serde_json::from_slice(&fs::read(&records[0])?)?;
        assert_eq!(record["status"], "error");
        assert!(
            record["reason"].as_str().is_some_and(|reason| reason.contains("injected low-space"))
        );
        assert!(result.output.contains("synced emergency record"));
        // A later ordinary receipt allocation/write failure cannot erase the
        // independent, already-synced resource record.
        fs::create_dir(receipt_dir.join("receipt.json"))?;
        assert!(fs::write(receipt_dir.join("receipt.json"), "ordinary receipt").is_err());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&records[0])?)?["status"],
            "error"
        );
        assert!(fs::metadata(temp.path().join("gate.log"))?.len() <= LOG_LIMIT as u64);
        Ok(())
    }

    #[test]
    fn exited_leader_cannot_leave_pipe_holding_descendant_running() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let pids = temp.path().join("pid");
        let text = "sleep 60 & printf '%s' \"$!\" > \"$PIDS\"; exit 0";
        let mut command = shell(text);
        command.env("PIDS", &pids);
        let start = Instant::now();
        let result = run_with_probe(
            &mut command,
            text,
            &temp.path().join("gate.log"),
            &temp.path().join("receipt.json"),
            Duration::from_secs(10),
            &policy(temp.path()),
            || Ok(()),
        )?;
        assert_eq!(result.exit_code, 0);
        assert!(!(result.supervision_error && result.exit_code == 75));
        assert!(start.elapsed() < Duration::from_secs(5));
        assert_stopped(fs::read_to_string(pids)?.parse()?)?;
        Ok(())
    }

    #[test]
    fn output_is_bounded_on_disk_and_retains_binary_and_final_summary() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let text = "python3 -c 'import os; os.write(1,b\"x\"*5000000); os.write(2,b\"stderr\\n\"); os.write(1,b\"\\xff\\nFINAL-SUMMARY\\n\")'";
        let log = temp.path().join("gate.log");
        let result = run_with_probe(
            &mut shell(text),
            text,
            &log,
            &temp.path().join("receipt.json"),
            Duration::from_secs(10),
            &policy(temp.path()),
            || Ok(()),
        )?;
        assert_eq!(result.exit_code, 0);
        let bytes = fs::read(log)?;
        assert!(bytes.len() <= LOG_LIMIT);
        assert!(bytes.starts_with(TRUNCATED));
        assert!(bytes.contains(&0xff));
        assert!(result.output.contains("FINAL-SUMMARY"));
        assert!(result.output.contains("stderr"));
        Ok(())
    }

    #[test]
    fn final_drain_reads_large_buffer_and_marks_an_unsettled_stream() -> Result<()> {
        let mut buffered = vec![b'x'; 1024 * 1024];
        buffered.extend_from_slice(b"FINAL buffered summary\n");
        let mut tail = Tail::new();
        drain_settled(&mut io::Cursor::new(buffered), &mut tail)?;
        assert!(!tail.incomplete);
        assert!(tail.bytes().ends_with(b"FINAL buffered summary\n"));
        // Models a pipe kept busy by an escaped writer, without leaving a real
        // escaped process behind. Settlement must remain bounded and explicit.
        let start = Instant::now();
        drain_settled(&mut io::repeat(b'x'), &mut tail)?;
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(tail.incomplete);
        assert!(tail.bytes().windows(INCOMPLETE.len()).any(|bytes| bytes == INCOMPLETE));
        assert!(tail.bytes().len() <= LOG_LIMIT);
        Ok(())
    }

    #[test]
    fn final_drain_marks_open_pipe_and_preserves_primary_storage_error() -> Result<()> {
        use std::os::fd::FromRawFd;
        let mut fds = [0; 2];
        // SAFETY: pipe2 initializes exactly two owned descriptors on success.
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC | libc::O_NONBLOCK) } != 0 {
            return Err(io::Error::last_os_error().into());
        }
        // SAFETY: each new descriptor is transferred to one File owner.
        let mut reader = unsafe { File::from_raw_fd(fds[0]) };
        let writer = unsafe { File::from_raw_fd(fds[1]) };
        let mut tail = Tail::new();
        drain_settled(&mut reader, &mut tail)?;
        assert!(tail.incomplete, "open writer and EAGAIN are not EOF");
        drop(writer);
        let mut closed = Tail::new();
        drain_settled(&mut reader, &mut closed)?;
        assert!(!closed.incomplete);

        let result: Result<()> = finish_result(
            Err(StorageFailure("primary low space".to_owned()).into()),
            Err(eyre!("injected post-settlement read failure")),
            Err(eyre!("injected log write failure")),
        );
        let error = result.err().ok_or_else(|| eyre!("failure was lost"))?;
        assert!(error.downcast_ref::<StorageFailure>().is_some());
        let message = error.to_string();
        assert!(message.contains("primary low space"));
        assert!(message.contains("post-settlement read failure"));
        assert!(message.contains("log write failure"));
        Ok(())
    }

    #[test]
    fn missing_executable_is_execution_error_not_low_space() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let result = run_with_probe(
            &mut Command::new(temp.path().join("missing")),
            "missing executable",
            &temp.path().join("gate.log"),
            &temp.path().join("receipt.json"),
            Duration::from_secs(2),
            &policy(temp.path()),
            || Ok(()),
        )?;
        assert!(result.supervision_error);
        assert_eq!(result.exit_code, 70);
        assert!(result.output.contains("supervision error (execution)"));
        assert!(!result.output.contains("Storage admission lost"));
        Ok(())
    }

    #[test]
    fn completed_child_unlink_failure_is_execution_error_with_retained_record() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let marker = temp.path().join("completed");
        let text = "printf completed > \"$MARKER\"";
        let mut command = shell(text);
        command.env("MARKER", &marker);
        let result = run_with_checks(
            &mut command,
            text,
            &temp.path().join("gate.log"),
            &temp.path().join("receipt.json"),
            Duration::from_secs(2),
            &policy(temp.path()),
            Checks {
                space: &mut || Ok(()),
                remove_record: &|_| Err(io::Error::from_raw_os_error(libc::EACCES)),
            },
        )?;
        assert_eq!(fs::read_to_string(marker)?, "completed");
        assert!(result.supervision_error);
        assert_eq!(result.exit_code, 70);
        assert!(result.output.contains("Gate child completed"));
        assert!(result.output.contains("synced emergency record"));
        assert!(!result.output.contains("setup failed"));
        let paths = records(temp.path())?;
        assert_eq!(paths.len(), 1);
        let record: serde_json::Value = serde_json::from_slice(&fs::read(&paths[0])?)?;
        assert_eq!(record["status"], "error");
        assert!(record["reason"].as_str().is_some_and(|s| s.contains("child completed")));
        Ok(())
    }

    #[test]
    fn setup_probe_and_log_failures_do_not_start_a_child() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let marker = temp.path().join("started");
        let text = "touch \"$MARKER\"";
        let mut command = shell(text);
        command.env("MARKER", &marker);
        assert!(
            run_with_probe(
                &mut command,
                text,
                &temp.path().join("log"),
                &temp.path().join("receipt.json"),
                Duration::from_secs(2),
                &policy(temp.path()),
                || bail!("injected probe failure")
            )
            .is_err()
        );
        assert!(!marker.exists());
        let blocked = temp.path().join("blocked");
        fs::write(&blocked, "not a directory")?;
        assert!(
            run_with_probe(
                &mut command,
                text,
                &temp.path().join("log"),
                &blocked.join("receipt.json"),
                Duration::from_secs(2),
                &policy(temp.path()),
                || Ok(())
            )
            .is_err()
        );
        assert!(!marker.exists());
        let result = run_with_probe(
            &mut command,
            text,
            temp.path(),
            &temp.path().join("receipt.json"),
            Duration::from_secs(2),
            &policy(temp.path()),
            || Ok(()),
        )?;
        assert!((result.supervision_error && result.exit_code == 75));
        assert!(!marker.exists());
        assert!(result.output.contains("synced emergency record"));
        Ok(())
    }

    #[test]
    fn emergency_bounds_count_escaping_and_failed_rewrite_never_claims_sync() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let path = temp.path().join("receipt.json");
        let policy = policy(temp.path());
        let mut record = EmergencyRecord {
            schema: "gate-storage/1",
            status: "incomplete",
            command: "\"quoted\"\ncommand",
            log_path: &path,
            receipt_path: &path,
            policy: &policy,
            reason: String::new(),
        };
        let mut emergency = Emergency::create(&record)?;
        record.status = "error";
        record.reason = "\u{0001}".repeat(REASON_CHARS);
        emergency.record(&record)?;
        assert_eq!(fs::metadata(&emergency.path)?.len(), emergency.capacity as u64);
        let recorded: serde_json::Value = serde_json::from_slice(&fs::read(&emergency.path)?)?;
        assert_eq!(recorded["reason"], record.reason);
        emergency.file = File::open(&emergency.path)?; // controlled read-only handle
        assert!(emergency.record(&record).is_err());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&emergency.path)?)?["status"],
            "error"
        );
        Ok(())
    }

    #[test]
    fn emergency_reserve_follows_receipt_symlink_and_rejects_unresolved_routes() -> Result<()> {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir()?;
        let physical = temp.path().join("actual receipt destination");
        fs::create_dir(&physical)?;
        let target = physical.join("receipt.json");
        fs::write(&target, "previous receipt")?;
        let requested = temp.path().join("receipt-link.json");
        symlink(&target, &requested)?;
        let policy = policy(temp.path());
        let record = EmergencyRecord {
            schema: "gate-storage/1",
            status: "incomplete",
            command: "fixture",
            log_path: &requested,
            receipt_path: &requested,
            policy: &policy,
            reason: String::new(),
        };
        let emergency = Emergency::create(&record)?;
        assert_eq!(emergency.path.parent(), Some(physical.as_path()));
        assert_eq!(emergency.file.metadata()?.dev(), fs::metadata(&target)?.dev());
        assert_eq!(records(&physical)?.len(), 1);
        assert!(records(temp.path())?.is_empty());
        let parent_link = temp.path().join("parent-link");
        symlink(&physical, &parent_link)?;
        assert_eq!(
            resolve_storage_path(&parent_link.join("new/receipt.json"))?.1,
            physical.join("new/receipt.json")
        );
        let dangling = temp.path().join("dangling");
        symlink(physical.join("missing"), &dangling)?;
        assert!(resolve_storage_path(&dangling.join("receipt.json")).is_err());
        assert!(resolve_storage_path(&dangling).is_err());
        let mismatch = Emergency::create_with_device(&record, |_| {
            Ok(emergency.file.metadata()?.dev().wrapping_add(1))
        });
        assert!(mismatch.is_err());
        assert!(
            mismatch
                .err()
                .is_some_and(|e| e.to_string().contains("receipt destination filesystem"))
        );
        assert_eq!(records(&physical)?.len(), 1, "mismatched temporary reserve must be removed");
        Ok(())
    }
}
