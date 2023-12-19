use std::io::{self};

use cdq::Cache;
use clap::Parser;
use walkdir::WalkDir;

const CDQ_CACHE_FILE_NAME: &'static str = ".cdq.cache";

/// Handles setup of shell-specific functions
/// Exposes Logger
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

    // TODO  Try to move these consts somewhere as it looks disgusting
    const CDQ_CONFIG_FILE_NAME: &'static str = ".cdq.config";
    const SHELL_ENV_NAME: &'static str = "SHELL";
    const SHELL_TYPE_TO_FILE: &[(ShellType, &str)] = &[
        (ShellType::Zsh, ".zshrc"),       // ~/.zshrc
        (ShellType::Bash, ".bashrc"),     // ~/.bashrc
        (ShellType::Fish, "config.fish"), // ~/.config/fish/config.fish
    ];

    /// Exposes functions to log messages with different meaning
    ///
    /// `pub fn info(msg: &str)`
    /// `pub fn warn(msg: &str)`
    /// `pub fn error(msg: &str)`
    /// `pub fn query(msg: &str)`
    /// `pub fn debug(msg: &str)`
    pub enum Logger {}

    // TODO fix full path, make ~/.config/cdq/... | as fish does it
    //
    /// Config stored at `~/.config/$CDQ_CONFIG_NAME`
    /// Contains persistent information about user's environment,
    /// needed for **cdq** execution and its proper interaction with shell
    pub struct Config {
        /// Contains information about user's shell and its config path
        shell: Shell,
    }

    pub struct Cache<'a> {
        path: &'a str,
    }

    impl<'a> Cache<'a> {
        pub fn new(path: &'a str) -> Self {
            Self { path }
        }

        pub fn push(&self, _pattern: &str) {
            let _file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open(self.path)
                .unwrap();
        }
        pub fn try_pop() {}
        pub fn try_pop_until() {}
    }

    /// Retrieves **cdq** config from its config file, if it exists
    /// Configures it and saves the configuration to the config file otherwise.
    ///
    /// Flow:
    /// - Retrieve home directory
    /// - Try to read user's config
    /// - Finish if config exists
    /// - Retrieve user's shell information
    /// - Ask for permission to write to shell config file
    /// - Save config if allowed, otherwise terminate and
    ///   log information about the need to write it by user themselves
    pub fn setup() -> io::Result<()> {
        let curr_dir = get_current_dir().unwrap();

        let home_path = home::home_dir();
        if home_path.is_none() {
            // TODO it should ask user for his home path here,
            // as otherwise .unwrap() in the following line will fail
            Logger::warn("Failed to retrieve HOME directory");
        }
        let mut path_to_cdq = home_path.unwrap().as_os_str().to_owned();
        path_to_cdq.push(format!("/.config/{}", CDQ_CONFIG_FILE_NAME));
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
                let executor = get_cdq_func_executor(user_shell.name, &curr_dir);
                Logger::info(&format!("CDQ wants to write following lines:{}", executor));
                println!("Proceed? [Type Y/n]:");
                let mut ans = String::new();
                io::stdin().read_line(&mut ans)?;
                if ans.trim() == "Y".to_lowercase() {
                    if let Err(e) = try_write_to_shell_config(user_shell.path.clone(), executor) {
                        return Err(e);
                    }
                    if let Err(e) = try_write_to_cdq_config(&user_shell, &path_to_cdq.into()) {
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

    impl Logger {
        /// Function for outputing messages, that
        /// contain information about current state of the program
        pub fn info(msg: &str) {
            println!("{} {}", "[CDQ][INFO]:".cyan(), msg);
        }

        /// Function for outputing warnings, that may affect
        /// the behavior of the program and lead to its termination,
        /// but allows to continue execution.
        pub fn warn(msg: &str) {
            println!("{} {}", "[CDQ][WARN]:".yellow(), msg);
        }

        /// Function for outputing errors, after which the
        /// program gets terminated.
        pub fn error(msg: &str) {
            println!("{} {}", "[CDQ][ERROR]:".red(), msg);
        }

        /// Function for outputing performed queries, in predefined
        /// format that is looked for by the shell, to intercept
        /// values needed to execute **cd** command.
        pub fn query(msg: &str) {
            println!("{} {}", "[CDQ][QUERY]:".bright_green(), msg);
        }

        /// Function for logging debug-specific messages, not visible
        /// in non-debug builds
        #[cfg(debug_assertions)]
        pub fn debug(msg: &str) {
            println!("{} {}", "[CDQ][DEBUG]:".green(), msg);
        }
    }

    impl Config {
        // TODO
        // 1. Consider changing it to `from_file`
        // 2. Add validation
        //
        /// Constructs `Config` based on stringified config file.
        fn from_string(content: &str) -> Self {
            let lines: Vec<&str> = content.trim().split('\n').collect();
            let shell_type = lines[0];
            let config_path_line: Vec<&str> = lines[1].split('=').collect();
            Self {
                shell: Shell::new(
                    ShellType::from(shell_type),
                    path::Path::new(config_path_line[1]).to_path_buf(),
                ),
            }
        }
    }

    /// Shell types supported out-of-the-box are
    /// - Bash
    /// - Fish
    /// - Zsh
    /// Other need to be configured by the user
    #[derive(PartialEq, Copy, Clone)]
    enum ShellType {
        Bash,
        Fish,
        Zsh,
        Other,
    }

    impl ShellType {
        /// Constructs `ShellType` based on `_str` read
        /// from the config file.
        fn from(_str: &str) -> Self {
            Self::Zsh
        }
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
                "Shell type: {}\nShell config file path: {}",
                self.name,
                self.path.display()
            );
            write!(f, "{}", stringified)
        }
    }

    // functions here goes according to the flow

    fn try_read_user_config(path: PathBuf) -> Option<Config> {
        match path.exists() {
            true => Some(read_user_config(path)),
            false => None,
        }
    }

    fn read_user_config(path: PathBuf) -> Config {
        let content = fs::read_to_string(path);

        match content {
            Ok(_) => Config::from_string(&content.unwrap()),
            Err(_) => {
                Logger::error("Attempt to read a non-exsitent config file");
                panic!()
            }
        }
    }

    // no config things below

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

    fn ask_for_shell_config_path(detected_shell: &Option<ShellType>) -> io::Result<path::PathBuf> {
        match detected_shell {
            Some(shell_type) => {
                Logger::info(&format!("Detected shell: {}", shell_type));
                // TODO as_os_str may be useless, read PathBuf docs
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

    fn try_write_to_shell_config(path: PathBuf, executor: Box<String>) -> io::Result<()> {
        let mut file = fs::OpenOptions::new().append(true).open(path).unwrap();
        if let Err(e) = writeln!(file, "{}", executor) {
            Logger::error(&format!("Couldn't write to file: {}", e));
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Failed to write to file",
            ))
        } else {
            Ok(())
        }
    }

    fn try_write_to_cdq_config(shell: &Shell, path: &PathBuf) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        if let Err(e) = writeln!(
            file,
            "{}",
            format!(
                r#"SHELL={}
SHELL_CONFIG_PATH={}"#,
                shell.name,
                shell.path.display()
            )
        ) {
            Logger::error(&format!("Couldn't write to file: {}", e));
            Err(io::Error::new(
                io::ErrorKind::WriteZero,
                "Failed to write to file",
            ))
        } else {
            Ok(())
        }
    }

    //
    fn get_current_dir() -> io::Result<PathBuf> {
        match env::current_dir() {
            Ok(dir) => {
                Logger::debug(&format!("Current dir is: {}", dir.display()));
                Ok(dir)
            }
            Err(e) => Err(e),
        }
    }

    fn get_cdq_func_executor(shell_type: ShellType, curr_dir: &PathBuf) -> Box<String> {
        // TODO
        //
        // 1. Write configs for Bash and Fish
        // 2. Get program directory, when executing CDQ for thr first time,
        //    in order to use it for specyfing the `output` variable below
        let cdq_func_executor_zsh = Box::new(format!(
            r#"
// GENERATED BY CDQ
cdq() {{
    local output=$({}/target/release/cdq $1)
    echo $output
    local dir=$(echo $output | awk -F'Path=' '{{print $2}}')
    if [ -d "$dir" ]; then
        echo "Proceeding to the $dir"
        cd "$dir"
    fi
}}
"#,
            curr_dir.display()
        ));

        match shell_type {
            ShellType::Bash => cdq_func_executor_zsh,
            ShellType::Fish => cdq_func_executor_zsh,
            ShellType::Zsh => cdq_func_executor_zsh,
            ShellType::Other => cdq_func_executor_zsh,
        }
    }
}

