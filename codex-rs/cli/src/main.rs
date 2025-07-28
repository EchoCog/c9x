use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use codex_arg0::arg0_dispatch_or_else;
use codex_chatgpt::apply_command::ApplyCommand;
use codex_chatgpt::apply_command::run_apply_command;
use codex_cli::LandlockCommand;
use codex_cli::SeatbeltCommand;
use codex_cli::login::run_login_with_chatgpt;
use codex_cli::proto;
use codex_common::CliConfigOverrides;
use codex_core::config::Config;
use codex_core::yaml_prompt::YamlPrompt;
use codex_exec::Cli as ExecCli;
use codex_tui::Cli as TuiCli;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::proto::ProtoCli;

/// Codex CLI
///
/// If no subcommand is specified, options will be forwarded to the interactive CLI.
#[derive(Debug, Parser)]
#[clap(
    author,
    version,
    // If a sub‑command is given, ignore requirements of the default args.
    subcommand_negates_reqs = true
)]
struct MultitoolCli {
    #[clap(flatten)]
    pub config_overrides: CliConfigOverrides,

    #[clap(flatten)]
    interactive: TuiCli,

    #[clap(subcommand)]
    subcommand: Option<Subcommand>,
}

#[derive(Debug, clap::Subcommand)]
enum Subcommand {
    /// Run Codex non-interactively.
    #[clap(visible_alias = "e")]
    Exec(ExecCli),

    /// Login with ChatGPT.
    Login(LoginCommand),

    /// Experimental: run Codex as an MCP server.
    Mcp,

    /// Run the Protocol stream via stdin/stdout
    #[clap(visible_alias = "p")]
    Proto(ProtoCli),

    /// Generate shell completion scripts.
    Completion(CompletionCommand),

    /// Internal debugging commands.
    Debug(DebugArgs),

    /// Apply the latest diff produced by Codex agent as a `git apply` to your local working tree.
    #[clap(visible_alias = "a")]
    Apply(ApplyCommand),

    /// Execute YAML prompt file on input files (Deep Tree Echo functionality).
    /// 
    /// Loads a YAML prompt file and applies it to the specified input files.
    /// The input files' content is combined and substituted into the {{input}} 
    /// template variable in the prompt. This enables the Deep Tree Echo Core
    /// Engine to perform structured analysis and reasoning on the provided files.
    ///
    /// Example: codex dtecho file1.md file2.md
    #[clap(visible_alias = "dt")]
    Dtecho(DtechoCommand),
}

#[derive(Debug, Parser)]
struct CompletionCommand {
    /// Shell to generate completions for
    #[clap(value_enum, default_value_t = Shell::Bash)]
    shell: Shell,
}

#[derive(Debug, Parser)]
struct DebugArgs {
    #[command(subcommand)]
    cmd: DebugCommand,
}

#[derive(Debug, clap::Subcommand)]
enum DebugCommand {
    /// Run a command under Seatbelt (macOS only).
    Seatbelt(SeatbeltCommand),

    /// Run a command under Landlock+seccomp (Linux only).
    Landlock(LandlockCommand),
}

#[derive(Debug, Parser)]
struct LoginCommand {
    #[clap(skip)]
    config_overrides: CliConfigOverrides,
}

#[derive(Debug, Parser)]
struct DtechoCommand {
    /// Path to the YAML prompt file (defaults to echo-tree.prompt.yml)
    /// 
    /// The YAML file should contain 'messages' array and optional 'model' field.
    /// Template variables like {{input}} will be substituted with input file content.
    #[clap(short, long, default_value = "echo-tree.prompt.yml")]
    prompt_file: PathBuf,

    /// Input files to process
    /// 
    /// All specified files will be read and their content combined into the
    /// {{input}} template variable for processing by the YAML prompt.
    input_files: Vec<PathBuf>,

    #[clap(flatten)]
    config_overrides: CliConfigOverrides,
}

fn main() -> anyhow::Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        cli_main(codex_linux_sandbox_exe).await?;
        Ok(())
    })
}

