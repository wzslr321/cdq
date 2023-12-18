use clap::Parser;
use walkdir::WalkDir;

mod cdq {
    use colored::Colorize;
    use core::fmt;
    use std::io::prelude::*;
    use std::{
        env::{self},
        fs,
        io::{self},
        path::{self, PathBuf},
    };

    const CDQ_CONFIG_NAME: &'static str = "cdq.config";
    const SHELL_ENV_NAME: &'static str = "SHELL";
    const SHELL_TYPE_TO_FILE: &[(ShellType, &str)] = &[
        (ShellType::Zsh, ".zshrc"),
        // add other shell types here...
    ];

    const CDQ_FUNC_EXECUTOR_ZSH: &'static str = r#"
            cdq() {
                local output=$(~/Remi/rust/cdq/target/release/cdq $1)
                echo $output
                local dir=$(echo $output | awk -F'Path=' '{print $2}')
                if [ -d "$dir" ]; then
                    echo "Proceeding to the $dir"
                    cd "$dir"
                fi
            }
        "#;

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
        pub fn query(msg: &str) {
            println!("{} {}", "[CDQ | Query]:".bright_green(), msg);
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

    #[derive(PartialEq)]
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
            Some(shell_type) => {
                Logger::info(&format!("Detected shell: {}", shell_type));
                let mut default_config_file_path = home::home_dir().unwrap().as_os_str().to_owned();
                default_config_file_path.push(format!(
                    "/{}",
                    SHELL_TYPE_TO_FILE
                        .iter()
                        .find_map(|(st, file)| {
                            if *st == *shell_type {
                                Some(*file)
                            } else {
                                None
                            }
                        })
                        .unwrap()
                ));
                Logger::info(&format!(
                    "Default config file for {} is: {}",
                    shell_type,
                    default_config_file_path.clone().into_string().unwrap()
                ));
                println!("Proceed? [Type Y/n]:");
                let mut ans = String::new();
                io::stdin().read_line(&mut ans)?;
                if ans.trim() == "Y".to_lowercase() {
                    Ok(
                        path::Path::new(&default_config_file_path.into_string().unwrap())
                            .to_path_buf(),
                    )
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Aborting due to invalid input...",
                    ))
                }
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Aborting due to invalid shell...",
            )),
        }
    }

    fn detect_shell_type() -> Option<ShellType> {
        match env::var(SHELL_ENV_NAME) {
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
        let mut shell_config_path = ask_for_shell_config_path(&shell_type);
        while shell_config_path.is_err() {
            shell_config_path = ask_for_shell_config_path(&shell_type);
        }

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

    fn try_write_to_shell_config(path: PathBuf) -> io::Result<()> {
        let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
        if let Err(e) = writeln!(file, "{}", CDQ_FUNC_EXECUTOR_ZSH) {
            Logger::error(&format!("Couldn't write to file: {}", e));
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Aborting due to invalid input...",
            ))
        } else {
            Ok(())
        }
    }

    pub fn setup() -> io::Result<()> {
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
                Ok(())
            }
            None => {
                Logger::info("It appears that you are running CDQ for the first time");
                let user_shell = get_user_shell();
                Logger::info(&format!(
                    "Detected shell: {}, with config file at: {}",
                    user_shell.name,
                    user_shell.path.display()
                ));
                Logger::info(&format!(
                    "CDQ wants to write following lines:{}",
                    CDQ_FUNC_EXECUTOR_ZSH
                ));
                println!("Proceed? [Type Y/n]:");
                let mut ans = String::new();
                io::stdin().read_line(&mut ans)?;
                if ans.trim() == "Y".to_lowercase() {
                    if let Err(e) = try_write_to_shell_config(user_shell.path) {
                        return Err(e);
                    }
                    Ok(())
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "Aborting due to invalid input...",
                    ))
                }
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

    match cdq::setup() {
        Ok(_) => cdq::Logger::info("Setup finished correctly"),
        Err(_) => cdq::Logger::error("Setup failed"),
    };

    let dir_name = args.name;
    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(last_component) = path.file_name() {
            if last_component.to_str().unwrap() == dir_name {
                cdq::Logger::query(&format!("{}", path.display()));
            }
        }
    }
}
