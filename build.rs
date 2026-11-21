use std::{env, fs, path::PathBuf, process::Command};

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
}

fn resource(output: &mut Vec<u8>, kind: u16, id: u16, data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_le_bytes());
    output.extend_from_slice(&32_u32.to_le_bytes());
    for word in [0xffff_u16, kind, 0xffff, id] {
        output.extend_from_slice(&word.to_le_bytes());
    }
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.extend_from_slice(&0x1030_u16.to_le_bytes());
    output.extend_from_slice(&0_u16.to_le_bytes());
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.extend_from_slice(&0_u32.to_le_bytes());
    output.extend_from_slice(data);
    while output.len() % 4 != 0 {
        output.push(0);
    }
}

fn main() {
    println!("cargo:rerun-if-changed=assets/notvas.ico");
    println!("cargo:rerun-if-env-changed=WINDRES");
    println!("cargo:rerun-if-env-changed=CVTRES");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let icon = fs::read("assets/notvas.ico")
        .expect("Missing assets/notvas.ico; run scripts/make-icon.ps1");
    assert_eq!(u16_at(&icon, 2), 1, "Expected ICO data");
    let count = u16_at(&icon, 4);
    let mut res = Vec::new();
    resource(&mut res, 0, 0, &[]);
    let mut group = icon[..6].to_vec();
    for index in 0..count {
        let at = 6 + usize::from(index) * 16;
        let length = u32_at(&icon, at + 8) as usize;
        let offset = u32_at(&icon, at + 12) as usize;
        resource(&mut res, 3, index + 1, &icon[offset..offset + length]);
        group.extend_from_slice(&icon[at..at + 12]);
        group.extend_from_slice(&(index + 1).to_le_bytes());
    }
    resource(&mut res, 14, 1, &group);
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let resource_path = out.join("notvas.res");
    fs::write(&resource_path, res).expect("Cannot write icon resources");
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins={}", resource_path.display());
    } else {
        let object = out.join("notvas-icon.o");
        let cvtres = env::var_os("CVTRES").map(PathBuf::from).or_else(|| {
            let root = PathBuf::from(env::var_os("SystemRoot")?);
            let compiler = root.join("Microsoft.NET/Framework64/v4.0.30319/cvtres.exe");
            compiler.exists().then_some(compiler)
        });
        if let Some(compiler) = cvtres {
            let machine = match env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
                "x86" => "X86",
                "x86_64" => "X64",
                "aarch64" => "ARM64",
                target => panic!("Unsupported Windows icon target: {target}"),
            };
            let status = Command::new(compiler)
                .arg("/NOLOGO")
                .arg(format!("/MACHINE:{machine}"))
                .arg(format!("/OUT:{}", object.display()))
                .arg(&resource_path)
                .status()
                .expect("Cannot run Windows resource converter");
            assert!(status.success(), "Windows icon resource conversion failed");
            println!("cargo:rustc-link-arg-bins={}", object.display());
            return;
        }
        let portable = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join(".tools/mingw64/bin/windres.exe");
        let compiler = env::var_os("WINDRES")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                if portable.exists() {
                    portable
                } else {
                    PathBuf::from("windres")
                }
            });
        let status = Command::new(compiler)
            .args(["-J", "res", "-O", "coff", "-i"])
            .arg(&resource_path)
            .arg("-o")
            .arg(&object)
            .status()
            .expect("Cannot run windres; install GNU binutils or set WINDRES");
        assert!(status.success(), "Icon resource compilation failed");
        println!("cargo:rustc-link-arg-bins={}", object.display());
    }
}
