//! Process API - Node.js compatible process object
//!
//! Provides:
//! - process.argv - Command line arguments
//! - process.exit(code) - Exit the process
//! - process.cwd() - Current working directory
//! - process.env - Environment variables (via Viper.env)
//! - process.pid - Process ID
//! - process.platform - Operating system platform
//! - process.arch - CPU architecture
//! - process.version - Viper version
//! - process.nextTick(callback) - Schedule callback before next event loop tick

use boa_engine::{
    Context, JsResult, JsValue, NativeFunction, Source, js_string, object::ObjectInitializer,
    object::builtins::JsArray,
};

/// Register the process object
pub fn register_process(context: &mut Context, args: &[String]) -> JsResult<()> {
    // Register platform and arch constants FIRST (before JS code uses them)
    let platform = if cfg!(target_os = "windows") {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else if cfg!(target_arch = "x86") {
        "ia32"
    } else if cfg!(target_arch = "arm") {
        "arm"
    } else {
        "unknown"
    };

    context.global_object().set(
        js_string!("__VIPER_PLATFORM__"),
        JsValue::from(js_string!(platform)),
        false,
        context,
    )?;

    context.global_object().set(
        js_string!("__VIPER_ARCH__"),
        JsValue::from(js_string!(arch)),
        false,
        context,
    )?;

    // Store argv in a global for the JS code to access
    let argv_array = JsArray::new(context);
    for arg in args {
        argv_array.push(JsValue::from(js_string!(arg.clone())), context)?;
    }
    context
        .global_object()
        .set(js_string!("__viper_argv"), argv_array, false, context)?;

    // Register process ID
    context.global_object().set(
        js_string!("__viper_pid"),
        JsValue::from(std::process::id() as i32),
        false,
        context,
    )?;

    // Register native process.exit
    let exit_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let code = args
            .get(0)
            .map(|v| v.to_i32(context))
            .transpose()?
            .unwrap_or(0);
        std::process::exit(code);
    });
    context.global_object().set(
        js_string!("__viper_exit"),
        exit_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Register process.hrtime for high-resolution timing
    let hrtime_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let secs = now.as_secs();
        let nanos = now.subsec_nanos();

        // If previous time provided, calculate difference
        if let Some(prev) = args.get(0) {
            if let Some(prev_arr) = prev.as_object() {
                let prev_secs = prev_arr.get(0, context)?.to_number(context)? as u64;
                let prev_nanos = prev_arr.get(1, context)?.to_number(context)? as u32;

                let diff_secs = secs.saturating_sub(prev_secs);
                let diff_nanos = if nanos >= prev_nanos {
                    nanos - prev_nanos
                } else {
                    1_000_000_000 - (prev_nanos - nanos)
                };

                let result = JsArray::new(context);
                result.push(JsValue::from(diff_secs as f64), context)?;
                result.push(JsValue::from(diff_nanos as f64), context)?;
                return Ok(result.into());
            }
        }

        let result = JsArray::new(context);
        result.push(JsValue::from(secs as f64), context)?;
        result.push(JsValue::from(nanos as f64), context)?;
        Ok(result.into())
    });
    context.global_object().set(
        js_string!("__viper_hrtime"),
        hrtime_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Register hrtime.bigint
    let hrtime_bigint_fn = NativeFunction::from_fn_ptr(|_this, _args, _context| {
        use std::time::{SystemTime, UNIX_EPOCH};

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();

        let nanos = now.as_nanos() as f64;
        Ok(JsValue::from(nanos))
    });
    context.global_object().set(
        js_string!("__viper_hrtime_bigint"),
        hrtime_bigint_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Register __viper_cwd
    let cwd_fn =
        NativeFunction::from_fn_ptr(|_this, _args, _context| match std::env::current_dir() {
            Ok(path) => Ok(JsValue::from(js_string!(
                path.to_string_lossy().to_string()
            ))),
            Err(_) => Ok(JsValue::from(js_string!("."))),
        });
    context.global_object().set(
        js_string!("__viper_cwd"),
        cwd_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Register __viper_env_get
    let env_get_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        match std::env::var(&key) {
            Ok(val) => Ok(JsValue::from(js_string!(val))),
            Err(_) => Ok(JsValue::undefined()),
        }
    });
    context.global_object().set(
        js_string!("__viper_env_get"),
        env_get_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Register __viper_env_set
    let env_set_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let key = args
            .get(0)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();
        let value = args
            .get(1)
            .map(|v| v.to_string(context))
            .transpose()?
            .map(|s| s.to_std_string_escaped())
            .unwrap_or_default();

        // SAFETY: Single-threaded environment
        unsafe {
            std::env::set_var(&key, &value);
        }
        Ok(JsValue::undefined())
    });
    context.global_object().set(
        js_string!("__viper_env_set"),
        env_set_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Register __viper_env_all
    let env_all_fn = NativeFunction::from_fn_ptr(|_this, _args, context| {
        let obj = ObjectInitializer::new(context).build();
        for (key, value) in std::env::vars() {
            obj.set(
                js_string!(key),
                JsValue::from(js_string!(value)),
                false,
                context,
            )?;
        }
        Ok(JsValue::from(obj))
    });
    context.global_object().set(
        js_string!("__viper_env_all"),
        env_all_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Create the process object in JavaScript
    let process_code = r#"
        globalThis.process = {
            // Command line arguments
            argv: __viper_argv || [],

            // Exit the process
            exit: (code = 0) => __viper_exit(code),

            // Current working directory
            cwd: () => __viper_cwd(),

            // Change directory (limited support)
            chdir: (dir) => {
                throw new Error('process.chdir() is not supported in Viper');
            },

            // Environment variables
            env: new Proxy({}, {
                get: (target, prop) => __viper_env_get(String(prop)),
                set: (target, prop, value) => {
                    __viper_env_set(String(prop), String(value));
                    return true;
                },
                has: (target, prop) => __viper_env_get(String(prop)) !== undefined,
                ownKeys: () => Object.keys(__viper_env_all()),
                getOwnPropertyDescriptor: (target, prop) => {
                    const value = __viper_env_get(String(prop));
                    if (value !== undefined) {
                        return { value, writable: true, enumerable: true, configurable: true };
                    }
                    return undefined;
                }
            }),

            // Process ID
            pid: __viper_pid || 0,

            // Parent process ID (not available, return 0)
            ppid: 0,

            // Platform
            platform: (() => {
                const p = __VIPER_PLATFORM__ || 'unknown';
                return p;
            })(),

            // Architecture
            arch: (() => {
                const a = __VIPER_ARCH__ || 'unknown';
                return a;
            })(),

            // Version
            version: 'v' + (__VIPER_VERSION__ || '0.1.0'),

            // Versions object
            versions: {
                viper: __VIPER_VERSION__ || '0.1.0',
                boa: '0.21',
                oxc: '0.46',
            },

            // Title (read-only for now)
            title: 'viper',

            // Executable path (approximate)
            execPath: __viper_argv?.[0] || 'viper',

            // High-resolution time
            hrtime: Object.assign(
                (prev) => __viper_hrtime(prev),
                { bigint: () => BigInt(Math.floor(__viper_hrtime_bigint())) }
            ),

            // Memory usage (approximate)
            memoryUsage: () => ({
                rss: 0,
                heapTotal: 0,
                heapUsed: 0,
                external: 0,
                arrayBuffers: 0,
            }),

            // CPU usage (not implemented)
            cpuUsage: () => ({ user: 0, system: 0 }),

            // Uptime in seconds
            uptime: (() => {
                const start = Date.now();
                return () => (Date.now() - start) / 1000;
            })(),

            // nextTick - schedule callback before next event loop iteration
            // For now, uses queueMicrotask as approximation
            nextTick: (callback, ...args) => {
                if (typeof callback !== 'function') {
                    throw new TypeError('callback must be a function');
                }
                queueMicrotask(() => callback(...args));
            },

            // Standard streams (basic implementation)
            stdout: {
                write: (data) => {
                    console.log(String(data).replace(/\n$/, ''));
                    return true;
                },
                isTTY: true,
            },
            stderr: {
                write: (data) => {
                    console.error(String(data).replace(/\n$/, ''));
                    return true;
                },
                isTTY: true,
            },
            stdin: {
                isTTY: true,
            },

            // Event emitter methods (minimal implementation)
            on: (event, listener) => {
                // Store listeners for later
                if (!process._listeners) process._listeners = {};
                if (!process._listeners[event]) process._listeners[event] = [];
                process._listeners[event].push(listener);
                return process;
            },
            once: (event, listener) => {
                const wrapped = (...args) => {
                    process.off(event, wrapped);
                    listener(...args);
                };
                return process.on(event, wrapped);
            },
            off: (event, listener) => {
                if (process._listeners?.[event]) {
                    process._listeners[event] = process._listeners[event].filter(l => l !== listener);
                }
                return process;
            },
            emit: (event, ...args) => {
                if (process._listeners?.[event]) {
                    for (const listener of process._listeners[event]) {
                        listener(...args);
                    }
                    return true;
                }
                return false;
            },

            // Listeners storage
            _listeners: {},
        };
    "#;

    let source = Source::from_bytes(process_code.as_bytes());
    context.eval(source)?;

    Ok(())
}
