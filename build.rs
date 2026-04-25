use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    tauri_build::build();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let libs_dir = manifest_dir.join("libs");
    let target = env::var("TARGET").expect("TARGET");
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    // Do not add native=. here: the crate root may still contain an old libwg-go-x86_64.so and the
    // linker would pick it before OUT_DIR. Only link native libraries from OUT_DIR below.
    println!("cargo:rerun-if-changed={}", libs_dir.display());

    let roots = lib_search_roots(&libs_dir, &target, &os);
    for r in &roots {
        if r.is_dir() {
            println!("cargo:rerun-if-changed={}", r.display());
        }
    }

    let wg = discover_wg_go(&roots, &os, &target).unwrap_or_else(|m| {
        panic!(
            "{m}\nExpected libwg-go.so (Windows: libwg-go.dll) or libwg-go.a under:\n  macOS: libs/x86_64-apple-darwin/ or libs/aarch64-apple-darwin/\n  Linux: libs/Linux/\n  (then libs/ as fallback)"
        )
    });

    let wg_link_kind = match &wg {
        WgGoArtifact::Shared(p) => {
            prepare_wg_go_shared(&out_dir, p, &os);
            "dylib"
        }
        WgGoArtifact::Static(p) => {
            prepare_wg_go_static(&out_dir, p);
            "static"
        }
    };

    let fakeip = discover_fakeip(&roots, &arch)
        .unwrap_or_else(|m| {
            panic!("{m}\nExpected libfakeip.a under the same dirs as wg-go (macOS triple subdir, libs/Linux, or libs/)")
        });
    prepare_fakeip(&out_dir, &fakeip);

    // OUT_DIR must come first (and be the only -L for wg-go / fakeip).
    println!("cargo:rustc-link-search=native={}", out_dir.display());

    // macOS: Go-built libwg-go often embeds LC_ID_DYLIB as libwg-go-x86_64.so; normalize to @rpath/libwg-go.so
    // and pass rpath to this crate's OUT_DIR (where we copy libwg-go.so).
    if os == "macos" {
        if let Ok(abs) = fs::canonicalize(&out_dir) {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", abs.display());
        }
    }

    println!("cargo:rustc-link-lib={wg_link_kind}=wg-go");
    println!("cargo:rustc-link-lib=static=fakeip");

    if os == "macos" {
        println!("cargo:rustc-link-lib=framework=Security");
        println!("cargo:rustc-link-lib=framework=CoreFoundation");
        println!("cargo:rustc-link-lib=framework=NetworkExtension");
    }
}

/// Platform layout:
/// - macOS: `libs/x86_64-apple-darwin/` (Intel), `libs/aarch64-apple-darwin/` (Apple Silicon); current `TARGET` is tried first.
/// - Linux: `libs/Linux/`
/// - Other: `libs/<TARGET>/`, then `libs/`.
fn lib_search_roots(libs_dir: &Path, target_triple: &str, os: &str) -> Vec<PathBuf> {
    let mut v = Vec::new();
    match os {
        "macos" => {
            v.push(libs_dir.join(target_triple));
            if target_triple != "x86_64-apple-darwin" {
                v.push(libs_dir.join("x86_64-apple-darwin"));
            }
            if target_triple != "aarch64-apple-darwin" {
                v.push(libs_dir.join("aarch64-apple-darwin"));
            }
        }
        "linux" => {
            v.push(libs_dir.join("Linux"));
        }
        _ => {
            v.push(libs_dir.join(target_triple));
        }
    }
    v.push(libs_dir.to_path_buf());
    v
}

fn list_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        if let Ok(rd) = fs::read_dir(root) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_file() {
                    out.push(p);
                }
            }
        }
    }
    out
}

fn arch_aliases(rust_arch: &str) -> &'static [&'static str] {
    match rust_arch {
        "x86_64" => &["x86_64", "amd64"],
        "aarch64" => &["aarch64", "arm64"],
        "arm" => &["arm", "armv7", "armeabi-v7a"],
        "x86" | "i686" => &["i686", "x86", "i386"],
        _ => &[],
    }
}

enum WgGoArtifact {
    Shared(PathBuf),
    Static(PathBuf),
}

/// Shared object must be named exactly `libwg-go.so` (or `libwg-go.dll` on Windows).
/// Static fallback: `libwg-go.a`.
fn discover_wg_go(roots: &[PathBuf], os: &str, target: &str) -> Result<WgGoArtifact, String> {
    let shared_name = if os == "windows" {
        "libwg-go.dll"
    } else {
        "libwg-go.so"
    };

    for root in roots {
        let p = root.join(shared_name);
        if p.is_file() {
            return Ok(WgGoArtifact::Shared(p));
        }
    }

    for root in roots {
        let p = root.join("libwg-go.a");
        if p.is_file() {
            return Ok(WgGoArtifact::Static(p));
        }
    }

    Err(format!(
        "missing {shared_name} or libwg-go.a (see libs layout for target {target})"
    ))
}

