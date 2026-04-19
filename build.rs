fn main() {
    tauri_build::build();

    println!("cargo:rustc-link-search=native=.");
    // 告诉 Rust 去哪里找静态库
    println!("cargo:rustc-link-search=native=libs");
    // 链接生成的静态库
    println!("cargo:rustc-link-lib=static=wg-go");
    println!("cargo:rustc-link-lib=static=fakeip");

    // Link required Apple frameworks
    println!("cargo:rustc-link-lib=framework=Security");
    println!("cargo:rustc-link-lib=framework=CoreFoundation");
    println!("cargo:rustc-link-lib=framework=NetworkExtension");
}
