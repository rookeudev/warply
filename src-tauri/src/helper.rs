//! One elevated, headless helper per app session. The webview stays unelevated.
//! Bounded local pipe frames, PID/image checks in both directions, no arbitrary paths/commands.
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const MAX_FRAME: usize = 131_072;

#[derive(Deserialize, Serialize)]
#[serde(transparent)]
struct Secret(String);
impl Drop for Secret {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "operation", deny_unknown_fields)]
enum Request {
    Inspect {},
    Reconcile {},
    ValidateProtection {},
    Connect { profile: Secret, protect: bool },
    Stop { unlock: bool },
    Repair {},
    InstallWireguard {},
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    guard: crate::guard::GuardView,
    error: Option<String>,
}

fn write_frame(writer: &mut impl std::io::Write, value: &impl Serialize) -> Result<(), String> {
    let bytes = Zeroizing::new(
        serde_json::to_vec(value).map_err(|_| "Could not prepare a helper request.")?,
    );
    if bytes.is_empty() || bytes.len() > MAX_FRAME {
        return Err("The helper request is too large.".into());
    }
    writer
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .and_then(|_| writer.write_all(&bytes))
        .map_err(|_| {
            "The privileged helper disconnected. Reopen Warply or use Restore internet.".into()
        })
}

fn read_frame(reader: &mut impl std::io::Read) -> Result<Zeroizing<Vec<u8>>, String> {
    let mut header = [0; 4];
    reader.read_exact(&mut header).map_err(|_| {
        "The privileged helper disconnected. Reopen Warply or use Restore internet."
    })?;
    let size = u32::from_le_bytes(header) as usize;
    if size == 0 || size > MAX_FRAME {
        return Err("Invalid helper frame size.".into());
    }
    let mut bytes = Zeroizing::new(vec![0; size]);
    reader
        .read_exact(&mut bytes)
        .map_err(|_| "Incomplete helper response.")?;
    Ok(bytes)
}

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use std::{
        io::{Read, Write},
        ptr,
        sync::{
            atomic::{AtomicBool, Ordering},
            Mutex, OnceLock,
        },
    };
    use windows_sys::Win32::{
        Foundation::*,
        Security::{Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW, *},
        Storage::FileSystem::*,
        System::{Com::CoCreateGuid, Pipes::*, Threading::*, IO::*},
        UI::Shell::{IsUserAnAdmin, ShellExecuteExW, SHELLEXECUTEINFOW},
    };
    pub struct Handle(HANDLE);
    unsafe impl Send for Handle {}
    impl Drop for Handle {
        fn drop(&mut self) {
            if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
                unsafe {
                    CloseHandle(self.0);
                }
            }
        }
    }
    fn event() -> std::io::Result<Handle> {
        let handle = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if handle.is_null() {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(Handle(handle))
        }
    }
    pub struct Pipe(Handle, bool);
    impl Pipe {
        fn complete(
            &self,
            overlapped: &mut OVERLAPPED,
            started: bool,
            timeout: u32,
        ) -> std::io::Result<usize> {
            if !started && unsafe { GetLastError() } != ERROR_IO_PENDING {
                return Err(std::io::Error::last_os_error());
            }
            if unsafe { WaitForSingleObject(overlapped.hEvent, timeout) } != WAIT_OBJECT_0 {
                unsafe {
                    CancelIoEx(self.0 .0, overlapped);
                }
                let mut transferred = 0;
                unsafe {
                    GetOverlappedResult(self.0 .0, overlapped, &mut transferred, 1);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Helper timed out",
                ));
            }
            let mut transferred = 0;
            if unsafe { GetOverlappedResult(self.0 .0, overlapped, &mut transferred, 0) } == 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(transferred as usize)
        }
    }
    impl Read for Pipe {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let event = event()?;
            let mut overlapped = OVERLAPPED {
                hEvent: event.0,
                ..Default::default()
            };
            let started = unsafe {
                ReadFile(
                    self.0 .0,
                    buffer.as_mut_ptr(),
                    buffer.len().min(u32::MAX as usize) as u32,
                    ptr::null_mut(),
                    &mut overlapped,
                )
            } != 0;
            self.complete(
                &mut overlapped,
                started,
                if self.1 { 180_000 } else { u32::MAX },
            )
        }
    }
    impl Write for Pipe {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            let event = event()?;
            let mut overlapped = OVERLAPPED {
                hEvent: event.0,
                ..Default::default()
            };
            let started = unsafe {
                WriteFile(
                    self.0 .0,
                    buffer.as_ptr(),
                    buffer.len().min(u32::MAX as usize) as u32,
                    ptr::null_mut(),
                    &mut overlapped,
                )
            } != 0;
            self.complete(&mut overlapped, started, 180_000)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    struct Client {
        pipe: Pipe,
        _process: Handle,
    }
    static CLIENT: OnceLock<Mutex<Option<Client>>> = OnceLock::new();
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static KNOWN: AtomicBool = AtomicBool::new(false);
    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(Some(0)).collect()
    }
    fn same_image(pid: u32, elevated: bool) -> Result<(), String> {
        let process = Handle(unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) });
        if process.0.is_null() {
            return Err("Could not authenticate the helper process.".into());
        }
        let mut path = vec![0; 32768];
        let mut size = path.len() as u32;
        if unsafe { QueryFullProcessImageNameW(process.0, 0, path.as_mut_ptr(), &mut size) } == 0 {
            return Err("Could not authenticate the helper image.".into());
        }
        let image = std::path::PathBuf::from(String::from_utf16_lossy(&path[..size as usize]));
        let current = std::env::current_exe()
            .and_then(std::fs::canonicalize)
            .map_err(|_| "Could not authenticate Warply.")?;
        if std::fs::canonicalize(image).map_err(|_| "Could not authenticate the helper image.")?
            != current
        {
            return Err("Unexpected helper image. Nothing was sent.".into());
        }
        // The parent authenticates an exact live PID returned by ShellExecuteExW
        // with verb runas; the helper also requires IsUserAnAdmin before connecting.
        // Avoid reading a different administrator account's token from a standard GUI.
        if elevated {
            return Ok(());
        }
        let mut token = ptr::null_mut();
        if unsafe { OpenProcessToken(process.0, TOKEN_QUERY, &mut token) } == 0 {
            return Err("Could not authenticate helper privileges.".into());
        }
        let token = Handle(token);
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0;
        if unsafe {
            GetTokenInformation(
                token.0,
                TokenElevation,
                (&mut elevation as *mut TOKEN_ELEVATION).cast(),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut size,
            )
        } == 0
            || (elevation.TokenIsElevated != 0) != elevated
        {
            return Err("Unexpected process privileges. Nothing was sent.".into());
        }
        Ok(())
    }
    fn start() -> Result<Client, String> {
        let pid = std::process::id();
        let mut nonce = windows_sys::core::GUID::default();
        if unsafe { CoCreateGuid(&mut nonce) } < 0 {
            return Err("Could not create a private helper channel.".into());
        }
        let name = format!(
            r"\\.\pipe\Warply-{pid}-{:08x}{:04x}{:04x}{:02x?}",
            nonce.data1, nonce.data2, nonce.data3, nonce.data4
        )
        .replace(['[', ']', ',', ' '], "");
        let name_wide = wide(&name);
        let sddl = wide("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)");
        let mut sd = ptr::null_mut();
        if unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                1,
                &mut sd,
                ptr::null_mut(),
            )
        } == 0
        {
            return Err("Could not secure the helper channel.".into());
        }
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        let pipe = Pipe(
            Handle(unsafe {
                CreateNamedPipeW(
                    name_wide.as_ptr(),
                    PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED | FILE_FLAG_FIRST_PIPE_INSTANCE,
                    PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                    1,
                    MAX_FRAME as u32,
                    MAX_FRAME as u32,
                    0,
                    &attributes,
                )
            }),
            true,
        );
        unsafe {
            LocalFree(sd);
        }
        if pipe.0 .0 == INVALID_HANDLE_VALUE {
            return Err("Could not open a private helper channel.".into());
        }
        let event = event().map_err(|_| "Could not wait for the helper.")?;
        let mut overlapped = OVERLAPPED {
            hEvent: event.0,
            ..Default::default()
        };
        let connected = unsafe { ConnectNamedPipe(pipe.0 .0, &mut overlapped) };
        if connected == 0 && unsafe { GetLastError() } != ERROR_IO_PENDING {
            return Err("Could not prepare the helper connection.".into());
        }
        let executable = std::env::current_exe().map_err(|_| "Could not locate the helper.")?;
        use std::os::windows::ffi::OsStrExt;
        let file: Vec<u16> = executable
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let verb = wide("runas");
        let arguments = wide(&format!("--privileged-helper --parent {pid} --pipe {name}"));
        let mut info = SHELLEXECUTEINFOW {
            cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
            fMask: 0x40,
            lpVerb: verb.as_ptr(),
            lpFile: file.as_ptr(),
            lpParameters: arguments.as_ptr(),
            nShow: 0,
            ..Default::default()
        };
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            unsafe {
                CancelIoEx(pipe.0 .0, &overlapped);
                let mut transferred = 0;
                GetOverlappedResult(pipe.0 .0, &overlapped, &mut transferred, 1);
            }
            return Err("Approve the Windows prompt to let Warply manage the tunnel. Select Try again to retry.".into());
        }
        let process = Handle(info.hProcess);
        pipe.complete(&mut overlapped, true, 60_000)
            .map_err(|_| "The privileged helper did not start.")?;
        let mut client_pid = 0;
        if unsafe { GetNamedPipeClientProcessId(pipe.0 .0, &mut client_pid) } == 0
            || client_pid != unsafe { GetProcessId(process.0) }
        {
            return Err("The helper channel was not authenticated. Nothing was sent.".into());
        }
        same_image(client_pid, true)?;
        Ok(Client {
            pipe,
            _process: process,
        })
    }
    pub(super) fn request(
        request: Request,
        allow_start: bool,
    ) -> Result<crate::guard::GuardView, String> {
        if !matches!(&request, Request::Inspect {}) {
            KNOWN.store(false, Ordering::SeqCst);
        }
        let mut client = CLIENT
            .get_or_init(|| Mutex::new(None))
            .lock()
            .map_err(|_| "The helper is unavailable.")?;
        if client.is_none() {
            if !allow_start {
                KNOWN.store(false, Ordering::SeqCst);
                return Err("The privileged helper stopped. Select Connect or Restore internet to restart it.".into());
            }
            *client = Some(start()?);
        }
        let Some(session) = client.as_mut() else {
            return Err("The helper is unavailable.".into());
        };
        if let Err(error) = write_frame(&mut session.pipe, &request) {
            *client = None;
            KNOWN.store(false, Ordering::SeqCst);
            return Err(error);
        }
        let bytes = match read_frame(&mut session.pipe) {
            Ok(bytes) => bytes,
            Err(error) => {
                *client = None;
                KNOWN.store(false, Ordering::SeqCst);
                return Err(error);
            }
        };
        let reply: Reply = match serde_json::from_slice(&bytes) {
            Ok(reply) => reply,
            Err(_) => {
                *client = None;
                KNOWN.store(false, Ordering::SeqCst);
                return Err("Invalid helper response.".into());
            }
        };
        ACTIVE.store(reply.guard.active, Ordering::SeqCst);
        KNOWN.store(reply.guard.known, Ordering::SeqCst);
        if let Some(error) = reply.error {
            return Err(error);
        }
        Ok(reply.guard)
    }
    pub fn cached() -> crate::guard::GuardView {
        crate::guard::GuardView {
            active: ACTIVE.load(Ordering::SeqCst),
            known: KNOWN.load(Ordering::SeqCst),
        }
    }
    fn handle(request: Request) -> Result<(), String> {
        match request {
            Request::Inspect {} => {
                crate::network_name::maintain(false);
                Ok(())
            }
            Request::Reconcile {} => {
                if crate::guard::view()?.active
                    && crate::tunnel::backend()
                        .status()
                        .map_err(|error| error.to_string())?
                        == crate::tunnel::TunnelStatus::Connected
                {
                    crate::guard::enable(Some(crate::diagnostics::tunnel_luid()?))?;
                }
                Ok(())
            }
            Request::ValidateProtection {} => {
                crate::storage::machine_folder()?;
                crate::guard::validate()
            }
            Request::Connect {
                mut profile,
                protect,
            } => {
                let profile = Zeroizing::new(std::mem::take(&mut profile.0));
                crate::config::validate_import(&profile).map_err(|error| error.to_string())?;
                crate::diagnostics::require_no_conflict()?;
                let active = crate::guard::view()?.active;
                let resolved = crate::network::resolve_endpoint(&profile, active)?;
                let path = crate::storage::machine_service_profile(&resolved)?;
                if protect || active {
                    crate::guard::enable(None)?;
                }
                crate::tunnel::backend()
                    .connect(&path)
                    .map_err(|error| error.to_string())?;
                if protect || active {
                    let luid = crate::diagnostics::tunnel_luid()?;
                    crate::guard::enable(Some(luid))?;
                }
                crate::network_name::maintain(true);
                Ok(())
            }
            Request::Stop { unlock } => {
                crate::tunnel::backend()
                    .disconnect()
                    .map_err(|error| error.to_string())?;
                if unlock {
                    crate::guard::disable()?;
                }
                if unlock {
                    crate::storage::remove_machine_service_profile()?;
                }
                Ok(())
            }
            Request::Repair {} => crate::cleanup::run().map_err(|error| error.to_string()),
            Request::InstallWireguard {} => {
                tauri::async_runtime::block_on(crate::installer::install())
            }
        }
    }
    pub fn run() -> bool {
        let args: Vec<String> = std::env::args().collect();
        if args.len() == 2 && args[1] == "--diagnostics" {
            let result = tauri::async_runtime::block_on(crate::diagnostics::report(
                &crate::setup::AppState::default(),
            ));
            match result {
                Ok(report) => {
                    println!("{report}");
                    std::process::exit(0);
                }
                Err(_) => std::process::exit(1),
            }
        }
        if args.len() == 2 && args[1] == "--validate-protection" {
            let result = if unsafe { IsUserAnAdmin() } != 0 {
                crate::storage::machine_folder().and_then(|_| crate::guard::validate())
            } else {
                super::send(Request::ValidateProtection {}, true).map(|_| ())
            };
            if let Err(error) = &result {
                eprintln!("{error}");
            }
            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
        if args.len() == 2 && args[1] == "--cleanup-for-uninstall" {
            // NSIS per-machine uninstall is already elevated. Never open GUI/UAC here.
            if unsafe { IsUserAnAdmin() } == 0 {
                std::process::exit(23);
            }
            std::process::exit(match crate::cleanup::run() {
                Ok(()) => 0,
                Err(error) => error.exit_code(),
            });
        }
        if args.len() == 2 && args[1] == "--restore-internet" {
            let result = if unsafe { IsUserAnAdmin() } != 0 {
                handle(Request::Repair {})
            } else {
                super::repair()
            };
            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
        if args.len() == 2 && args[1] == "--migrate-startup" {
            let result = if unsafe { IsUserAnAdmin() } != 0 {
                crate::autostart::migrate()
            } else {
                Err("Administrator required.".into())
            };
            std::process::exit(if result.is_ok() { 0 } else { 1 });
        }
        if args.get(1).map(String::as_str) != Some("--privileged-helper") {
            return false;
        }
        let valid = args.len() == 6 && args[2] == "--parent" && args[4] == "--pipe";
        let result = (|| {
            if !valid || unsafe { IsUserAnAdmin() } == 0 {
                return Err("Invalid helper invocation.".to_string());
            }
            let pid = args[3].parse::<u32>().map_err(|_| "Invalid parent.")?;
            let prefix = format!(r"\\.\pipe\Warply-{pid}-");
            if !args[5].starts_with(&prefix)
                || !args[5][prefix.len()..]
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit())
                || args[5].len() != prefix.len() + 32
            {
                return Err("Invalid helper channel.".into());
            }
            same_image(pid, false)?;
            let name = wide(&args[5]);
            let mut pipe = Pipe(
                Handle(unsafe {
                    CreateFileW(
                        name.as_ptr(),
                        GENERIC_READ | GENERIC_WRITE,
                        0,
                        ptr::null(),
                        OPEN_EXISTING,
                        FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                        ptr::null_mut(),
                    )
                }),
                false,
            );
            if pipe.0 .0 == INVALID_HANDLE_VALUE {
                return Err("Could not connect to Warply.".into());
            }
            let mut server_pid = 0;
            if unsafe { GetNamedPipeServerProcessId(pipe.0 .0, &mut server_pid) } == 0
                || server_pid != pid
            {
                return Err("Unexpected helper server.".into());
            }
            same_image(server_pid, false)?;
            while let Ok(bytes) = read_frame(&mut pipe) {
                let request: Request =
                    serde_json::from_slice(&bytes).map_err(|_| "Invalid helper request.")?;
                let result = handle(request);
                let guard = crate::guard::view();
                let reply = Reply {
                    guard: guard.clone().unwrap_or(crate::guard::GuardView {
                        active: true,
                        known: false,
                    }),
                    error: result.err().or_else(|| guard.err()),
                };
                if write_frame(&mut pipe, &reply).is_err() {
                    break;
                }
            }
            Ok::<_, String>(())
        })();
        let _ = result; // No profile or protocol data is logged or displayed.
        true
    }
}

