
use std::fs;
use std::path::Path;

fn main() {
    
    let mut  current_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    current_dir = Path::new(&current_dir).parent()
        .map_or(current_dir.clone(), |parent| {
            parent.to_str().unwrap().to_string()
        });

    


    if cfg!(debug_assertions) {
        //println!("cargo:rustc-cfg=tokio_unstable");
       // println!("cargo:rustc-cfg=feature=\"dev-tools\"");
        
    } else {
        // #[cfg(target_os = "windows")]
        
    }

    let dir = r"F:\tdlib\td\tdlib";
    println!("cargo:include={dir}\\include");
    println!("cargo:rustc-link-search=native={dir}\\bin");
    println!("cargo:rustc-link-search=native={dir}\\lib");
    println!("cargo:rustc-link-lib=dylib=tdjson");
    // println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}\\bin");

    println!("cargo:rerun-if-changed=build.rs");
    // println!("cargo:rerun-if-env-changed=LOCAL_TDLIB_PATH");
    // println!("cargo:rerun-if-env-changed=PROFILE");

}

fn is_debian() -> bool {
    match fs::read_to_string("/etc/os-release") {
        Ok(content) => content.contains("ID=debian") || content.contains("Debian"),
        Err(_) => false,
    }
}

fn is_ubuntu() -> bool {
    match fs::read_to_string("/etc/os-release") {
        Ok(content) => content.contains("ID=ubuntu") || content.contains("Ubuntu"),
        Err(_) => false,
    }
}

