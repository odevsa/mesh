pub mod keys;
pub mod locales;

pub use keys::TextKey;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    #[default]
    Auto,
    En,
    #[serde(alias = "pt-BR", alias = "pt_br", alias = "pt")]
    PtBr,
    #[serde(alias = "pt-PT", alias = "pt_pt")]
    PtPt,
    Es,
    Fr,
    De,
    It,
    Ja,
    Zh,
    #[serde(alias = "zh-TW", alias = "zh_tw", alias = "zh-HK", alias = "zh_hk")]
    ZhTw,
    Ru,
    Ko,
    Nl,
    Pl,
    Tr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedLanguage {
    En,
    PtBr,
    PtPt,
    Es,
    Fr,
    De,
    It,
    Ja,
    Zh,
    ZhTw,
    Ru,
    Ko,
    Nl,
    Pl,
    Tr,
}

impl Language {
    pub const ALL: &'static [Language] = &[
        Language::Auto,
        Language::En,
        Language::PtBr,
        Language::PtPt,
        Language::Es,
        Language::Fr,
        Language::De,
        Language::It,
        Language::Ja,
        Language::Zh,
        Language::ZhTw,
        Language::Ru,
        Language::Ko,
        Language::Nl,
        Language::Pl,
        Language::Tr,
    ];

    pub fn resolve(self) -> ResolvedLanguage {
        match self {
            Language::Auto => Self::detect_system_language(),
            Language::En => ResolvedLanguage::En,
            Language::PtBr => ResolvedLanguage::PtBr,
            Language::PtPt => ResolvedLanguage::PtPt,
            Language::Es => ResolvedLanguage::Es,
            Language::Fr => ResolvedLanguage::Fr,
            Language::De => ResolvedLanguage::De,
            Language::It => ResolvedLanguage::It,
            Language::Ja => ResolvedLanguage::Ja,
            Language::Zh => ResolvedLanguage::Zh,
            Language::ZhTw => ResolvedLanguage::ZhTw,
            Language::Ru => ResolvedLanguage::Ru,
            Language::Ko => ResolvedLanguage::Ko,
            Language::Nl => ResolvedLanguage::Nl,
            Language::Pl => ResolvedLanguage::Pl,
            Language::Tr => ResolvedLanguage::Tr,
        }
    }

    pub fn detect_system_language() -> ResolvedLanguage {
        if let Some(locale) = sys_locale::get_locale() {
            let lower = locale.to_lowercase().replace('_', "-");
            if lower.starts_with("pt-br") || lower == "pt" {
                return ResolvedLanguage::PtBr;
            } else if lower.starts_with("pt-pt") {
                return ResolvedLanguage::PtPt;
            } else if lower.starts_with("es") {
                return ResolvedLanguage::Es;
            } else if lower.starts_with("fr") {
                return ResolvedLanguage::Fr;
            } else if lower.starts_with("de") {
                return ResolvedLanguage::De;
            } else if lower.starts_with("it") {
                return ResolvedLanguage::It;
            } else if lower.starts_with("ja") {
                return ResolvedLanguage::Ja;
            } else if lower.starts_with("zh-tw") || lower.starts_with("zh-hk") {
                return ResolvedLanguage::ZhTw;
            } else if lower.starts_with("zh") {
                return ResolvedLanguage::Zh;
            } else if lower.starts_with("ru") {
                return ResolvedLanguage::Ru;
            } else if lower.starts_with("ko") {
                return ResolvedLanguage::Ko;
            } else if lower.starts_with("nl") {
                return ResolvedLanguage::Nl;
            } else if lower.starts_with("pl") {
                return ResolvedLanguage::Pl;
            } else if lower.starts_with("tr") {
                return ResolvedLanguage::Tr;
            } else if lower.starts_with("en") {
                return ResolvedLanguage::En;
            }
        }

        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = std::env::var(var) {
                let lower = val.to_lowercase().replace('_', "-");
                if lower.starts_with("pt-br") || lower == "pt" {
                    return ResolvedLanguage::PtBr;
                } else if lower.starts_with("pt-pt") {
                    return ResolvedLanguage::PtPt;
                } else if lower.starts_with("es") {
                    return ResolvedLanguage::Es;
                } else if lower.starts_with("fr") {
                    return ResolvedLanguage::Fr;
                } else if lower.starts_with("de") {
                    return ResolvedLanguage::De;
                } else if lower.starts_with("it") {
                    return ResolvedLanguage::It;
                } else if lower.starts_with("ja") {
                    return ResolvedLanguage::Ja;
                } else if lower.starts_with("zh-tw") || lower.starts_with("zh-hk") {
                    return ResolvedLanguage::ZhTw;
                } else if lower.starts_with("zh") {
                    return ResolvedLanguage::Zh;
                } else if lower.starts_with("ru") {
                    return ResolvedLanguage::Ru;
                } else if lower.starts_with("ko") {
                    return ResolvedLanguage::Ko;
                } else if lower.starts_with("nl") {
                    return ResolvedLanguage::Nl;
                } else if lower.starts_with("pl") {
                    return ResolvedLanguage::Pl;
                } else if lower.starts_with("tr") {
                    return ResolvedLanguage::Tr;
                } else if lower.starts_with("en") {
                    return ResolvedLanguage::En;
                }
            }
        }

        ResolvedLanguage::En
    }

    pub fn t(self, key: TextKey) -> &'static str {
        locales::translate(self.resolve(), key)
    }

    pub fn display_name(self, current_lang: Language) -> &'static str {
        match self {
            Language::Auto => current_lang.t(TextKey::SystemDefault),
            Language::En => "English",
            Language::PtBr => "Português (Brasil)",
            Language::PtPt => "Português (Portugal)",
            Language::Es => "Español",
            Language::Fr => "Français",
            Language::De => "Deutsch",
            Language::It => "Italiano",
            Language::Ja => "日本語",
            Language::Zh => "中文 (简体)",
            Language::ZhTw => "中文 (繁體)",
            Language::Ru => "Русский",
            Language::Ko => "한국어",
            Language::Nl => "Nederlands",
            Language::Pl => "Polski",
            Language::Tr => "Türkçe",
        }
    }
}

