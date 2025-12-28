//! OS Module - Node.js compatible operating system utilities
//!
//! Native Rust implementation for maximum performance. Provides:
//! - os.EOL - Platform-specific line ending
//! - os.arch() - CPU architecture
//! - os.platform() - Operating system platform
//! - os.type() - Operating system name
//! - os.release() - Operating system release
//! - os.version() - Operating system version/kernel
//! - os.machine() - Machine type
//! - os.hostname() - System hostname
//! - os.homedir() - User home directory
//! - os.tmpdir() - Temporary directory
//! - os.devNull - Null device path
//! - os.cpus() - CPU information
//! - os.freemem() - Free system memory
//! - os.totalmem() - Total system memory
//! - os.uptime() - System uptime
//! - os.loadavg() - Load averages (Unix only)
//! - os.networkInterfaces() - Network interface information
//! - os.userInfo() - Current user information
//! - os.endianness() - CPU endianness
//! - os.availableParallelism() - Available CPU parallelism
//! - os.getPriority() / os.setPriority() - Process priority
//! - os.constants - OS constants (signals, errno, priority)

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, object::builtins::JsArray, property::Attribute,
};

#[cfg(windows)]
use windows_sys::Win32::{
    Foundation::{CloseHandle, FILETIME},
    NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_PREFIX, GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_MULTICAST,
        GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH, IP_ADAPTER_UNICAST_ADDRESS_LH,
    },
    Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6},
    System::{
        SystemInformation::{
            ComputerNameDnsHostname, GetComputerNameExW, GetSystemInfo, GetTickCount64,
            GlobalMemoryStatusEx, MEMORYSTATUSEX, OSVERSIONINFOEXW, SYSTEM_INFO,
        },
        Threading::{
            ABOVE_NORMAL_PRIORITY_CLASS, BELOW_NORMAL_PRIORITY_CLASS, GetCurrentProcess,
            GetPriorityClass, HIGH_PRIORITY_CLASS, IDLE_PRIORITY_CLASS, NORMAL_PRIORITY_CLASS,
            REALTIME_PRIORITY_CLASS, SetPriorityClass,
        },
    },
};

#[cfg(windows)]
use windows_sys::Win32::System::WindowsProgramming::GetUserNameW;

#[cfg(windows)]
use windows_sys::Win32::System::Threading::GetSystemTimes;

/// Register the OS module
pub fn register_os_module(context: &mut Context) -> JsResult<()> {
    // Register all native functions first
    register_native_os_functions(context)?;

    // Create the os module object in JavaScript
    let os_code = r#"
        (function() {
            // Platform-specific EOL
            const EOL = __VIPER_PLATFORM__ === 'win32' ? '\r\n' : '\n';

            // Null device path
            const devNull = __VIPER_PLATFORM__ === 'win32' ? '\\\\.\\nul' : '/dev/null';

            // Create the os object
            const os = {
                // Constants
                EOL: EOL,
                devNull: devNull,

                // Functions
                arch: () => __viper_os_arch(),
                platform: () => __viper_os_platform(),
                type: () => __viper_os_type(),
                release: () => __viper_os_release(),
                version: () => __viper_os_version(),
                machine: () => __viper_os_machine(),
                hostname: () => __viper_os_hostname(),
                homedir: () => __viper_os_homedir(),
                tmpdir: () => __viper_os_tmpdir(),
                cpus: () => __viper_os_cpus(),
                freemem: () => __viper_os_freemem(),
                totalmem: () => __viper_os_totalmem(),
                uptime: () => __viper_os_uptime(),
                loadavg: () => __viper_os_loadavg(),
                networkInterfaces: () => __viper_os_network_interfaces(),
                userInfo: (options) => __viper_os_userinfo(options),
                endianness: () => __viper_os_endianness(),
                availableParallelism: () => __viper_os_available_parallelism(),
                getPriority: (pid) => __viper_os_get_priority(pid),
                setPriority: (pid, priority) => {
                    if (typeof pid === 'number' && priority === undefined) {
                        // setPriority(priority) - set current process
                        return __viper_os_set_priority(0, pid);
                    }
                    return __viper_os_set_priority(pid || 0, priority);
                },

                // Constants object
                constants: __viper_os_constants(),
            };

            // Make os global
            globalThis.os = os;

            return os;
        })();
    "#;

    let source = Source::from_bytes(os_code.as_bytes());
    context.eval(source)?;

    Ok(())
}

