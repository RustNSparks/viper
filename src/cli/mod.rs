//! Interactive REPL (Read-Eval-Print Loop) for the Viper TypeScript runtime

use crate::runtime::{Runtime, RuntimeResult};
use colored::Colorize;
use std::io::{self, BufRead, Write};

/// REPL configuration
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Prompt string
    pub prompt: String,
    /// Continuation prompt for multiline input
    pub continuation_prompt: String,
    /// Whether to show the result of each evaluation
    pub show_result: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            prompt: "viper> ".to_string(),
            continuation_prompt: "   ... ".to_string(),
            show_result: true,
        }
    }
}

/// Interactive REPL for TypeScript execution
pub struct Repl {
    runtime: Runtime,
    config: ReplConfig,
    line_count: usize,
}

impl Repl {
    /// Create a new REPL with default configuration
    pub fn new() -> RuntimeResult<Self> {
        Ok(Self {
            runtime: Runtime::new()?,
            config: ReplConfig::default(),
            line_count: 0,
        })
    }

    /// Create a new REPL with custom configuration
    #[allow(dead_code)]
    pub fn with_config(config: ReplConfig) -> RuntimeResult<Self> {
        Ok(Self {
            runtime: Runtime::new()?,
            config,
            line_count: 0,
        })
    }

    /// Print the welcome banner
    fn print_banner(&self) {
        println!("{}", "╔════════════════════════════════════════════════════════════╗".cyan());
        println!("{}", "║                     Viper TypeScript Runtime               ║".cyan());
        println!("{}", "║                    Powered by Boa + OXC                    ║".cyan());
        println!("{}", "╚════════════════════════════════════════════════════════════╝".cyan());
        println!();
        println!("Version: {}", env!("CARGO_PKG_VERSION").green());
        println!("Type {} for help, {} to exit", ".help".yellow(), ".exit".yellow());
        println!();
    }

    /// Print help information
    fn print_help(&self) {
        println!("{}", "Available commands:".bold());
        println!("  {}    - Show this help message", ".help".yellow());
        println!("  {}   - Clear the screen", ".clear".yellow());
        println!("  {}    - Exit the REPL", ".exit".yellow());
        println!("  {}    - Show runtime info", ".info".yellow());
        println!();
        println!("{}", "Tips:".bold());
        println!("  - Enter TypeScript or JavaScript code directly");
        println!("  - Multi-line input: end line with {{ or \\");
        println!("  - Type annotations are stripped at runtime");
        println!();
    }

    /// Print runtime info
    fn print_info(&self) {
        println!("{}", "Runtime Information:".bold());
        println!("  Engine: Boa JavaScript Engine v0.21");
        println!("  Transpiler: OXC v0.105");
        println!("  Runtime: Viper v{}", env!("CARGO_PKG_VERSION"));
        println!();
    }

    /// Check if input is complete (not waiting for more lines)
    fn is_complete(&self, input: &str) -> bool {
        let open_braces = input.matches('{').count();
        let close_braces = input.matches('}').count();
        let open_parens = input.matches('(').count();
        let close_parens = input.matches(')').count();
        let open_brackets = input.matches('[').count();
        let close_brackets = input.matches(']').count();

        open_braces == close_braces
            && open_parens == close_parens
            && open_brackets == close_brackets
            && !input.trim().ends_with('\\')
    }

    /// Process a REPL command (starts with .)
    fn process_command(&mut self, command: &str) -> bool {
        match command.trim() {
            ".exit" | ".quit" | ".q" => return false,
            ".help" | ".h" => self.print_help(),
            ".clear" | ".cls" => {
                // Clear screen using ANSI escape codes
                print!("\x1B[2J\x1B[1;1H");
                let _ = io::stdout().flush();
            }
            ".info" => self.print_info(),
            cmd => {
                println!("{}: Unknown command '{}'. Type .help for available commands.",
                    "Error".red(), cmd);
            }
        }
        true
    }

    /// Evaluate TypeScript code and print the result
    fn evaluate(&mut self, code: &str) {
        self.line_count += 1;
        let filename = format!("repl_{}.ts", self.line_count);

        match self.runtime.eval(code, &filename) {
            Ok(value) => {
                if self.config.show_result && !value.is_undefined() {
                    let result_str = self.runtime.value_to_string(&value);
                    println!("{} {}", "=>".green(), result_str);
                }
            }
            Err(e) => {
                println!("{}: {}", "Error".red(), e);
            }
        }
    }

    /// Run the interactive REPL
    pub fn run(&mut self) -> RuntimeResult<()> {
        self.print_banner();

        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut input_buffer = String::new();
        let mut in_multiline = false;

        loop {
            // Print prompt
            let prompt = if in_multiline {
                &self.config.continuation_prompt
            } else {
                &self.config.prompt
            };
            print!("{}", prompt.cyan());
            stdout.flush().map_err(|e| crate::runtime::RuntimeError::IoError(e))?;

            // Read line
            let mut line = String::new();
            match stdin.lock().read_line(&mut line) {
                Ok(0) => {
                    // EOF
                    println!();
                    break;
                }
                Ok(_) => {}
                Err(e) => {
                    println!("{}: Failed to read input: {}", "Error".red(), e);
                    continue;
                }
            }

            let line = line.trim_end_matches('\n').trim_end_matches('\r');

            // Handle empty input
            if line.is_empty() && !in_multiline {
                continue;
            }

            // Handle commands
            if line.starts_with('.') && !in_multiline {
                if !self.process_command(line) {
                    break;
                }
                continue;
            }

            // Accumulate input
            if in_multiline {
                input_buffer.push('\n');
            }
            input_buffer.push_str(line);

            // Check if input is complete
            if self.is_complete(&input_buffer) {
                if !input_buffer.trim().is_empty() {
                    self.evaluate(&input_buffer);
                }
                input_buffer.clear();
                in_multiline = false;
            } else {
                in_multiline = true;
            }
        }

        println!("{}", "Goodbye!".cyan());
        Ok(())
    }
}

impl Default for Repl {
    fn default() -> Self {
        Self::new().expect("Failed to create REPL")
    }
}
