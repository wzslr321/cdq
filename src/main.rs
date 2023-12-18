use clap::Parser;
use walkdir::WalkDir;

mod cdq {
    use colored::Colorize;
    use core::fmt;
    use std::{
        env::{self},
        fs,
        io::{self},
        path::{self, PathBuf},
    };

    const CDQ_CONFIG_NAME: &'static str = "cdq.config";

    pub enum Logger {}

    impl Logger {
        pub fn info(msg: &str) {
            println!("{} {}", "[CDQ]:".cyan(), msg);
        }
        pub fn warn(msg: &str) {
            println!("{} {}", "[CDQ]:".yellow(), msg);
        }
        pub fn error(msg: &str) {
            println!("{} {}", "[CDQ]:".red(), msg);
        }
        #[cfg(debug_assertions)]
        pub fn debug(msg: &str) {
            println!("{} {}", "[CDQ | Debug]:".green(), msg);
        }
    }
    pub struct Config {
        shell: Shell,
    }

    impl Config {
        fn from_string(_content: &str) -> Self {
            Self {
                shell: Shell::new(ShellType::Other, PathBuf::new()),
            }
        }
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
        fn new(name: ShellType, path: path::PathBuf) -> Self {
            Self { name, path }
        }
    }

    impl fmt::Display for Shell {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            let stringified = format!(
                "Shell type: {}, with config at: {}",
                self.name,
                self.path.display()
            );
            write!(f, "{}", stringified) 
        }
    }

    fn ask_for_shell_config_path(detected_shell: &Option<ShellType>) -> io::Result<path::PathBuf> {
        match detected_shell {
            Some(shell_type) => println!("Detected shell: {}", shell_type),
            None => println!("Failed to detect shell"),
        };
        println!("Type path to your shell config file");

        let mut path = String::new();
        io::stdin().read_line(&mut path)?;

        Ok(path::Path::new(&path).to_path_buf())
    }

    fn detect_shell_type() -> Option<ShellType> {
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

        Shell::new(
            shell_type.unwrap_or_else(|| ShellType::Other),
            shell_config_path.unwrap(),
        )
    }

    fn read_user_config(path: PathBuf) -> Config {
        let content = fs::read_to_string(path);

        if content.is_err() {
            Logger::error("Attempt to read a non-exsitent config file");
        }

        Config::from_string(&content.unwrap())
    }

    fn try_read_user_config(path: PathBuf) -> Option<Config> {
        match path.exists() {
            true => Some(read_user_config(path)),
            false => None,
        }
    }

    pub fn setup() {
        let home_path = home::home_dir();
        if home_path.is_none() {
            Logger::warn("Failed to retrieve HOME directory");
        }
        let mut path_to_cdq = home_path.unwrap().as_os_str().to_owned();
        path_to_cdq.push(format!("/.config/{}", CDQ_CONFIG_NAME));
        let user_config = try_read_user_config(path_to_cdq.clone().into());

        match user_config {
            Some(config) => {
                let path = String::from(&path_to_cdq.clone().into_string().unwrap());
                Logger::debug(&format!("Found config at: {}", &path));
                Logger::debug(&format!("Shell Config:\n{}", config.shell));
            }
            None => {
                Logger::info("It appears that you are running CDQ for the first time");
                let user_shell = get_user_shell();
                println!(
                    "Detected shell: {}, with config file at: {}",
                    user_shell.name,
                    user_shell.path.display()
                );
            }
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    name: String,
}

fn main() {
    let args = Args::parse();

    cdq::setup();

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