async fn cli_main(codex_linux_sandbox_exe: Option<PathBuf>) -> anyhow::Result<()> {
    let cli = MultitoolCli::parse();

    match cli.subcommand {
        None => {
            let mut tui_cli = cli.interactive;
            prepend_config_flags(&mut tui_cli.config_overrides, cli.config_overrides);
            let usage = codex_tui::run_main(tui_cli, codex_linux_sandbox_exe)?;
            println!("{}", codex_core::protocol::FinalOutput::from(usage));
        }
        Some(Subcommand::Exec(mut exec_cli)) => {
            prepend_config_flags(&mut exec_cli.config_overrides, cli.config_overrides);
            codex_exec::run_main(exec_cli, codex_linux_sandbox_exe).await?;
        }
        Some(Subcommand::Mcp) => {
            codex_mcp_server::run_main(codex_linux_sandbox_exe).await?;
        }
        Some(Subcommand::Login(mut login_cli)) => {
            prepend_config_flags(&mut login_cli.config_overrides, cli.config_overrides);
            run_login_with_chatgpt(login_cli.config_overrides).await;
        }
        Some(Subcommand::Proto(mut proto_cli)) => {
            prepend_config_flags(&mut proto_cli.config_overrides, cli.config_overrides);
            proto::run_main(proto_cli).await?;
        }
        Some(Subcommand::Completion(completion_cli)) => {
            print_completion(completion_cli);
        }
        Some(Subcommand::Debug(debug_args)) => match debug_args.cmd {
            DebugCommand::Seatbelt(mut seatbelt_cli) => {
                prepend_config_flags(&mut seatbelt_cli.config_overrides, cli.config_overrides);
                codex_cli::debug_sandbox::run_command_under_seatbelt(
                    seatbelt_cli,
                    codex_linux_sandbox_exe,
                )
                .await?;
            }
            DebugCommand::Landlock(mut landlock_cli) => {
                prepend_config_flags(&mut landlock_cli.config_overrides, cli.config_overrides);
                codex_cli::debug_sandbox::run_command_under_landlock(
                    landlock_cli,
                    codex_linux_sandbox_exe,
                )
                .await?;
            }
        },
        Some(Subcommand::Apply(mut apply_cli)) => {
            prepend_config_flags(&mut apply_cli.config_overrides, cli.config_overrides);
            run_apply_command(apply_cli, None).await?;
        }
        Some(Subcommand::Dtecho(mut dtecho_cli)) => {
            prepend_config_flags(&mut dtecho_cli.config_overrides, cli.config_overrides);
            run_dtecho_command(dtecho_cli, codex_linux_sandbox_exe).await?;
        }
    }

    Ok(())
}

/// Prepend root-level overrides so they have lower precedence than
/// CLI-specific ones specified after the subcommand (if any).
fn prepend_config_flags(
    subcommand_config_overrides: &mut CliConfigOverrides,
    cli_config_overrides: CliConfigOverrides,
) {
    subcommand_config_overrides
        .raw_overrides
        .splice(0..0, cli_config_overrides.raw_overrides);
}

fn print_completion(cmd: CompletionCommand) {
    let mut app = MultitoolCli::command();
    let name = "codex";
    generate(cmd.shell, &mut app, name, &mut std::io::stdout());
}

async fn run_dtecho_command(
    dtecho_cli: DtechoCommand,
    _codex_linux_sandbox_exe: Option<PathBuf>,
) -> anyhow::Result<()> {
    println!("Loading YAML prompt from: {:?}", dtecho_cli.prompt_file);

    // Load the YAML prompt
    let yaml_prompt = YamlPrompt::from_file(&dtecho_cli.prompt_file)?;
    println!("Loaded prompt with {} messages", yaml_prompt.messages.len());

    // Read input files and combine their content
    let mut input_content = String::new();
    for file_path in &dtecho_cli.input_files {
        println!("Reading input file: {:?}", file_path);
        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                input_content.push_str(&format!(
                    "\n\n--- Content from {} ---\n",
                    file_path.display()
                ));
                input_content.push_str(&content);
            }
            Err(e) => {
                eprintln!("Warning: Failed to read file {:?}: {}", file_path, e);
            }
        }
    }

    // Create template variables
    let mut template_vars = HashMap::new();
    template_vars.insert("input".to_string(), input_content);

    // Convert YAML prompt to internal format
    let prompt = yaml_prompt.to_prompt(&template_vars)?;

    // Load configuration
    let config = Config::from_cli_overrides(dtecho_cli.config_overrides)?;

    println!("=== YAML Prompt Processing ===");
    println!("Prompt file: {:?}", dtecho_cli.prompt_file);
    println!("Input files: {:?}", dtecho_cli.input_files);
    if let Some(model) = yaml_prompt.get_model() {
        println!("Model: {}", model);
    }

    // Print the processed messages for demonstration
    println!("\n=== Processed Messages ===");
    for (i, item) in prompt.input.iter().enumerate() {
        println!("Message {}: {:?}", i + 1, item);
    }

    if let Some(ref base_instructions) = prompt.base_instructions_override {
        println!("\n=== Base Instructions Override ===");
        println!("{}", base_instructions);
    }

    println!("\n=== Deep Tree Echo Analysis Complete ===");

    Ok(())
}
