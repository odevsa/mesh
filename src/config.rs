use serde::{Deserialize, Serialize};
use directories::ProjectDirs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub background: [u8; 3],
    pub object_color: [u8; 3],
    pub light_color: [u8; 3],
    pub shadow_color: [u8; 3],
    pub object_scale: f32,
    pub camera_eye: [f32; 3],
    pub scroll_speed: f32,
    pub invert_scroll: bool,
    pub scroll_min: f32,
    pub scroll_max: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            background: [0x0A, 0x0A, 0x0A],
            object_color: [0xCC, 0xCC, 0xCC],
            light_color: [0xCC, 0xCC, 0xCC],
            shadow_color: [0x00, 0x00, 0x00],
            object_scale: 1.0,
            camera_eye: [0.0, -1.5, 1.5],
            scroll_speed: 0.005,
            invert_scroll: false,
            scroll_min: 0.5,
            scroll_max: 5.0,
        }
    }
}

impl Config {
    pub fn path() -> Option<PathBuf> {
        if let Some(proj) = ProjectDirs::from("com", "mesh", "mesh") {
            let cfg_dir = proj.config_dir();
            Some(cfg_dir.join("config.json"))
        } else {
            None
        }
    }

    pub fn load_or_create() -> Result<Config, String> {
        let p = Self::path().ok_or_else(|| "couldn't determine config path".to_string())?;
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create dir: {}", e))?;
        }
        if p.exists() {
            let s = std::fs::read_to_string(&p).map_err(|e| format!("read cfg: {}", e))?;
            let default = serde_json::to_value(Config::default()).map_err(|e| format!("serialize default: {}", e))?;
            let mut loaded: serde_json::Value = serde_json::from_str(&s).map_err(|e| format!("parse cfg: {}", e))?;
            fn merge(a: &serde_json::Value, b: &mut serde_json::Value) {
                match (a, b) {
                    (serde_json::Value::Object(ma), serde_json::Value::Object(mb)) => {
                        for (k, v) in ma {
                            if !mb.contains_key(k) {
                                mb.insert(k.clone(), v.clone());
                            } else {
                                merge(v, mb.get_mut(k).unwrap());
                            }
                        }
                    }
                    _ => {}
                }
            }
            merge(&default, &mut loaded);
            let s2 = serde_json::to_string_pretty(&loaded).map_err(|e| format!("serialize cfg: {}", e))?;
            std::fs::write(&p, s2).map_err(|e| format!("write cfg: {}", e))?;
            let c: Config = serde_json::from_value(loaded).map_err(|e| format!("final parse cfg: {}", e))?;
            Ok(c)
        } else {
            let c = Config::default();
            let s = serde_json::to_string_pretty(&c).map_err(|e| format!("serialize cfg: {}", e))?;
            std::fs::write(&p, s).map_err(|e| format!("write cfg: {}", e))?;
            Ok(c)
        }
    }
}