fn prepare_wg_go_shared(out_dir: &Path, src: &Path, os: &str) {
    fs::create_dir_all(out_dir).expect("create OUT_DIR");
    let dst = if os == "windows" {
        out_dir.join("libwg-go.dll")
    } else {
        out_dir.join("libwg-go.so")
    };
    // Always copy (never symlink): install_name_tool must edit the exact file the linker sees, and
    // we must not mutate the copy under libs/.
    copy_into_out(src, &dst);

    // rustc/clang on macOS resolve -lwg-go to libwg-go.dylib; keep a sibling symlink to libwg-go.so.
    #[cfg(unix)]
    if os == "macos" {
        let dy = out_dir.join("libwg-go.dylib");
        let _ = fs::remove_file(&dy);
        std::os::unix::fs::symlink("libwg-go.so", &dy).expect("symlink libwg-go.dylib");
        fix_macho_wg_go_install_id(&dst);
    }
}

/// Rewrites LC_ID_DYLIB / self-references so the main binary loads `@rpath/libwg-go.so` instead of
/// e.g. `libwg-go-x86_64.so` (common for `go build -buildmode=c-shared -o libwg-go-x86_64.so`).
#[cfg(unix)]
fn fix_macho_wg_go_install_id(path: &Path) {
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let new_id = "@rpath/libwg-go.so";
    let ps = path.to_str().expect("UTF-8 path");

    // Set install name first.
    let st = Command::new("install_name_tool")
        .args(["-id", new_id, ps])
        .status()
        .expect("run install_name_tool -id");
    if !st.success() {
        panic!(
            "install_name_tool -id failed for {} (re-sign or chmod?)",
            path.display()
        );
    }

    // Replace any remaining install-name lines reported by otool -D (basename or absolute path).
    if let Ok(out) = Command::new("otool").args(["-D", ps]).output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
                let old = line.trim();
                if old.is_empty() || old == new_id {
                    continue;
                }
                let _ = Command::new("install_name_tool")
                    .args(["-change", old, new_id, ps])
                    .status();
            }
        }
    }

    for old in [
        "libwg-go-x86_64.so",
        "libwg-go-arm64.so",
        "libwg-go-aarch64.so",
        "libwg-go-amd64.so",
    ] {
        let _ = Command::new("install_name_tool")
            .args(["-change", old, new_id, ps])
            .status();
    }
}

fn prepare_wg_go_static(out_dir: &Path, src: &Path) {
    fs::create_dir_all(out_dir).expect("create OUT_DIR");
    let dst = out_dir.join("libwg-go.a");
    link_or_copy(src, &dst);
}

fn discover_fakeip(roots: &[PathBuf], arch: &str) -> Result<PathBuf, String> {
    let files = list_files(roots);
    let mut candidates: Vec<(u8, PathBuf)> = Vec::new();

    for p in files {
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !name.starts_with("libfakeip") || !name.ends_with(".a") {
            continue;
        }
        if name == "libfakeip.a" {
            candidates.push((0, p));
            continue;
        }
        let aliases = arch_aliases(arch);
        let mut scored = None;
        for (i, a) in aliases.iter().enumerate() {
            if name.starts_with(&format!("libfakeip-{a}."))
                || name.starts_with(&format!("libfakeip_{a}."))
            {
                scored = Some((i + 1) as u8);
                break;
            }
        }
        candidates.push((scored.unwrap_or(100), p));
    }

    candidates.sort_by_key(|(s, _)| *s);
    candidates
        .into_iter()
        .find(|(s, _)| *s < u8::MAX)
        .map(|(_, p)| p)
        .ok_or_else(|| format!("no libfakeip*.a for arch={arch}"))
}

fn prepare_fakeip(out_dir: &Path, src: &Path) {
    fs::create_dir_all(out_dir).expect("create OUT_DIR");
    let dst = out_dir.join("libfakeip.a");
    link_or_copy(src, &dst);
}

fn copy_into_out(src: &Path, dst: &Path) {
    let _ = fs::remove_file(dst);
    let src_abs = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    fs::copy(&src_abs, dst).unwrap_or_else(|e| {
        panic!(
            "copy {} -> {}: {e}",
            src_abs.display(),
            dst.display()
        );
    });
}

fn link_or_copy(src: &Path, dst: &Path) {
    let _ = fs::remove_file(dst);
    let src_abs = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    #[cfg(unix)]
    {
        if std::os::unix::fs::symlink(&src_abs, dst).is_err() {
            fs::copy(&src_abs, dst).unwrap_or_else(|e| {
                panic!("could not symlink or copy {} -> {}: {e}", src_abs.display(), dst.display())
            });
        }
    }
    #[cfg(not(unix))]
    {
        fs::copy(&src_abs, dst).unwrap_or_else(|e| {
            panic!("copy {} -> {}: {e}", src_abs.display(), dst.display());
        });
    }
}
