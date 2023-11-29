use clap::Parser;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    name: String,
}

fn main() {
    let args = Args::parse();

    let dir_name = args.name;
    for entry in WalkDir::new("foo") {
        let entry = entry.unwrap();
        let path = entry.path();
        if let Some(last_component) = path.file_name() {
            if last_component.to_str().unwrap() == dir_name {
                println!("found {}. Path={}", dir_name, path.display());
            }
        }
    }
}
