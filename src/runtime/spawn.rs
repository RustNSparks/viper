//! Spawn API - Run shell commands
//!
//! Provides:
//! - Viper.spawn(command, args?, options?) - Spawn a child process
//! - Viper.exec(command) - Execute a shell command and return output
//! - Viper.$ - Tagged template literal for shell commands (Bun-style)

use boa_engine::{
    Context, JsNativeError, JsResult, JsValue, NativeFunction, Source, js_string,
    object::ObjectInitializer, object::builtins::JsUint8Array,
};
use std::process::{Command, Stdio};

/// Register the spawn APIs
pub fn register_spawn(context: &mut Context) -> JsResult<()> {
    // Viper.spawn(command, args?, options?)
    let spawn_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let command = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("spawn requires a command"))?
            .to_string(context)?
            .to_std_string_escaped();

        // Parse arguments
        let cmd_args: Vec<String> = if let Some(args_val) = args.get(1) {
            if let Some(arr) = args_val.as_object() {
                let mut result = Vec::new();
                if let Ok(len_val) = arr.get(js_string!("length"), context) {
                    if let Some(len) = len_val.as_number() {
                        for i in 0..(len as usize) {
                            if let Ok(item) = arr.get(i, context) {
                                if let Ok(s) = item.to_string(context) {
                                    result.push(s.to_std_string_escaped());
                                }
                            }
                        }
                    }
                }
                result
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        // Parse options
        let options = args.get(2).and_then(|v| v.as_object());

        let cwd = options
            .as_ref()
            .and_then(|o| o.get(js_string!("cwd"), context).ok())
            .and_then(|v| {
                if v.is_undefined() || v.is_null() {
                    None
                } else {
                    v.to_string(context).ok().map(|s| s.to_std_string_escaped())
                }
            });

        let shell = options
            .as_ref()
            .and_then(|o| o.get(js_string!("shell"), context).ok())
            .map(|v| v.to_boolean())
            .unwrap_or(false);

        // Build the command
        let mut cmd = if shell {
            if cfg!(target_os = "windows") {
                let mut c = Command::new("cmd");
                c.arg("/C").arg(&command);
                for arg in &cmd_args {
                    c.arg(arg);
                }
                c
            } else {
                let mut c = Command::new("sh");
                let full_cmd = if cmd_args.is_empty() {
                    command.clone()
                } else {
                    format!("{} {}", command, cmd_args.join(" "))
                };
                c.arg("-c").arg(&full_cmd);
                c
            }
        } else {
            let mut c = Command::new(&command);
            c.args(&cmd_args);
            c
        };

        // Set working directory
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }

        // Note: Custom env is handled in JavaScript wrapper for simplicity

        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute
        let output = cmd.output().map_err(|e| {
            JsNativeError::error().with_message(format!("Failed to spawn process: {}", e))
        })?;

        // Create result object
        let result = ObjectInitializer::new(context)
            .property(
                js_string!("exitCode"),
                JsValue::from(output.status.code().unwrap_or(-1)),
                Default::default(),
            )
            .property(
                js_string!("success"),
                JsValue::from(output.status.success()),
                Default::default(),
            )
            .build();

        // Add stdout as Uint8Array
        let stdout_array = JsUint8Array::from_iter(output.stdout.clone(), context)?;
        result.set(js_string!("stdout"), stdout_array, false, context)?;

        // Add stderr as Uint8Array
        let stderr_array = JsUint8Array::from_iter(output.stderr.clone(), context)?;
        result.set(js_string!("stderr"), stderr_array, false, context)?;

        Ok(JsValue::from(result))
    });
    context.global_object().set(
        js_string!("__viper_spawn"),
        spawn_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Viper.exec(command) - Simple shell execution
    let exec_fn = NativeFunction::from_fn_ptr(|_this, args, context| {
        let command = args
            .get(0)
            .ok_or_else(|| JsNativeError::typ().with_message("exec requires a command"))?
            .to_string(context)?
            .to_std_string_escaped();

        let output = if cfg!(target_os = "windows") {
            Command::new("cmd").arg("/C").arg(&command).output()
        } else {
            Command::new("sh").arg("-c").arg(&command).output()
        };

        let output = output.map_err(|e| {
            JsNativeError::error().with_message(format!("Failed to execute command: {}", e))
        })?;

        // Create result object
        let result = ObjectInitializer::new(context)
            .property(
                js_string!("exitCode"),
                JsValue::from(output.status.code().unwrap_or(-1)),
                Default::default(),
            )
            .property(
                js_string!("success"),
                JsValue::from(output.status.success()),
                Default::default(),
            )
            .build();

        // Add stdout as string
        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        result.set(
            js_string!("stdout"),
            JsValue::from(js_string!(stdout_str)),
            false,
            context,
        )?;

        // Add stderr as string
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
        result.set(
            js_string!("stderr"),
            JsValue::from(js_string!(stderr_str)),
            false,
            context,
        )?;

        Ok(JsValue::from(result))
    });
    context.global_object().set(
        js_string!("__viper_exec"),
        exec_fn.to_js_function(context.realm()),
        false,
        context,
    )?;

    // Create JavaScript wrappers
    let spawn_code = r#"
        // Ensure Viper namespace exists
        if (typeof Viper === 'undefined') {
            globalThis.Viper = {};
        }

        // Add spawn to Viper namespace
        {
            // Viper.spawn(command, args?, options?) - Returns a Promise
            Viper.spawn = async (command, args, options) => {
                const result = __viper_spawn(command, args || [], options || {});

                // Decode stdout/stderr to strings
                const decoder = new TextDecoder();
                const stdoutText = decoder.decode(result.stdout);
                const stderrText = decoder.decode(result.stderr);

                return {
                    exitCode: result.exitCode,
                    success: result.success,
                    stdout: stdoutText,
                    stderr: stderrText,
                    text: () => stdoutText,
                    toString: () => stdoutText.trim(),
                };
            };

            // Viper.exec(command) - Simple shell execution returning strings (async)
            Viper.exec = async (command) => {
                const result = __viper_exec(command);
                return {
                    exitCode: result.exitCode,
                    success: result.success,
                    stdout: result.stdout,
                    stderr: result.stderr,
                    toString: () => result.stdout.trim(),
                };
            };

            // Viper.$ - Tagged template literal for shell commands (Bun-style)
            Viper.$ = (strings, ...values) => {
                // Build command from template literal
                let command = strings[0];
                for (let i = 0; i < values.length; i++) {
                    // Escape shell special characters in interpolated values
                    const val = String(values[i]).replace(/(["\s'$`\\])/g, '\\$1');
                    command += val + strings[i + 1];
                }
                return Viper.exec(command);
            };

            // Viper.sleep(ms) - Promise-based sleep
            Viper.sleep = (ms) => new Promise(resolve => setTimeout(resolve, ms));

            // Viper.which(command) - Find executable path
            Viper.which = (command) => {
                const isWin = process?.platform === 'win32';
                const result = Viper.exec(isWin ? `where ${command}` : `which ${command}`);
                if (result.success) {
                    return result.stdout.trim().split('\n')[0];
                }
                return null;
            };
        }
    "#;

    let source = Source::from_bytes(spawn_code.as_bytes());
    context.eval(source)?;

    Ok(())
}