/// Register all native OS functions
fn register_native_os_functions(context: &mut Context) -> JsResult<()> {
    let global = context.global_object();

    // os.arch()
    let arch_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let arch = if cfg!(target_arch = "x86_64") {
            "x64"
        } else if cfg!(target_arch = "aarch64") {
            "arm64"
        } else if cfg!(target_arch = "x86") {
            "ia32"
        } else if cfg!(target_arch = "arm") {
            "arm"
        } else if cfg!(target_arch = "mips") {
            "mips"
        } else if cfg!(target_arch = "mips64") {
            "mips64"
        } else if cfg!(target_arch = "powerpc64") {
            "ppc64"
        } else if cfg!(target_arch = "s390x") {
            "s390x"
        } else if cfg!(target_arch = "riscv64") {
            "riscv64"
        } else if cfg!(target_arch = "loongarch64") {
            "loong64"
        } else {
            "unknown"
        };
        Ok(JsValue::from(js_string!(arch)))
    });
    global.set(
        js_string!("__viper_os_arch"),
        arch_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.platform()
    let platform_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let platform = if cfg!(target_os = "windows") {
            "win32"
        } else if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else if cfg!(target_os = "freebsd") {
            "freebsd"
        } else if cfg!(target_os = "openbsd") {
            "openbsd"
        } else if cfg!(target_os = "netbsd") {
            "netbsd"
        } else if cfg!(target_os = "android") {
            "android"
        } else if cfg!(target_os = "ios") {
            "darwin"
        } else if cfg!(target_os = "solaris") {
            "sunos"
        } else if cfg!(target_os = "aix") {
            "aix"
        } else {
            "unknown"
        };
        Ok(JsValue::from(js_string!(platform)))
    });
    global.set(
        js_string!("__viper_os_platform"),
        platform_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.type()
    let type_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let os_type = if cfg!(target_os = "windows") {
            "Windows_NT"
        } else if cfg!(target_os = "macos") || cfg!(target_os = "ios") {
            "Darwin"
        } else if cfg!(target_os = "linux") || cfg!(target_os = "android") {
            "Linux"
        } else if cfg!(target_os = "freebsd") {
            "FreeBSD"
        } else if cfg!(target_os = "openbsd") {
            "OpenBSD"
        } else if cfg!(target_os = "netbsd") {
            "NetBSD"
        } else if cfg!(target_os = "solaris") {
            "SunOS"
        } else if cfg!(target_os = "aix") {
            "AIX"
        } else {
            "Unknown"
        };
        Ok(JsValue::from(js_string!(os_type)))
    });
    global.set(
        js_string!("__viper_os_type"),
        type_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.release()
    let release_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let release = get_os_release();
        Ok(JsValue::from(js_string!(release)))
    });
    global.set(
        js_string!("__viper_os_release"),
        release_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.version()
    let version_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let version = get_os_version();
        Ok(JsValue::from(js_string!(version)))
    });
    global.set(
        js_string!("__viper_os_version"),
        version_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.machine()
    let machine_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let machine = if cfg!(target_arch = "x86_64") {
            "x86_64"
        } else if cfg!(target_arch = "aarch64") {
            "aarch64"
        } else if cfg!(target_arch = "x86") {
            "i686"
        } else if cfg!(target_arch = "arm") {
            "arm"
        } else if cfg!(target_arch = "mips") {
            "mips"
        } else if cfg!(target_arch = "mips64") {
            "mips64"
        } else if cfg!(target_arch = "powerpc64") {
            "ppc64le"
        } else if cfg!(target_arch = "s390x") {
            "s390x"
        } else if cfg!(target_arch = "riscv64") {
            "riscv64"
        } else {
            "unknown"
        };
        Ok(JsValue::from(js_string!(machine)))
    });
    global.set(
        js_string!("__viper_os_machine"),
        machine_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.hostname()
    let hostname_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let hostname = get_hostname();
        Ok(JsValue::from(js_string!(hostname)))
    });
    global.set(
        js_string!("__viper_os_hostname"),
        hostname_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.homedir()
    let homedir_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let homedir = get_homedir();
        Ok(JsValue::from(js_string!(homedir)))
    });
    global.set(
        js_string!("__viper_os_homedir"),
        homedir_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.tmpdir()
    let tmpdir_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let tmpdir = get_tmpdir();
        Ok(JsValue::from(js_string!(tmpdir)))
    });
    global.set(
        js_string!("__viper_os_tmpdir"),
        tmpdir_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.cpus()
    let cpus_fn = NativeFunction::from_fn_ptr(|_this, _args, context| get_cpus(context));
    global.set(
        js_string!("__viper_os_cpus"),
        cpus_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.freemem()
    let freemem_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let free = get_freemem();
        Ok(JsValue::from(free as f64))
    });
    global.set(
        js_string!("__viper_os_freemem"),
        freemem_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.totalmem()
    let totalmem_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let total = get_totalmem();
        Ok(JsValue::from(total as f64))
    });
    global.set(
        js_string!("__viper_os_totalmem"),
        totalmem_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.uptime()
    let uptime_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let uptime = get_uptime();
        Ok(JsValue::from(uptime))
    });
    global.set(
        js_string!("__viper_os_uptime"),
        uptime_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.loadavg()
    let loadavg_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        let (one, five, fifteen) = get_loadavg();
        let arr = JsArray::new(context);
        arr.push(JsValue::from(one), context)?;
        arr.push(JsValue::from(five), context)?;
        arr.push(JsValue::from(fifteen), context)?;
        Ok(arr.into())
    });
    global.set(
        js_string!("__viper_os_loadavg"),
        loadavg_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.networkInterfaces()
    let network_fn =
        NativeFunction::from_fn_ptr(|_this, _args, context| get_network_interfaces(context));
    global.set(
        js_string!("__viper_os_network_interfaces"),
        network_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.userInfo()
    let userinfo_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let encoding = args
            .get(0)
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get(js_string!("encoding"), context).ok())
            .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
            .unwrap_or_else(|| "utf8".to_string());

        get_userinfo(context, &encoding)
    });
    global.set(
        js_string!("__viper_os_userinfo"),
        userinfo_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.endianness()
    let endianness_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let endian = if cfg!(target_endian = "little") {
            "LE"
        } else {
            "BE"
        };
        Ok(JsValue::from(js_string!(endian)))
    });
    global.set(
        js_string!("__viper_os_endianness"),
        endianness_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.availableParallelism()
    let parallelism_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        let count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        Ok(JsValue::from(count as i32))
    });
    global.set(
        js_string!("__viper_os_available_parallelism"),
        parallelism_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.getPriority(pid)
    let get_priority_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let pid = args
            .get(0)
            .map(|v| v.to_i32(context))
            .transpose()?
            .unwrap_or(0);

        get_priority(pid)
    });
    global.set(
        js_string!("__viper_os_get_priority"),
        get_priority_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.setPriority(pid, priority)
    let set_priority_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let pid = args
            .get(0)
            .map(|v| v.to_i32(context))
            .transpose()?
            .unwrap_or(0);
        let priority = args
            .get(1)
            .ok_or_else(|| JsNativeError::typ().with_message("priority required"))?
            .to_i32(context)?;

        set_priority(pid, priority)
    });
    global.set(
        js_string!("__viper_os_set_priority"),
        set_priority_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // os.constants
    let constants_fn =
        NativeFunction::from_fn_ptr(|_this, _args, context| build_os_constants(context));
    global.set(
        js_string!("__viper_os_constants"),
        constants_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    Ok(())
}

// ============================================================================
// Platform-specific implementations
// ============================================================================

/// Get OS release string
fn get_os_release() -> String {
    #[cfg(unix)]
    {
        use std::ffi::CStr;
        use std::mem::MaybeUninit;

        unsafe {
            let mut info = MaybeUninit::<libc::utsname>::uninit();
            if libc::uname(info.as_mut_ptr()) == 0 {
                let info = info.assume_init();
                CStr::from_ptr(info.release.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            } else {
                "0.0.0".to_string()
            }
        }
    }

    #[cfg(windows)]
    {
        // Use RtlGetVersion for accurate Windows version
        use std::mem::MaybeUninit;

        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOEXW) -> i32;
        }

        unsafe {
            let mut info = MaybeUninit::<OSVERSIONINFOEXW>::uninit();
            let ptr = info.as_mut_ptr();
            (*ptr).dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;

            if RtlGetVersion(ptr) == 0 {
                let info = info.assume_init();
                format!(
                    "{}.{}.{}",
                    info.dwMajorVersion, info.dwMinorVersion, info.dwBuildNumber
                )
            } else {
                "0.0.0".to_string()
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        "0.0.0".to_string()
    }
}

/// Get OS version string
fn get_os_version() -> String {
    #[cfg(unix)]
    {
        use std::ffi::CStr;
        use std::mem::MaybeUninit;

        unsafe {
            let mut info = MaybeUninit::<libc::utsname>::uninit();
            if libc::uname(info.as_mut_ptr()) == 0 {
                let info = info.assume_init();
                CStr::from_ptr(info.version.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            } else {
                "".to_string()
            }
        }
    }

    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;

        #[link(name = "ntdll")]
        unsafe extern "system" {
            fn RtlGetVersion(lpVersionInformation: *mut OSVERSIONINFOEXW) -> i32;
        }

        unsafe {
            let mut info = MaybeUninit::<OSVERSIONINFOEXW>::uninit();
            let ptr = info.as_mut_ptr();
            (*ptr).dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;

            if RtlGetVersion(ptr) == 0 {
                let info = info.assume_init();
                format!(
                    "Windows {} (Version {} Build {})",
                    if info.dwMajorVersion >= 10 {
                        if info.dwBuildNumber >= 22000 {
                            "11"
                        } else {
                            "10"
                        }
                    } else {
                        "NT"
                    },
                    format!("{}.{}", info.dwMajorVersion, info.dwMinorVersion),
                    info.dwBuildNumber
                )
            } else {
                "Windows".to_string()
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        "".to_string()
    }
}

/// Get hostname
fn get_hostname() -> String {
    #[cfg(unix)]
    {
        use std::ffi::CStr;

        let mut buf = [0i8; 256];
        unsafe {
            if libc::gethostname(buf.as_mut_ptr(), buf.len()) == 0 {
                CStr::from_ptr(buf.as_ptr()).to_string_lossy().into_owned()
            } else {
                "localhost".to_string()
            }
        }
    }

    #[cfg(windows)]
    {
        let mut size: u32 = 0;
        unsafe {
            GetComputerNameExW(ComputerNameDnsHostname, std::ptr::null_mut(), &mut size);
            if size == 0 {
                return "localhost".to_string();
            }

            let mut buf = vec![0u16; size as usize];
            if GetComputerNameExW(ComputerNameDnsHostname, buf.as_mut_ptr(), &mut size) != 0 {
                String::from_utf16_lossy(&buf[..size as usize])
            } else {
                "localhost".to_string()
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        "localhost".to_string()
    }
}

/// Get home directory
fn get_homedir() -> String {
    // Try environment variables first
    if let Ok(home) = std::env::var("HOME") {
        return home;
    }

    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return profile;
        }

        // Fallback to HOMEDRIVE + HOMEPATH
        if let (Ok(drive), Ok(path)) = (std::env::var("HOMEDRIVE"), std::env::var("HOMEPATH")) {
            return format!("{}{}", drive, path);
        }
    }

    #[cfg(unix)]
    {
        // Try passwd entry
        unsafe {
            let uid = libc::getuid();
            let pwd = libc::getpwuid(uid);
            if !pwd.is_null() {
                let dir = (*pwd).pw_dir;
                if !dir.is_null() {
                    return std::ffi::CStr::from_ptr(dir).to_string_lossy().into_owned();
                }
            }
        }
    }

    // Last resort fallback
    if cfg!(windows) {
        "C:\\Users\\Default".to_string()
    } else {
        "/".to_string()
    }
}

/// Get temp directory
fn get_tmpdir() -> String {
    // Check environment variables in order
    #[cfg(windows)]
    {
        if let Ok(temp) = std::env::var("TEMP") {
            return temp;
        }
        if let Ok(tmp) = std::env::var("TMP") {
            return tmp;
        }
        // Windows defaults
        if let Ok(root) = std::env::var("SystemRoot") {
            return format!("{}\\temp", root);
        }
        if let Ok(windir) = std::env::var("windir") {
            return format!("{}\\temp", windir);
        }
        return "C:\\Windows\\Temp".to_string();
    }

    #[cfg(not(windows))]
    {
        if let Ok(tmpdir) = std::env::var("TMPDIR") {
            return tmpdir;
        }
        if let Ok(tmp) = std::env::var("TMP") {
            return tmp;
        }
        if let Ok(temp) = std::env::var("TEMP") {
            return temp;
        }
        "/tmp".to_string()
    }
}

/// Get CPU information
fn get_cpus(context: &mut Context) -> JsResult<JsValue> {
    let arr = JsArray::new(context);

    #[cfg(unix)]
    {
        // Read /proc/cpuinfo on Linux
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
                let mut model = String::new();
                let mut speed = 0u32;
                let mut cpu_count = 0;

                for line in content.lines() {
                    if line.starts_with("model name") {
                        if let Some(val) = line.split(':').nth(1) {
                            model = val.trim().to_string();
                        }
                    } else if line.starts_with("cpu MHz") {
                        if let Some(val) = line.split(':').nth(1) {
                            if let Ok(mhz) = val.trim().parse::<f64>() {
                                speed = mhz as u32;
                            }
                        }
                    } else if line.starts_with("processor") {
                        if cpu_count > 0 {
                            // Push previous CPU
                            let cpu_obj = ObjectInitializer::new(context).build();
                            cpu_obj.set(
                                js_string!("model"),
                                JsValue::from(js_string!(model.clone())),
                                false,
                                context,
                            )?;
                            cpu_obj.set(
                                js_string!("speed"),
                                JsValue::from(speed),
                                false,
                                context,
                            )?;

                            let times = ObjectInitializer::new(context).build();
                            times.set(js_string!("user"), JsValue::from(0), false, context)?;
                            times.set(js_string!("nice"), JsValue::from(0), false, context)?;
                            times.set(js_string!("sys"), JsValue::from(0), false, context)?;
                            times.set(js_string!("idle"), JsValue::from(0), false, context)?;
                            times.set(js_string!("irq"), JsValue::from(0), false, context)?;
                            cpu_obj.set(js_string!("times"), times, false, context)?;

                            arr.push(cpu_obj, context)?;
                        }
                        cpu_count += 1;
                    }
                }

                // Push last CPU
                if cpu_count > 0 {
                    let cpu_obj = ObjectInitializer::new(context).build();
                    cpu_obj.set(
                        js_string!("model"),
                        JsValue::from(js_string!(model)),
                        false,
                        context,
                    )?;
                    cpu_obj.set(js_string!("speed"), JsValue::from(speed), false, context)?;

                    let times = ObjectInitializer::new(context).build();
                    times.set(js_string!("user"), JsValue::from(0), false, context)?;
                    times.set(js_string!("nice"), JsValue::from(0), false, context)?;
                    times.set(js_string!("sys"), JsValue::from(0), false, context)?;
                    times.set(js_string!("idle"), JsValue::from(0), false, context)?;
                    times.set(js_string!("irq"), JsValue::from(0), false, context)?;
                    cpu_obj.set(js_string!("times"), times, false, context)?;

                    arr.push(cpu_obj, context)?;
                }

                // Read CPU times from /proc/stat
                if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
                    let mut cpu_idx = 0usize;
                    for line in stat.lines() {
                        if line.starts_with("cpu") && !line.starts_with("cpu ") {
                            let parts: Vec<&str> = line.split_whitespace().collect();
                            if parts.len() >= 8 {
                                if let Some(cpu_val) = arr.get(cpu_idx, context)?.as_object() {
                                    if let Some(times_val) =
                                        cpu_val.get(js_string!("times"), context)?.as_object()
                                    {
                                        let user: u64 = parts[1].parse().unwrap_or(0);
                                        let nice: u64 = parts[2].parse().unwrap_or(0);
                                        let sys: u64 = parts[3].parse().unwrap_or(0);
                                        let idle: u64 = parts[4].parse().unwrap_or(0);
                                        let irq: u64 = parts[6].parse().unwrap_or(0);

                                        times_val.set(
                                            js_string!("user"),
                                            JsValue::from((user * 10) as f64),
                                            false,
                                            context,
                                        )?;
                                        times_val.set(
                                            js_string!("nice"),
                                            JsValue::from((nice * 10) as f64),
                                            false,
                                            context,
                                        )?;
                                        times_val.set(
                                            js_string!("sys"),
                                            JsValue::from((sys * 10) as f64),
                                            false,
                                            context,
                                        )?;
                                        times_val.set(
                                            js_string!("idle"),
                                            JsValue::from((idle * 10) as f64),
                                            false,
                                            context,
                                        )?;
                                        times_val.set(
                                            js_string!("irq"),
                                            JsValue::from((irq * 10) as f64),
                                            false,
                                            context,
                                        )?;
                                    }
                                }
                                cpu_idx += 1;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS: Use sysctl
            use std::process::Command;

            let cpu_count = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);

            // Get CPU brand string
            let model = Command::new("sysctl")
                .args(["-n", "machdep.cpu.brand_string"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            // Get CPU frequency (in Hz, convert to MHz)
            let speed: u32 = Command::new("sysctl")
                .args(["-n", "hw.cpufrequency"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<u64>().ok())
                .map(|hz| (hz / 1_000_000) as u32)
                .unwrap_or(0);

            for _ in 0..cpu_count {
                let cpu_obj = ObjectInitializer::new(context).build();
                cpu_obj.set(
                    js_string!("model"),
                    JsValue::from(js_string!(model.clone())),
                    false,
                    context,
                )?;
                cpu_obj.set(js_string!("speed"), JsValue::from(speed), false, context)?;

                let times = ObjectInitializer::new(context).build();
                times.set(js_string!("user"), JsValue::from(0), false, context)?;
                times.set(js_string!("nice"), JsValue::from(0), false, context)?;
                times.set(js_string!("sys"), JsValue::from(0), false, context)?;
                times.set(js_string!("idle"), JsValue::from(0), false, context)?;
                times.set(js_string!("irq"), JsValue::from(0), false, context)?;
                cpu_obj.set(js_string!("times"), times, false, context)?;

                arr.push(cpu_obj, context)?;
            }
        }
    }

    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;

        // Get CPU count
        let mut sys_info = MaybeUninit::<SYSTEM_INFO>::uninit();
        unsafe {
            GetSystemInfo(sys_info.as_mut_ptr());
        }
        let cpu_count = unsafe { sys_info.assume_init().dwNumberOfProcessors };

        // Get CPU model from registry
        let model = get_windows_cpu_model();

        // Get CPU speed from registry
        let speed = get_windows_cpu_speed();

        // Get system times (idle, kernel, user)
        let (idle_time, kernel_time, user_time) = unsafe {
            let mut idle = MaybeUninit::<FILETIME>::uninit();
            let mut kernel = MaybeUninit::<FILETIME>::uninit();
            let mut user = MaybeUninit::<FILETIME>::uninit();

            if GetSystemTimes(idle.as_mut_ptr(), kernel.as_mut_ptr(), user.as_mut_ptr()) != 0 {
                let idle = idle.assume_init();
                let kernel = kernel.assume_init();
                let user = user.assume_init();

                // Convert FILETIME to u64 (100-nanosecond intervals)
                let idle_100ns = ((idle.dwHighDateTime as u64) << 32) | (idle.dwLowDateTime as u64);
                let kernel_100ns =
                    ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
                let user_100ns = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);

                // Convert to milliseconds and divide by CPU count for per-CPU times
                let idle_ms = idle_100ns / 10000 / cpu_count as u64;
                let kernel_ms = kernel_100ns / 10000 / cpu_count as u64;
                let user_ms = user_100ns / 10000 / cpu_count as u64;

                // Kernel time includes idle time, so subtract it
                let sys_ms = kernel_ms.saturating_sub(idle_ms);

                (idle_ms, sys_ms, user_ms)
            } else {
                (0u64, 0u64, 0u64)
            }
        };

        for _ in 0..cpu_count {
            let cpu_obj = ObjectInitializer::new(context).build();
            cpu_obj.set(
                js_string!("model"),
                JsValue::from(js_string!(model.clone())),
                false,
                context,
            )?;
            cpu_obj.set(js_string!("speed"), JsValue::from(speed), false, context)?;

            let times = ObjectInitializer::new(context).build();
            times.set(
                js_string!("user"),
                JsValue::from(user_time as f64),
                false,
                context,
            )?;
            times.set(js_string!("nice"), JsValue::from(0), false, context)?;
            times.set(
                js_string!("sys"),
                JsValue::from(kernel_time as f64),
                false,
                context,
            )?;
            times.set(
                js_string!("idle"),
                JsValue::from(idle_time as f64),
                false,
                context,
            )?;
            times.set(js_string!("irq"), JsValue::from(0), false, context)?;
            cpu_obj.set(js_string!("times"), times, false, context)?;

            arr.push(cpu_obj, context)?;
        }
    }

    Ok(arr.into())
}

#[cfg(windows)]
fn get_windows_cpu_model() -> String {
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, KEY_READ, REG_SZ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    unsafe {
        let subkey: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0\0"
            .encode_utf16()
            .collect();
        let value_name: Vec<u16> = "ProcessorNameString\0".encode_utf16().collect();

        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return "Unknown".to_string();
        }

        let mut data_type: u32 = 0;
        let mut data_size: u32 = 0;

        // Get size first
        RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            std::ptr::null_mut(),
            &mut data_size,
        );

        if data_size == 0 || data_type != REG_SZ {
            RegCloseKey(hkey);
            return "Unknown".to_string();
        }

        let mut buffer: Vec<u16> = vec![0; (data_size / 2) as usize];
        if RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            buffer.as_mut_ptr() as *mut u8,
            &mut data_size,
        ) == 0
        {
            RegCloseKey(hkey);
            // Remove null terminator
            while buffer.last() == Some(&0) {
                buffer.pop();
            }
            String::from_utf16_lossy(&buffer)
        } else {
            RegCloseKey(hkey);
            "Unknown".to_string()
        }
    }
}

#[cfg(windows)]
fn get_windows_cpu_speed() -> u32 {
    use windows_sys::Win32::System::Registry::{
        HKEY_LOCAL_MACHINE, KEY_READ, REG_DWORD, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
    };

    unsafe {
        let subkey: Vec<u16> = "HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0\0"
            .encode_utf16()
            .collect();
        let value_name: Vec<u16> = "~MHz\0".encode_utf16().collect();

        let mut hkey = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_LOCAL_MACHINE, subkey.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return 0;
        }

        let mut data_type: u32 = 0;
        let mut data: u32 = 0;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;

        if RegQueryValueExW(
            hkey,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut data_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        ) == 0
            && data_type == REG_DWORD
        {
            RegCloseKey(hkey);
            data
        } else {
            RegCloseKey(hkey);
            0
        }
    }
}

/// Get free memory in bytes
fn get_freemem() -> u64 {
    #[cfg(unix)]
    {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/meminfo")
                .ok()
                .and_then(|s| {
                    for line in s.lines() {
                        if line.starts_with("MemAvailable:") || line.starts_with("MemFree:") {
                            return line
                                .split_whitespace()
                                .nth(1)?
                                .parse::<u64>()
                                .ok()
                                .map(|kb| kb * 1024);
                        }
                    }
                    None
                })
                .unwrap_or(0)
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            // Get page size
            let page_size: u64 = Command::new("sysctl")
                .args(["-n", "hw.pagesize"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(4096);

            // Get free pages
            Command::new("vm_stat")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    for line in s.lines() {
                        if line.contains("Pages free:") {
                            return line
                                .split(':')
                                .nth(1)?
                                .trim()
                                .trim_end_matches('.')
                                .parse::<u64>()
                                .ok()
                                .map(|pages| pages * page_size);
                        }
                    }
                    None
                })
                .unwrap_or(0)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0
        }
    }

    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;

        let mut mem_info = MaybeUninit::<MEMORYSTATUSEX>::uninit();
        unsafe {
            let ptr = mem_info.as_mut_ptr();
            (*ptr).dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(ptr) != 0 {
                (*ptr).ullAvailPhys
            } else {
                0
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        0
    }
}

/// Get total memory in bytes
fn get_totalmem() -> u64 {
    #[cfg(unix)]
    {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/meminfo")
                .ok()
                .and_then(|s| {
                    for line in s.lines() {
                        if line.starts_with("MemTotal:") {
                            return line
                                .split_whitespace()
                                .nth(1)?
                                .parse::<u64>()
                                .ok()
                                .map(|kb| kb * 1024);
                        }
                    }
                    None
                })
                .unwrap_or(0)
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;

            Command::new("sysctl")
                .args(["-n", "hw.memsize"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0
        }
    }

    #[cfg(windows)]
    {
        use std::mem::MaybeUninit;

        let mut mem_info = MaybeUninit::<MEMORYSTATUSEX>::uninit();
        unsafe {
            let ptr = mem_info.as_mut_ptr();
            (*ptr).dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(ptr) != 0 {
                (*ptr).ullTotalPhys
            } else {
                0
            }
        }
    }

    #[cfg(not(any(unix, windows)))]
    {
        0
    }
}

/// Get system uptime in seconds
fn get_uptime() -> f64 {
    #[cfg(unix)]
    {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/uptime")
                .ok()
                .and_then(|s| s.split_whitespace().next()?.parse().ok())
                .unwrap_or(0.0)
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            use std::time::{SystemTime, UNIX_EPOCH};

            // Get boot time from sysctl
            Command::new("sysctl")
                .args(["-n", "kern.boottime"])
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| {
                    // Format: "{ sec = 1234567890, usec = 123456 } ..."
                    let sec_start = s.find("sec = ")? + 6;
                    let sec_end = s[sec_start..].find(',')?;
                    let boot_time: u64 = s[sec_start..sec_start + sec_end].parse().ok()?;

                    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

                    Some((now - boot_time) as f64)
                })
                .unwrap_or(0.0)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            0.0
        }
    }

    #[cfg(windows)]
    {
        // GetTickCount64 returns milliseconds since system start
        unsafe { (GetTickCount64() / 1000) as f64 }
    }

    #[cfg(not(any(unix, windows)))]
    {
        0.0
    }
}

/// Get load averages (Unix only, returns [0,0,0] on Windows)
fn get_loadavg() -> (f64, f64, f64) {
    #[cfg(unix)]
    {
        #[cfg(target_os = "linux")]
        {
            std::fs::read_to_string("/proc/loadavg")
                .ok()
                .and_then(|s| {
                    let parts: Vec<&str> = s.split_whitespace().collect();
                    if parts.len() >= 3 {
                        Some((
                            parts[0].parse().unwrap_or(0.0),
                            parts[1].parse().unwrap_or(0.0),
                            parts[2].parse().unwrap_or(0.0),
                        ))
                    } else {
                        None
                    }
                })
                .unwrap_or((0.0, 0.0, 0.0))
        }

        #[cfg(target_os = "macos")]
        {
            let mut loadavg = [0.0f64; 3];
            unsafe {
                libc::getloadavg(loadavg.as_mut_ptr(), 3);
            }
            (loadavg[0], loadavg[1], loadavg[2])
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            (0.0, 0.0, 0.0)
        }
    }

    #[cfg(windows)]
    {
        // Windows doesn't have load average
        (0.0, 0.0, 0.0)
    }

    #[cfg(not(any(unix, windows)))]
    {
        (0.0, 0.0, 0.0)
    }
}

/// Get network interfaces
fn get_network_interfaces(context: &mut Context) -> JsResult<JsValue> {
    let result = ObjectInitializer::new(context).build();

    #[cfg(unix)]
    {
        use std::ffi::CStr;

        unsafe {
            let mut ifaddrs: *mut libc::ifaddrs = std::ptr::null_mut();
            if libc::getifaddrs(&mut ifaddrs) == 0 {
                let mut current = ifaddrs;

                while !current.is_null() {
                    let iface = &*current;

                    if !iface.ifa_name.is_null() && !iface.ifa_addr.is_null() {
                        let name = CStr::from_ptr(iface.ifa_name)
                            .to_string_lossy()
                            .into_owned();

                        let family = (*iface.ifa_addr).sa_family as i32;

                        // Only handle IPv4 and IPv6
                        if family == libc::AF_INET || family == libc::AF_INET6 {
                            let info = ObjectInitializer::new(context).build();

                            // Get address
                            let (address, family_str, internal) = if family == libc::AF_INET {
                                let addr = iface.ifa_addr as *const libc::sockaddr_in;
                                let ip = (*addr).sin_addr.s_addr;
                                let bytes = ip.to_ne_bytes();
                                let addr_str =
                                    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3]);
                                let is_internal = bytes[0] == 127;
                                (addr_str, "IPv4", is_internal)
                            } else {
                                let addr = iface.ifa_addr as *const libc::sockaddr_in6;
                                let ip = (*addr).sin6_addr.s6_addr;
                                let mut parts = Vec::new();
                                for i in 0..8 {
                                    parts.push(format!(
                                        "{:x}",
                                        ((ip[i * 2] as u16) << 8) | (ip[i * 2 + 1] as u16)
                                    ));
                                }
                                let addr_str = parts.join(":");
                                let is_internal = ip[0] == 0
                                    && ip[1] == 0
                                    && ip[2] == 0
                                    && ip[3] == 0
                                    && ip[4] == 0
                                    && ip[5] == 0
                                    && ip[6] == 0
                                    && ip[7] == 0
                                    && ip[8] == 0
                                    && ip[9] == 0
                                    && ip[10] == 0
                                    && ip[11] == 0
                                    && ip[12] == 0
                                    && ip[13] == 0
                                    && ip[14] == 0
                                    && ip[15] == 1;
                                (addr_str, "IPv6", is_internal)
                            };

                            info.set(
                                js_string!("address"),
                                JsValue::from(js_string!(address.clone())),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("family"),
                                JsValue::from(js_string!(family_str)),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("internal"),
                                JsValue::from(internal),
                                false,
                                context,
                            )?;

                            // Get netmask if available
                            if !iface.ifa_netmask.is_null() {
                                let netmask = if family == libc::AF_INET {
                                    let mask = iface.ifa_netmask as *const libc::sockaddr_in;
                                    let ip = (*mask).sin_addr.s_addr;
                                    let bytes = ip.to_ne_bytes();
                                    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
                                } else {
                                    "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff".to_string()
                                };
                                info.set(
                                    js_string!("netmask"),
                                    JsValue::from(js_string!(netmask)),
                                    false,
                                    context,
                                )?;
                            }

                            // MAC address (placeholder)
                            info.set(
                                js_string!("mac"),
                                JsValue::from(js_string!("00:00:00:00:00:00")),
                                false,
                                context,
                            )?;

                            // Scope ID for IPv6
                            if family == libc::AF_INET6 {
                                let addr = iface.ifa_addr as *const libc::sockaddr_in6;
                                info.set(
                                    js_string!("scopeid"),
                                    JsValue::from((*addr).sin6_scope_id),
                                    false,
                                    context,
                                )?;
                            }

                            // CIDR
                            info.set(
                                js_string!("cidr"),
                                JsValue::from(js_string!(format!("{}/24", address))),
                                false,
                                context,
                            )?;

                            // Get or create array for this interface
                            let iface_arr = if let Ok(existing) =
                                result.get(js_string!(name.clone()), context)
                            {
                                if let Some(arr) = existing.as_object() {
                                    if arr.is_array() {
                                        JsArray::from_object(arr.clone())?
                                    } else {
                                        JsArray::new(context)
                                    }
                                } else {
                                    JsArray::new(context)
                                }
                            } else {
                                JsArray::new(context)
                            };

                            iface_arr.push(info, context)?;
                            result.set(js_string!(name), iface_arr, false, context)?;
                        }
                    }

                    current = (*current).ifa_next;
                }

                libc::freeifaddrs(ifaddrs);
            }
        }
    }

    #[cfg(windows)]
    {
        // Windows implementation using GetAdaptersAddresses
        unsafe {
            let flags = GAA_FLAG_INCLUDE_PREFIX | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;
            let mut size: u32 = 0;

            // First call to get required buffer size
            GetAdaptersAddresses(
                AF_UNSPEC as u32,
                flags,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
            );

            if size == 0 {
                return Ok(result.into());
            }

            // Allocate buffer
            let mut buffer: Vec<u8> = vec![0u8; size as usize];
            let adapters = buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;

            // Get adapter addresses
            let ret = GetAdaptersAddresses(
                AF_UNSPEC as u32,
                flags,
                std::ptr::null_mut(),
                adapters,
                &mut size,
            );

            if ret != 0 {
                return Ok(result.into());
            }

            // Iterate through adapters
            let mut current_adapter = adapters;
            while !current_adapter.is_null() {
                let adapter = &*current_adapter;

                // Get adapter friendly name
                let name = if !adapter.FriendlyName.is_null() {
                    let mut len = 0;
                    let mut ptr = adapter.FriendlyName;
                    while *ptr != 0 {
                        len += 1;
                        ptr = ptr.add(1);
                    }
                    String::from_utf16_lossy(std::slice::from_raw_parts(adapter.FriendlyName, len))
                } else {
                    "Unknown".to_string()
                };

                // Get MAC address
                let mac = if adapter.PhysicalAddressLength > 0 {
                    let mac_bytes =
                        &adapter.PhysicalAddress[..adapter.PhysicalAddressLength as usize];
                    mac_bytes
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(":")
                } else {
                    "00:00:00:00:00:00".to_string()
                };

                // Check if internal (loopback)
                let is_internal = adapter.IfType == 24; // IF_TYPE_SOFTWARE_LOOPBACK

                // Get or create array for this interface
                let iface_arr = if let Ok(existing) = result.get(js_string!(name.clone()), context)
                {
                    if let Some(arr) = existing.as_object() {
                        if arr.is_array() {
                            JsArray::from_object(arr.clone())?
                        } else {
                            JsArray::new(context)
                        }
                    } else {
                        JsArray::new(context)
                    }
                } else {
                    JsArray::new(context)
                };

                // Iterate through unicast addresses
                let mut unicast = adapter.FirstUnicastAddress;
                while !unicast.is_null() {
                    let addr = &*(unicast as *const IP_ADAPTER_UNICAST_ADDRESS_LH);
                    let sockaddr = addr.Address.lpSockaddr;

                    if !sockaddr.is_null() {
                        let family = (*sockaddr).sa_family;

                        let info = ObjectInitializer::new(context).build();

                        if family == AF_INET as u16 {
                            // IPv4
                            let sockaddr_in = sockaddr as *const SOCKADDR_IN;
                            let ip_bytes = (*sockaddr_in).sin_addr.S_un.S_addr.to_ne_bytes();
                            let address = format!(
                                "{}.{}.{}.{}",
                                ip_bytes[0], ip_bytes[1], ip_bytes[2], ip_bytes[3]
                            );

                            // Calculate netmask from prefix length
                            let prefix_len = addr.OnLinkPrefixLength;
                            let netmask = if prefix_len > 0 && prefix_len <= 32 {
                                let mask: u32 = !0u32 << (32 - prefix_len);
                                format!(
                                    "{}.{}.{}.{}",
                                    (mask >> 24) & 0xff,
                                    (mask >> 16) & 0xff,
                                    (mask >> 8) & 0xff,
                                    mask & 0xff
                                )
                            } else {
                                "255.255.255.0".to_string()
                            };

                            let cidr = format!("{}/{}", address, prefix_len);

                            info.set(
                                js_string!("address"),
                                JsValue::from(js_string!(address)),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("netmask"),
                                JsValue::from(js_string!(netmask)),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("family"),
                                JsValue::from(js_string!("IPv4")),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("mac"),
                                JsValue::from(js_string!(mac.clone())),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("internal"),
                                JsValue::from(is_internal),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("cidr"),
                                JsValue::from(js_string!(cidr)),
                                false,
                                context,
                            )?;

                            iface_arr.push(info, context)?;
                        } else if family == AF_INET6 as u16 {
                            // IPv6
                            let sockaddr_in6 = sockaddr as *const SOCKADDR_IN6;
                            let ip_bytes = (*sockaddr_in6).sin6_addr.u.Byte;

                            // Format IPv6 address
                            let mut parts = Vec::new();
                            for i in 0..8 {
                                parts.push(format!(
                                    "{:x}",
                                    ((ip_bytes[i * 2] as u16) << 8) | (ip_bytes[i * 2 + 1] as u16)
                                ));
                            }
                            let address = parts.join(":");

                            let prefix_len = addr.OnLinkPrefixLength;
                            let cidr = format!("{}/{}", address, prefix_len);

                            // Check if loopback (::1)
                            let is_loopback =
                                ip_bytes[0..15].iter().all(|&b| b == 0) && ip_bytes[15] == 1;

                            info.set(
                                js_string!("address"),
                                JsValue::from(js_string!(address)),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("netmask"),
                                JsValue::from(js_string!("ffff:ffff:ffff:ffff::")),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("family"),
                                JsValue::from(js_string!("IPv6")),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("mac"),
                                JsValue::from(js_string!(mac.clone())),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("internal"),
                                JsValue::from(is_internal || is_loopback),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("scopeid"),
                                JsValue::from((*sockaddr_in6).Anonymous.sin6_scope_id),
                                false,
                                context,
                            )?;
                            info.set(
                                js_string!("cidr"),
                                JsValue::from(js_string!(cidr)),
                                false,
                                context,
                            )?;

                            iface_arr.push(info, context)?;
                        }
                    }

                    unicast = (*unicast).Next;
                }

                result.set(js_string!(name), iface_arr, false, context)?;
                current_adapter = adapter.Next;
            }
        }
    }

    Ok(result.into())
}