// TODO
// 1. Add support for --list-stack, which prints
// current stack of **cdq** execution
// 2. Add support for --pop, which pops one
// execution information from the execution stack
// 3. Add support for --pop-until [ID], which pops
// execution information from the execution stack,
// untill execution with [ID] that user retrieved
// from --list-stack
#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    /// Pattern describing desireed destination
    /// after executing cdq
    ///
    /// Examples:
    /// - foo
    /// - ~/bar
    pattern: Option<String>,

    /// Go back to the directory **cdq** was previously executed.
    /// Makes **cdq** do nothing if it is first time running it
    /// in the current session.
    #[arg(short, long)]
    back: Option<bool>,

    #[arg(short, long)]
    back_until: Option<String>,
}

enum Command {
    ChangeForward,
    ChangeBackwards,
    ChangeBackwardsUntil,
}

fn ensure_arg_or_terminate<T>(arg: Option<T>, arg_name: &str) -> T {
    match arg {
        Some(value) => value,
        None => {
            cdq::Logger::error(&format!("Missing argument value for: {}", arg_name));
            std::process::exit(0)
        }
    }
}

fn change_forward(pattern_arg: Option<&str>, cache: &Cache) {
    let pattern = ensure_arg_or_terminate(pattern_arg, "pattern");

    for entry in WalkDir::new(".") {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(last_component) = path.file_name() {
            if last_component.to_str().unwrap() == pattern {
                cdq::Logger::query(&format!("{}", path.display()));
            }
        }
    }

    cache.push(pattern);
}

fn change_backwards() {
    cdq::Cache::try_pop()
}

fn change_backwards_until() {
    cdq::Cache::try_pop_until()
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    match cdq::setup() {
        Ok(_) => cdq::Logger::info("Setup finished correctly"),
        Err(e) => {
            cdq::Logger::error(&format!("Setup failed. Error: {}", e));
            return Err(e);
        }
    };

    let command = if args.back.is_some() {
        Command::ChangeBackwards
    } else if args.back_until.is_some() {
        Command::ChangeBackwardsUntil
    } else {
        Command::ChangeForward
    };

    let cache = cdq::Cache::new(CDQ_CACHE_FILE_NAME);

    match command {
        Command::ChangeForward => change_forward(args.pattern.as_deref(), &cache),
        Command::ChangeBackwards => change_backwards(),
        Command::ChangeBackwardsUntil => change_backwards_until(),
    }

    Ok(())
}