pub fn setup_cjk_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let candidate_paths = [
        // Linux candidates
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Bold.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
        "/usr/share/fonts/noto/NotoSans-Regular.ttf",
        // Windows candidates
        "C:\\Windows\\Fonts\\msyh.ttc",
        "C:\\Windows\\Fonts\\msyh.ttf",
        "C:\\Windows\\Fonts\\simsun.ttc",
        "C:\\Windows\\Fonts\\meiryo.ttc",
        "C:\\Windows\\Fonts\\malgun.ttf",
        // macOS candidates
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/Library/Fonts/Arial Unicode.ttf",
    ];

    for path in candidate_paths {
        if let Ok(font_bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "cjk_fallback".to_string(),
                egui::FontData::from_owned(font_bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("cjk_fallback".to_string());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("cjk_fallback".to_string());
            break;
        }
    }

    let emoji_candidates = [
        "/usr/share/fonts/noto/NotoColorEmoji.ttf",
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
        "C:\\Windows\\Fonts\\seguiemj.ttf",
        "/System/Library/Fonts/Apple Color Emoji.ttc",
    ];

    for path in emoji_candidates {
        if let Ok(font_bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "emoji_fallback".to_string(),
                egui::FontData::from_owned(font_bytes).into(),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("emoji_fallback".to_string());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("emoji_fallback".to_string());
            break;
        }
    }

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_fallback_resolution() {
        assert_eq!(Language::En.resolve(), ResolvedLanguage::En);
        assert_eq!(Language::PtBr.resolve(), ResolvedLanguage::PtBr);
        assert_eq!(Language::Es.resolve(), ResolvedLanguage::Es);
        assert_eq!(Language::Zh.resolve(), ResolvedLanguage::Zh);

        let resolved_auto = Language::Auto.resolve();
        assert!(matches!(
            resolved_auto,
            ResolvedLanguage::En
                | ResolvedLanguage::PtBr
                | ResolvedLanguage::PtPt
                | ResolvedLanguage::Es
                | ResolvedLanguage::Fr
                | ResolvedLanguage::De
                | ResolvedLanguage::It
                | ResolvedLanguage::Ja
                | ResolvedLanguage::Zh
                | ResolvedLanguage::ZhTw
                | ResolvedLanguage::Ru
                | ResolvedLanguage::Ko
                | ResolvedLanguage::Nl
                | ResolvedLanguage::Pl
                | ResolvedLanguage::Tr
        ));
    }

    #[test]
    fn test_translation_keys() {
        assert_eq!(Language::En.t(TextKey::About), "About");
        assert_eq!(Language::PtBr.t(TextKey::About), "Sobre");
        assert_eq!(Language::Es.t(TextKey::About), "Acerca de");
        assert_eq!(Language::Fr.t(TextKey::About), "À propos");
        assert_eq!(Language::De.t(TextKey::About), "Über");
        assert_eq!(Language::Zh.t(TextKey::About), "关于");
        assert_eq!(Language::ZhTw.t(TextKey::About), "關於");
        assert_eq!(Language::Ru.t(TextKey::About), "О программе");
        assert_eq!(Language::Ja.t(TextKey::About), "情報");
    }

    #[test]
    fn test_serde_language() {
        let json = serde_json::to_string(&Language::PtBr).unwrap();
        assert_eq!(json, "\"pt_br\"");
        let deserialized: Language = serde_json::from_str("\"pt_br\"").unwrap();
        assert_eq!(deserialized, Language::PtBr);
        let deserialized_alias: Language = serde_json::from_str("\"pt-BR\"").unwrap();
        assert_eq!(deserialized, Language::PtBr);
    }
}
