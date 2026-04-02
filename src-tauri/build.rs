fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icons/icon.ico")
            .set("FileDescription", "Loglan Online Dictionary Manager")
            .set("ProductName", "LOD Manager")
            .set("CompanyName", "torrua")
            .set("LegalCopyright", "Copyright (C) 2026 torrua")
            .set("OriginalFilename", "LOD Manager.exe")
            .set("InternalName", "LOD Manager");
        res.compile().expect("Failed to compile Windows resources");
    }
    tauri_build::build()
}