#[cfg(target_os = "windows")]
pub use native::{cached, run};
#[cfg(target_os = "windows")]
fn send(request: Request, allow_start: bool) -> Result<crate::guard::GuardView, String> {
    native::request(request, allow_start)
}

pub fn initialize() -> Result<crate::guard::GuardView, String> {
    send(Request::Reconcile {}, true)
}
pub fn connect(profile: &str, protect: bool) -> Result<(), String> {
    send(
        Request::Connect {
            profile: Secret(profile.into()),
            protect,
        },
        false,
    )
    .map(|_| ())
}
pub fn stop(unlock: bool) -> Result<(), String> {
    send(Request::Stop { unlock }, unlock).map(|_| ())
}
pub fn repair() -> Result<(), String> {
    send(Request::Repair {}, true).map(|_| ())
}
pub fn install_wireguard() -> Result<(), String> {
    send(Request::InstallWireguard {}, true).map(|_| ())
}

pub fn refresh() -> Result<crate::guard::GuardView, String> {
    send(Request::Inspect {}, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_oversized_frames_and_arbitrary_helper_operations() {
        assert!(read_frame(&mut ((MAX_FRAME as u32 + 1).to_le_bytes().as_slice())).is_err());
        assert!(
            serde_json::from_str::<Request>(r#"{"operation":"Run","path":"cmd.exe"}"#).is_err()
        );
        assert!(
            serde_json::from_str::<Request>(r#"{"operation":"Repair","path":"C:\\other"}"#)
                .is_err()
        );
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &Request::Inspect {}).expect("frame");
        assert!(serde_json::from_slice::<Request>(
            &read_frame(&mut bytes.as_slice()).expect("frame")
        )
        .is_ok());
    }
}
