use std::{
    error::Error,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use askama::Template;
use clap::{Parser, Subcommand};

const HOME_MANAGER_FRAGMENTS_DIR: &str = "nix/home-manager";

const DARWIN_FLAKE_INPUTS: &str = include_str!("../../../nix/platforms/darwin/flake-inputs.nix");
const DARWIN_CONFIGURATION: &str =
    include_str!("../../../nix/platforms/darwin/darwin-configuration.nix");
const DARWIN_MODULE: &str = include_str!("../../../nix/platforms/darwin/darwin-module.nix");
const DARWIN_HOME_MANAGER_ZSH_INIT_CONTENT_PATH: &str =
    "nix/platforms/darwin/home-manager/zsh-init-content.nix";

const WSL_FLAKE_INPUTS: &str = include_str!("../../../nix/platforms/wsl/flake-inputs.nix");
const WSL_CONFIGURATION: &str = include_str!("../../../nix/platforms/wsl/wsl-configuration.nix");
const WSL_MODULE: &str = include_str!("../../../nix/platforms/wsl/wsl-module.nix");
const WSL_HOME_MANAGER_ZSH_INIT_CONTENT_PATH: &str =
    "nix/platforms/wsl/home-manager/zsh-init-content.nix";

const DARWIN_MODULE_PLACEHOLDER: &str = "    # DARWIN MODULE IS EMBEDDED HERE.";
const WSL_MODULE_PLACEHOLDER: &str = "    # WSL MODULE IS EMBEDDED HERE.";
const HOME_MANAGER_PLACEHOLDER: &str = "    # HOME MANAGER MODULE BODY IS EMBEDDED HERE.";
const PROGRAMS_ZSH_PLACEHOLDER: &str = "      # PROGRAMS ZSH BODY IS EMBEDDED HERE.";

#[derive(Template)]
#[template(path = "darwin-flake.nix", escape = "none")]
struct DarwinFlakeTemplate {
    flake_inputs: String,
    darwin_configuration: String,
}

#[derive(Template)]
#[template(path = "wsl-flake.nix", escape = "none")]
struct WslFlakeTemplate {
    flake_inputs: String,
    wsl_configuration: String,
}

#[derive(Parser, Debug)]
#[command(version, about = "Assemble Builder Kit generated flakes")]
struct Cli {
    #[command(subcommand)]
    command: CommandArgs,
}

#[derive(Subcommand, Debug)]
enum CommandArgs {
    /// Assemble the macOS nix-darwin/Home Manager flake.
    Darwin(OutArgs),

    /// Assemble the WSL NixOS/Home Manager flake.
    Wsl(OutArgs),
}

#[derive(clap::Args, Debug)]
struct OutArgs {
    /// Output directory for flake.nix and flake.lock.
    #[arg(long)]
    out: Option<PathBuf>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("kit-assembler: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match Cli::parse().command {
        CommandArgs::Darwin(args) => {
            let out_dir = args.out.unwrap_or_else(|| PathBuf::from("dist/darwin"));
            release(&out_dir, render_darwin_flake)
        }
        CommandArgs::Wsl(args) => {
            let out_dir = args.out.unwrap_or_else(|| PathBuf::from("dist/wsl"));
            release(&out_dir, render_wsl_flake)
        }
    }
}

fn release(
    out_dir: &Path,
    render: impl FnOnce() -> Result<String, Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(out_dir)?;

    let flake = render()?;

    write_atomically(&out_dir.join("flake.nix"), flake.as_bytes())?;
    write_lock_file(out_dir)?;

    println!("wrote {}", out_dir.join("flake.nix").display());
    println!("wrote {}", out_dir.join("flake.lock").display());

    Ok(())
}

fn render_darwin_flake() -> Result<String, Box<dyn Error>> {
    let zsh_init_content = fs::read_to_string(DARWIN_HOME_MANAGER_ZSH_INIT_CONTENT_PATH)?;
    let home_manager_body = render_home_manager_fragments(&zsh_init_content)?;

    let darwin_module = replace_placeholder(
        DARWIN_MODULE,
        HOME_MANAGER_PLACEHOLDER,
        &indent(&home_manager_body, 4),
    );

    let darwin_configuration = replace_placeholder(
        DARWIN_CONFIGURATION,
        DARWIN_MODULE_PLACEHOLDER,
        &indent(&render_import_fragment(&darwin_module), 4),
    );

    DarwinFlakeTemplate {
        flake_inputs: indent(DARWIN_FLAKE_INPUTS.trim_end(), 4),
        darwin_configuration: indent(&darwin_configuration, 8),
    }
    .render()
    .map_err(Into::into)
}

fn render_wsl_flake() -> Result<String, Box<dyn Error>> {
    let zsh_init_content = fs::read_to_string(WSL_HOME_MANAGER_ZSH_INIT_CONTENT_PATH)?;
    let home_manager_body = render_home_manager_fragments(&zsh_init_content)?;

    let wsl_module = replace_placeholder(
        WSL_MODULE,
        HOME_MANAGER_PLACEHOLDER,
        &indent(&home_manager_body, 4),
    );

    let wsl_configuration = replace_placeholder(
        WSL_CONFIGURATION,
        WSL_MODULE_PLACEHOLDER,
        &indent(&render_import_fragment(&wsl_module), 4),
    );

    WslFlakeTemplate {
        flake_inputs: indent(WSL_FLAKE_INPUTS.trim_end(), 4),
        wsl_configuration: indent(&wsl_configuration, 8),
    }
    .render()
    .map_err(Into::into)
}

fn render_home_manager_fragments(zsh_init_content: &str) -> Result<String, Box<dyn Error>> {
    let modules = read_nix_fragments(Path::new(HOME_MANAGER_FRAGMENTS_DIR))?
        .into_iter()
        .map(|(path, contents)| {
            let contents = if path.file_name() == Some(OsStr::new("base.nix")) {
                replace_placeholder(
                    &contents,
                    PROGRAMS_ZSH_PLACEHOLDER,
                    &indent(zsh_init_content.trim_end(), 6),
                )
            } else {
                contents
            };

            indent(&render_import_fragment(&contents), 2)
        })
        .collect::<Vec<_>>();

    Ok(format!("imports = [\n{}\n];", modules.join("\n")))
}

fn render_import_fragment(fragment: &str) -> String {
    let fragment = fragment.trim_end();
    let first_line = fragment.lines().next().unwrap_or_default().trim_end();

    if first_line.ends_with(':') {
        format!("(\n{}\n)", indent(fragment, 2))
    } else {
        fragment.to_owned()
    }
}

fn read_nix_fragments(directory: &Path) -> Result<Vec<(PathBuf, String)>, Box<dyn Error>> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;

    paths.retain(|path| {
        path.is_file() && path.extension().is_some_and(|extension| extension == "nix")
    });
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            let contents = fs::read_to_string(&path)?;
            Ok((path, contents))
        })
        .collect()
}

fn replace_placeholder(template: &str, placeholder: &str, replacement: &str) -> String {
    template.replace(placeholder, replacement)
}

fn indent(text: &str, spaces: usize) -> String {
    let prefix = " ".repeat(spaces);
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{prefix}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_lock_file(out_dir: &Path) -> Result<(), Box<dyn Error>> {
    let output_lock_file = out_dir.join("flake.lock");
    let flake_ref = format!("path:{}", out_dir.canonicalize()?.display());

    let status = Command::new("nix")
        .args([
            "flake",
            "lock",
            "--output-lock-file",
            output_lock_file
                .to_str()
                .ok_or("output lock path is not valid UTF-8")?,
            &flake_ref,
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("nix flake lock failed with status {status}").into())
    }
}

fn write_atomically(path: &Path, contents: &[u8]) -> Result<(), Box<dyn Error>> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, contents)?;

    if path.exists() {
        fs::remove_file(path)?;
    }

    fs::rename(tmp_path, path)?;
    Ok(())
}