/// Get user info
fn get_userinfo(context: &mut Context, _encoding: &str) -> JsResult<JsValue> {
    let obj = ObjectInitializer::new(context).build();

    #[cfg(unix)]
    {
        unsafe {
            let uid = libc::getuid();
            let gid = libc::getgid();
            let pwd = libc::getpwuid(uid);

            obj.set(js_string!("uid"), JsValue::from(uid as i32), false, context)?;
            obj.set(js_string!("gid"), JsValue::from(gid as i32), false, context)?;

            if !pwd.is_null() {
                let username = if !(*pwd).pw_name.is_null() {
                    std::ffi::CStr::from_ptr((*pwd).pw_name)
                        .to_string_lossy()
                        .into_owned()
                } else {
                    String::new()
                };

                let homedir = if !(*pwd).pw_dir.is_null() {
                    std::ffi::CStr::from_ptr((*pwd).pw_dir)
                        .to_string_lossy()
                        .into_owned()
                } else {
                    get_homedir()
                };

                let shell = if !(*pwd).pw_shell.is_null() {
                    std::ffi::CStr::from_ptr((*pwd).pw_shell)
                        .to_string_lossy()
                        .into_owned()
                } else {
                    "/bin/sh".to_string()
                };

                obj.set(
                    js_string!("username"),
                    JsValue::from(js_string!(username)),
                    false,
                    context,
                )?;
                obj.set(
                    js_string!("homedir"),
                    JsValue::from(js_string!(homedir)),
                    false,
                    context,
                )?;
                obj.set(
                    js_string!("shell"),
                    JsValue::from(js_string!(shell)),
                    false,
                    context,
                )?;
            } else {
                obj.set(
                    js_string!("username"),
                    JsValue::from(js_string!("")),
                    false,
                    context,
                )?;
                obj.set(
                    js_string!("homedir"),
                    JsValue::from(js_string!(get_homedir())),
                    false,
                    context,
                )?;
                obj.set(
                    js_string!("shell"),
                    JsValue::from(js_string!("/bin/sh")),
                    false,
                    context,
                )?;
            }
        }
    }

    #[cfg(windows)]
    {
        // Get username
        let mut size: u32 = 256;
        let mut buf = vec![0u16; size as usize];
        unsafe {
            if GetUserNameW(buf.as_mut_ptr(), &mut size) != 0 {
                let username = String::from_utf16_lossy(&buf[..(size - 1) as usize]);
                obj.set(
                    js_string!("username"),
                    JsValue::from(js_string!(username)),
                    false,
                    context,
                )?;
            } else {
                obj.set(
                    js_string!("username"),
                    JsValue::from(js_string!("")),
                    false,
                    context,
                )?;
            }
        }

        obj.set(js_string!("uid"), JsValue::from(-1), false, context)?;
        obj.set(js_string!("gid"), JsValue::from(-1), false, context)?;
        obj.set(
            js_string!("homedir"),
            JsValue::from(js_string!(get_homedir())),
            false,
            context,
        )?;
        obj.set(js_string!("shell"), JsValue::null(), false, context)?;
    }

    #[cfg(not(any(unix, windows)))]
    {
        obj.set(js_string!("uid"), JsValue::from(-1), false, context)?;
        obj.set(js_string!("gid"), JsValue::from(-1), false, context)?;
        obj.set(
            js_string!("username"),
            JsValue::from(js_string!("")),
            false,
            context,
        )?;
        obj.set(
            js_string!("homedir"),
            JsValue::from(js_string!(get_homedir())),
            false,
            context,
        )?;
        obj.set(js_string!("shell"), JsValue::null(), false, context)?;
    }

    Ok(obj.into())
}

