use std::borrow::Cow;
use std::env;

fn main() {
    built::write_built_file().expect("failed to acquire build-time information");

    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("target_os not defined!");
    // on armv6 we need to link with libatomic
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("target_arch not defined!");

    if target_os == "linux" && target_arch == "arm" {
        // Embrace the atomic capability library across various platforms.
        // For instance, on certain platforms, llvm has relocated the atomic of the arm32 architecture to libclang_rt.builtins.a
        // while some use libatomic.a, and others use libatomic_ops.a.
        let atomic_name = match env::var("DEP_ATOMIC") {
            Ok(atomic_name) => Cow::Owned(atomic_name),
            Err(_) => Cow::Borrowed("atomic"),
        };
        println!("cargo:rustc-link-lib={atomic_name}");
    }

    // Link with libs needed on Windows
    if target_os == "windows" {
        // https://github.com/microsoft/mimalloc/blob/af21001f7a65eafb8fb16460b018ebf9d75e2ad8/CMakeLists.txt#L487
        for lib in ["psapi", "shell32", "user32", "advapi32", "bcrypt"] {
            println!("cargo:rustc-link-lib={lib}");
        }
    }
}
