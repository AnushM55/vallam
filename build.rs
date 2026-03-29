use std::env;
use std::path::PathBuf;

fn main() {
    let libdir = env::var("SWC_LIBDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            manifest_dir
                .parent()
                .unwrap()
                .join("builddir")
                .join("libswc")
        });

    // Tell cargo where to find libswc.so
    println!("cargo:rustc-link-search=native={}", libdir.display());
    println!("cargo:rustc-link-lib=dylib=swc");

    // Embed rpath so the binary can find libswc.so at runtime
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir.display());

    // Link private dependencies of libswc
    println!("cargo:rustc-link-lib=dylib=wld");
    println!("cargo:rustc-link-lib=dylib=pixman-1");
    println!("cargo:rustc-link-lib=dylib=xkbcommon");
    println!("cargo:rustc-link-lib=dylib=drm");
    println!("cargo:rustc-link-lib=dylib=wayland-client");
    println!("cargo:rustc-link-lib=dylib=input");
    println!("cargo:rustc-link-lib=dylib=udev");

    // Xwayland deps
    println!("cargo:rustc-link-lib=dylib=xcb-composite");
    println!("cargo:rustc-link-lib=dylib=xcb-ewmh");
    println!("cargo:rustc-link-lib=dylib=xcb-icccm");
    println!("cargo:rustc-link-lib=dylib=xcb");

    // System libs
    println!("cargo:rustc-link-lib=dylib=wayland-server");

    // Tell cargo to rerun if build.rs changes
    println!("cargo:rerun-if-changed=build.rs");
}