/// Get process priority
fn get_priority(pid: i32) -> JsResult<JsValue> {
    #[cfg(unix)]
    {
        unsafe {
            // Reset errno before calling getpriority
            *libc::__errno_location() = 0;
            let prio = libc::getpriority(libc::PRIO_PROCESS, pid as libc::id_t);

            if prio == -1 && *libc::__errno_location() != 0 {
                return Err(JsNativeError::error()
                    .with_message(format!("getpriority failed for pid {}", pid))
                    .into());
            }

            Ok(JsValue::from(prio))
        }
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION};

        let handle = if pid == 0 {
            unsafe { GetCurrentProcess() }
        } else {
            unsafe { OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid as u32) }
        };

        if handle.is_null() {
            return Err(JsNativeError::error()
                .with_message(format!("Failed to open process {}", pid))
                .into());
        }

        let priority_class = unsafe { GetPriorityClass(handle) };

        if pid != 0 {
            unsafe {
                CloseHandle(handle);
            }
        }

        // Map Windows priority class to Unix-like priority
        let priority = match priority_class {
            REALTIME_PRIORITY_CLASS => -20,
            HIGH_PRIORITY_CLASS => -14,
            ABOVE_NORMAL_PRIORITY_CLASS => -7,
            NORMAL_PRIORITY_CLASS => 0,
            BELOW_NORMAL_PRIORITY_CLASS => 10,
            IDLE_PRIORITY_CLASS => 19,
            _ => 0,
        };

        Ok(JsValue::from(priority))
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(JsValue::from(0))
    }
}

