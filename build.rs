use std::path::PathBuf;
use std::process::Command;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        
        let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let icon_path = manifest_dir.join("assets").join("mesh.ico");
        
        if icon_path.exists() {
            res.set_icon(icon_path.to_str().unwrap());
        } else {
            eprintln!("Icon file not found at {}", icon_path.display());
            std::process::exit(1);
        }

        let target_env = std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default();
        if target_env == "gnu" {
            let candidates = ["x86_64-w64-mingw32-windres", "windres"];
            let mut windres_path: Option<String> = None;
            for c in &candidates {
                if Command::new("sh").arg("-c").arg(format!("command -v {}", c)).status().map(|s| s.success()).unwrap_or(false) {
                    windres_path = Some(c.to_string());
                    break;
                }
            }

            if let Some(w) = windres_path {
                res.set_windres_path(&w);
                let ar_candidates = ["x86_64-w64-mingw32-ar", "ar"];
                for a in &ar_candidates {
                    if Command::new("sh").arg("-c").arg(format!("command -v {}", a)).status().map(|s| s.success()).unwrap_or(false) {
                        res.set_ar_path(a);
                        break;
                    }
                }
            } else {
                eprintln!("warning: windres not found; skipping embedding Windows icon (non-fatal)");
                return;
            }
        }

        if let Err(e) = res.compile() {
            eprintln!("Failed to compile Windows resources: {}", e);
            eprintln!("warning: continuing without embedded Windows resources");
        }
    }
}
