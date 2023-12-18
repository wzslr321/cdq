use core::fmt;
use std::{env, io::{self, Write}, path};

use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    name: String,
}

enum ShellType {
    Bash,
    Fish,
    Zsh,
    Other,
}

impl fmt::Display for ShellType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ShellType::Bash => write!(f, "Bash"),
            ShellType::Fish => write!(f, "Fish"),
            ShellType::Zsh => write!(f, "Zsh"),
            ShellType::Other => write!(f, "Other"),
        }
    }
}

struct Shell {
    name: ShellType,
    path: path::PathBuf,
}

impl Shell {
    fn new(name: ShellType, path: path::PathBuf) -> Self { Self { name, path } }
}

fn ask_for_shell_config_path(detected_shell:&Option<ShellType>)  -> io::Result<path::PathBuf> {
    match detected_shell {
        Some(shell_type) => io::stdout().write_all(format!("Detected shell: {}", shell_type).as_bytes()).unwrap(),
        None => io::stdout().write_all(b"Failed to detect shell").unwrap(),
    };

    let mut path = String::new();
    io::stdin().read_line(&mut path)?;

    Ok(path::Path::new(&path).to_path_buf())
}

fn detect_shell_type() -> Option<ShellType>{
    let shell_env_name = "SHELL";

    match env::var(shell_env_name) {
        Ok(value) => match value.split('/').last().unwrap_or_else(|| "unknown") {
            "zsh" => Some(ShellType::Zsh),
            "bash" => Some(ShellType::Bash),
            "fish" => Some(ShellType::Fish),
            _ => None,
        },
        Err(_) => None,
    }
}

fn get_user_shell() -> Shell {
    let shell_type = detect_shell_type();
    let shell_config_path = ask_for_shell_config_path(&shell_type);

    Shell::new(shell_type.unwrap_or_else(|| ShellType::Other), shell_config_path.unwrap())
}

fn main() {
    let args = Args::parse();
    let user_shell = get_user_shell();
    println!("Detected shell: {}, with config file at: {}", user_shell.name, user_shell.path.display());


    let dir_name = args.name;
    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(last_component) = path.file_name() {
            if last_component.to_str().unwrap() == dir_name {
                println!("found {}. Path={}", dir_name, path.display());
            }
        }
    }
}