/// Set process priority
fn set_priority(pid: i32, priority: i32) -> JsResult<JsValue> {
    #[cfg(unix)]
    {
        let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as libc::id_t, priority) };

        if result == -1 {
            return Err(JsNativeError::error()
                .with_message(format!(
                    "setpriority failed for pid {} with priority {}",
                    pid, priority
                ))
                .into());
        }

        Ok(JsValue::undefined())
    }

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_SET_INFORMATION};

        let handle = if pid == 0 {
            unsafe { GetCurrentProcess() }
        } else {
            unsafe { OpenProcess(PROCESS_SET_INFORMATION, 0, pid as u32) }
        };

        if handle.is_null() {
            return Err(JsNativeError::error()
                .with_message(format!("Failed to open process {}", pid))
                .into());
        }

        // Map Unix-like priority to Windows priority class
        let priority_class = if priority <= -20 {
            REALTIME_PRIORITY_CLASS
        } else if priority <= -14 {
            HIGH_PRIORITY_CLASS
        } else if priority <= -7 {
            ABOVE_NORMAL_PRIORITY_CLASS
        } else if priority <= 0 {
            NORMAL_PRIORITY_CLASS
        } else if priority <= 10 {
            BELOW_NORMAL_PRIORITY_CLASS
        } else {
            IDLE_PRIORITY_CLASS
        };

        let result = unsafe { SetPriorityClass(handle, priority_class) };

        if pid != 0 {
            unsafe {
                CloseHandle(handle);
            }
        }

        if result == 0 {
            return Err(JsNativeError::error()
                .with_message(format!("SetPriorityClass failed for pid {}", pid))
                .into());
        }

        Ok(JsValue::undefined())
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(JsValue::undefined())
    }
}

