use std::env;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use tdlib_rs_gen::generate_rust_code;
use tdlib_rs_parser::parse_tl_file;
use tdlib_rs_parser::tl::Definition;


static lib_file:&str = include_str!("../temp/lib.rs");
static observer_file:&str = include_str!("../temp/observer.rs");
static tdjson_file:&str = include_str!("../temp/tdjson.rs");

static cargo_file:&str = include_str!("../temp/Cargo.toml.txt");

fn load_tl(file: &str) -> std::io::Result<Vec<Definition>> {
    let mut file = File::open(file)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    println!("{}", contents);
    Ok(parse_tl_file(contents)
        .filter_map(|d| match d {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("TL: parse error: {e:?}");
                None
            }
        })
        .collect())
}




fn main() -> std::io::Result<()> {
    let definitions = load_tl(r"F:\code\rust\tdlib-rs\tdlib-rs\tl\api_1.8.62.tl")?;
    let mut path = Path::new(r"F:\code\rust\tdlib_test").to_path_buf();
    let mut src_path = path.join("src");
    generate_rust_code(&mut src_path, &definitions, false)?;
    save(src_path.join("lib.rs"),lib_file)?;
    save(src_path.join("observer.rs"),observer_file)?;
    save(src_path.join("tdjson.rs"),tdjson_file)?;
    save(path.join("Cargo.toml"),cargo_file)?;
    Ok(())
}

fn save(file_path:PathBuf,contents:&str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new().write(true).create(true).open(file_path)?;
    writeln!(file, "{}", contents)?;
    Ok(())
}