/// Build OS constants object
fn build_os_constants(context: &mut Context) -> JsResult<JsValue> {
    let constants = ObjectInitializer::new(context).build();

    // Signal constants
    let signals = ObjectInitializer::new(context).build();

    #[cfg(unix)]
    {
        signals.set(js_string!("SIGHUP"), JsValue::from(1), false, context)?;
        signals.set(js_string!("SIGINT"), JsValue::from(2), false, context)?;
        signals.set(js_string!("SIGQUIT"), JsValue::from(3), false, context)?;
        signals.set(js_string!("SIGILL"), JsValue::from(4), false, context)?;
        signals.set(js_string!("SIGTRAP"), JsValue::from(5), false, context)?;
        signals.set(js_string!("SIGABRT"), JsValue::from(6), false, context)?;
        signals.set(js_string!("SIGIOT"), JsValue::from(6), false, context)?;
        signals.set(js_string!("SIGBUS"), JsValue::from(7), false, context)?;
        signals.set(js_string!("SIGFPE"), JsValue::from(8), false, context)?;
        signals.set(js_string!("SIGKILL"), JsValue::from(9), false, context)?;
        signals.set(js_string!("SIGUSR1"), JsValue::from(10), false, context)?;
        signals.set(js_string!("SIGSEGV"), JsValue::from(11), false, context)?;
        signals.set(js_string!("SIGUSR2"), JsValue::from(12), false, context)?;
        signals.set(js_string!("SIGPIPE"), JsValue::from(13), false, context)?;
        signals.set(js_string!("SIGALRM"), JsValue::from(14), false, context)?;
        signals.set(js_string!("SIGTERM"), JsValue::from(15), false, context)?;
        signals.set(js_string!("SIGCHLD"), JsValue::from(17), false, context)?;
        signals.set(js_string!("SIGCONT"), JsValue::from(18), false, context)?;
        signals.set(js_string!("SIGSTOP"), JsValue::from(19), false, context)?;
        signals.set(js_string!("SIGTSTP"), JsValue::from(20), false, context)?;
        signals.set(js_string!("SIGTTIN"), JsValue::from(21), false, context)?;
        signals.set(js_string!("SIGTTOU"), JsValue::from(22), false, context)?;
        signals.set(js_string!("SIGURG"), JsValue::from(23), false, context)?;
        signals.set(js_string!("SIGXCPU"), JsValue::from(24), false, context)?;
        signals.set(js_string!("SIGXFSZ"), JsValue::from(25), false, context)?;
        signals.set(js_string!("SIGVTALRM"), JsValue::from(26), false, context)?;
        signals.set(js_string!("SIGPROF"), JsValue::from(27), false, context)?;
        signals.set(js_string!("SIGWINCH"), JsValue::from(28), false, context)?;
        signals.set(js_string!("SIGIO"), JsValue::from(29), false, context)?;
        signals.set(js_string!("SIGPOLL"), JsValue::from(29), false, context)?;
        signals.set(js_string!("SIGPWR"), JsValue::from(30), false, context)?;
        signals.set(js_string!("SIGSYS"), JsValue::from(31), false, context)?;
    }

    #[cfg(windows)]
    {
        signals.set(js_string!("SIGHUP"), JsValue::from(1), false, context)?;
        signals.set(js_string!("SIGINT"), JsValue::from(2), false, context)?;
        signals.set(js_string!("SIGILL"), JsValue::from(4), false, context)?;
        signals.set(js_string!("SIGFPE"), JsValue::from(8), false, context)?;
        signals.set(js_string!("SIGKILL"), JsValue::from(9), false, context)?;
        signals.set(js_string!("SIGSEGV"), JsValue::from(11), false, context)?;
        signals.set(js_string!("SIGTERM"), JsValue::from(15), false, context)?;
        signals.set(js_string!("SIGBREAK"), JsValue::from(21), false, context)?;
        signals.set(js_string!("SIGABRT"), JsValue::from(22), false, context)?;
    }

    constants.set(js_string!("signals"), signals, false, context)?;

    // Error constants (errno)
    let errno = ObjectInitializer::new(context).build();

    // POSIX error constants
    errno.set(js_string!("E2BIG"), JsValue::from(7), false, context)?;
    errno.set(js_string!("EACCES"), JsValue::from(13), false, context)?;
    errno.set(js_string!("EADDRINUSE"), JsValue::from(98), false, context)?;
    errno.set(
        js_string!("EADDRNOTAVAIL"),
        JsValue::from(99),
        false,
        context,
    )?;
    errno.set(
        js_string!("EAFNOSUPPORT"),
        JsValue::from(97),
        false,
        context,
    )?;
    errno.set(js_string!("EAGAIN"), JsValue::from(11), false, context)?;
    errno.set(js_string!("EALREADY"), JsValue::from(114), false, context)?;
    errno.set(js_string!("EBADF"), JsValue::from(9), false, context)?;
    errno.set(js_string!("EBADMSG"), JsValue::from(74), false, context)?;
    errno.set(js_string!("EBUSY"), JsValue::from(16), false, context)?;
    errno.set(js_string!("ECANCELED"), JsValue::from(125), false, context)?;
    errno.set(js_string!("ECHILD"), JsValue::from(10), false, context)?;
    errno.set(
        js_string!("ECONNABORTED"),
        JsValue::from(103),
        false,
        context,
    )?;
    errno.set(
        js_string!("ECONNREFUSED"),
        JsValue::from(111),
        false,
        context,
    )?;
    errno.set(js_string!("ECONNRESET"), JsValue::from(104), false, context)?;
    errno.set(js_string!("EDEADLK"), JsValue::from(35), false, context)?;
    errno.set(
        js_string!("EDESTADDRREQ"),
        JsValue::from(89),
        false,
        context,
    )?;
    errno.set(js_string!("EDOM"), JsValue::from(33), false, context)?;
    errno.set(js_string!("EDQUOT"), JsValue::from(122), false, context)?;
    errno.set(js_string!("EEXIST"), JsValue::from(17), false, context)?;
    errno.set(js_string!("EFAULT"), JsValue::from(14), false, context)?;
    errno.set(js_string!("EFBIG"), JsValue::from(27), false, context)?;
    errno.set(
        js_string!("EHOSTUNREACH"),
        JsValue::from(113),
        false,
        context,
    )?;
    errno.set(js_string!("EIDRM"), JsValue::from(43), false, context)?;
    errno.set(js_string!("EILSEQ"), JsValue::from(84), false, context)?;
    errno.set(
        js_string!("EINPROGRESS"),
        JsValue::from(115),
        false,
        context,
    )?;
    errno.set(js_string!("EINTR"), JsValue::from(4), false, context)?;
    errno.set(js_string!("EINVAL"), JsValue::from(22), false, context)?;
    errno.set(js_string!("EIO"), JsValue::from(5), false, context)?;
    errno.set(js_string!("EISCONN"), JsValue::from(106), false, context)?;
    errno.set(js_string!("EISDIR"), JsValue::from(21), false, context)?;
    errno.set(js_string!("ELOOP"), JsValue::from(40), false, context)?;
    errno.set(js_string!("EMFILE"), JsValue::from(24), false, context)?;
    errno.set(js_string!("EMLINK"), JsValue::from(31), false, context)?;
    errno.set(js_string!("EMSGSIZE"), JsValue::from(90), false, context)?;
    errno.set(js_string!("EMULTIHOP"), JsValue::from(72), false, context)?;
    errno.set(
        js_string!("ENAMETOOLONG"),
        JsValue::from(36),
        false,
        context,
    )?;
    errno.set(js_string!("ENETDOWN"), JsValue::from(100), false, context)?;
    errno.set(js_string!("ENETRESET"), JsValue::from(102), false, context)?;
    errno.set(
        js_string!("ENETUNREACH"),
        JsValue::from(101),
        false,
        context,
    )?;
    errno.set(js_string!("ENFILE"), JsValue::from(23), false, context)?;
    errno.set(js_string!("ENOBUFS"), JsValue::from(105), false, context)?;
    errno.set(js_string!("ENODATA"), JsValue::from(61), false, context)?;
    errno.set(js_string!("ENODEV"), JsValue::from(19), false, context)?;
    errno.set(js_string!("ENOENT"), JsValue::from(2), false, context)?;
    errno.set(js_string!("ENOEXEC"), JsValue::from(8), false, context)?;
    errno.set(js_string!("ENOLCK"), JsValue::from(37), false, context)?;
    errno.set(js_string!("ENOLINK"), JsValue::from(67), false, context)?;
    errno.set(js_string!("ENOMEM"), JsValue::from(12), false, context)?;
    errno.set(js_string!("ENOMSG"), JsValue::from(42), false, context)?;
    errno.set(js_string!("ENOPROTOOPT"), JsValue::from(92), false, context)?;
    errno.set(js_string!("ENOSPC"), JsValue::from(28), false, context)?;
    errno.set(js_string!("ENOSR"), JsValue::from(63), false, context)?;
    errno.set(js_string!("ENOSTR"), JsValue::from(60), false, context)?;
    errno.set(js_string!("ENOSYS"), JsValue::from(38), false, context)?;
    errno.set(js_string!("ENOTCONN"), JsValue::from(107), false, context)?;
    errno.set(js_string!("ENOTDIR"), JsValue::from(20), false, context)?;
    errno.set(js_string!("ENOTEMPTY"), JsValue::from(39), false, context)?;
    errno.set(js_string!("ENOTSOCK"), JsValue::from(88), false, context)?;
    errno.set(js_string!("ENOTSUP"), JsValue::from(95), false, context)?;
    errno.set(js_string!("ENOTTY"), JsValue::from(25), false, context)?;
    errno.set(js_string!("ENXIO"), JsValue::from(6), false, context)?;
    errno.set(js_string!("EOPNOTSUPP"), JsValue::from(95), false, context)?;
    errno.set(js_string!("EOVERFLOW"), JsValue::from(75), false, context)?;
    errno.set(js_string!("EPERM"), JsValue::from(1), false, context)?;
    errno.set(js_string!("EPIPE"), JsValue::from(32), false, context)?;
    errno.set(js_string!("EPROTO"), JsValue::from(71), false, context)?;
    errno.set(
        js_string!("EPROTONOSUPPORT"),
        JsValue::from(93),
        false,
        context,
    )?;
    errno.set(js_string!("EPROTOTYPE"), JsValue::from(91), false, context)?;
    errno.set(js_string!("ERANGE"), JsValue::from(34), false, context)?;
    errno.set(js_string!("EROFS"), JsValue::from(30), false, context)?;
    errno.set(js_string!("ESPIPE"), JsValue::from(29), false, context)?;
    errno.set(js_string!("ESRCH"), JsValue::from(3), false, context)?;
    errno.set(js_string!("ESTALE"), JsValue::from(116), false, context)?;
    errno.set(js_string!("ETIME"), JsValue::from(62), false, context)?;
    errno.set(js_string!("ETIMEDOUT"), JsValue::from(110), false, context)?;
    errno.set(js_string!("ETXTBSY"), JsValue::from(26), false, context)?;
    errno.set(js_string!("EWOULDBLOCK"), JsValue::from(11), false, context)?;
    errno.set(js_string!("EXDEV"), JsValue::from(18), false, context)?;

    // Windows-specific error constants
    #[cfg(windows)]
    {
        errno.set(js_string!("WSAEINTR"), JsValue::from(10004), false, context)?;
        errno.set(js_string!("WSAEBADF"), JsValue::from(10009), false, context)?;
        errno.set(
            js_string!("WSAEACCES"),
            JsValue::from(10013),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEFAULT"),
            JsValue::from(10014),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEINVAL"),
            JsValue::from(10022),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEMFILE"),
            JsValue::from(10024),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEWOULDBLOCK"),
            JsValue::from(10035),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEINPROGRESS"),
            JsValue::from(10036),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEALREADY"),
            JsValue::from(10037),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENOTSOCK"),
            JsValue::from(10038),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEDESTADDRREQ"),
            JsValue::from(10039),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEMSGSIZE"),
            JsValue::from(10040),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEPROTOTYPE"),
            JsValue::from(10041),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENOPROTOOPT"),
            JsValue::from(10042),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEPROTONOSUPPORT"),
            JsValue::from(10043),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAESOCKTNOSUPPORT"),
            JsValue::from(10044),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEOPNOTSUPP"),
            JsValue::from(10045),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEPFNOSUPPORT"),
            JsValue::from(10046),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEAFNOSUPPORT"),
            JsValue::from(10047),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEADDRINUSE"),
            JsValue::from(10048),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEADDRNOTAVAIL"),
            JsValue::from(10049),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENETDOWN"),
            JsValue::from(10050),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENETUNREACH"),
            JsValue::from(10051),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENETRESET"),
            JsValue::from(10052),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAECONNABORTED"),
            JsValue::from(10053),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAECONNRESET"),
            JsValue::from(10054),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENOBUFS"),
            JsValue::from(10055),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEISCONN"),
            JsValue::from(10056),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENOTCONN"),
            JsValue::from(10057),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAESHUTDOWN"),
            JsValue::from(10058),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAETOOMANYREFS"),
            JsValue::from(10059),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAETIMEDOUT"),
            JsValue::from(10060),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAECONNREFUSED"),
            JsValue::from(10061),
            false,
            context,
        )?;
        errno.set(js_string!("WSAELOOP"), JsValue::from(10062), false, context)?;
        errno.set(
            js_string!("WSAENAMETOOLONG"),
            JsValue::from(10063),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEHOSTDOWN"),
            JsValue::from(10064),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEHOSTUNREACH"),
            JsValue::from(10065),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENOTEMPTY"),
            JsValue::from(10066),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEPROCLIM"),
            JsValue::from(10067),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEUSERS"),
            JsValue::from(10068),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEDQUOT"),
            JsValue::from(10069),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAESTALE"),
            JsValue::from(10070),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEREMOTE"),
            JsValue::from(10071),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSASYSNOTREADY"),
            JsValue::from(10091),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAVERNOTSUPPORTED"),
            JsValue::from(10092),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSANOTINITIALISED"),
            JsValue::from(10093),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEDISCON"),
            JsValue::from(10101),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAENOMORE"),
            JsValue::from(10102),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAECANCELLED"),
            JsValue::from(10103),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEINVALIDPROCTABLE"),
            JsValue::from(10104),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEINVALIDPROVIDER"),
            JsValue::from(10105),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEPROVIDERFAILEDINIT"),
            JsValue::from(10106),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSASYSCALLFAILURE"),
            JsValue::from(10107),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSASERVICE_NOT_FOUND"),
            JsValue::from(10108),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSATYPE_NOT_FOUND"),
            JsValue::from(10109),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSA_E_NO_MORE"),
            JsValue::from(10110),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSA_E_CANCELLED"),
            JsValue::from(10111),
            false,
            context,
        )?;
        errno.set(
            js_string!("WSAEREFUSED"),
            JsValue::from(10112),
            false,
            context,
        )?;
    }

    constants.set(js_string!("errno"), errno, false, context)?;

    // Priority constants
    let priority = ObjectInitializer::new(context).build();
    priority.set(
        js_string!("PRIORITY_LOW"),
        JsValue::from(19),
        false,
        context,
    )?;
    priority.set(
        js_string!("PRIORITY_BELOW_NORMAL"),
        JsValue::from(10),
        false,
        context,
    )?;
    priority.set(
        js_string!("PRIORITY_NORMAL"),
        JsValue::from(0),
        false,
        context,
    )?;
    priority.set(
        js_string!("PRIORITY_ABOVE_NORMAL"),
        JsValue::from(-7),
        false,
        context,
    )?;
    priority.set(
        js_string!("PRIORITY_HIGH"),
        JsValue::from(-14),
        false,
        context,
    )?;
    priority.set(
        js_string!("PRIORITY_HIGHEST"),
        JsValue::from(-20),
        false,
        context,
    )?;

    constants.set(js_string!("priority"), priority, false, context)?;

    // dlopen constants (Unix only)
    #[cfg(unix)]
    {
        let dlopen = ObjectInitializer::new(context).build();
        dlopen.set(js_string!("RTLD_LAZY"), JsValue::from(1), false, context)?;
        dlopen.set(js_string!("RTLD_NOW"), JsValue::from(2), false, context)?;
        dlopen.set(
            js_string!("RTLD_GLOBAL"),
            JsValue::from(0x100),
            false,
            context,
        )?;
        dlopen.set(js_string!("RTLD_LOCAL"), JsValue::from(0), false, context)?;

        constants.set(js_string!("dlopen"), dlopen, false, context)?;
    }

    // UV constants (libuv compatibility)
    let uv = ObjectInitializer::new(context).build();
    uv.set(
        js_string!("UV_UDP_REUSEADDR"),
        JsValue::from(4),
        false,
        context,
    )?;
    constants.set(
        js_string!("UV_UDP_REUSEADDR"),
        JsValue::from(4),
        false,
        context,
    )?;

    Ok(constants.into())
}
