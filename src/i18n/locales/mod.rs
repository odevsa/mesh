pub mod de;
pub mod en;
pub mod es;
pub mod fr;
pub mod it;
pub mod ja;
pub mod ko;
pub mod nl;
pub mod pl;
pub mod pt_br;
pub mod pt_pt;
pub mod ru;
pub mod tr;
pub mod zh;
pub mod zh_tw;

use super::keys::TextKey;
use super::ResolvedLanguage;

pub fn translate(lang: ResolvedLanguage, key: TextKey) -> &'static str {
    match lang {
        ResolvedLanguage::En => en::translate(key),
        ResolvedLanguage::PtBr => pt_br::translate(key),
        ResolvedLanguage::PtPt => pt_pt::translate(key),
        ResolvedLanguage::Es => es::translate(key),
        ResolvedLanguage::Fr => fr::translate(key),
        ResolvedLanguage::De => de::translate(key),
        ResolvedLanguage::It => it::translate(key),
        ResolvedLanguage::Ja => ja::translate(key),
        ResolvedLanguage::Zh => zh::translate(key),
        ResolvedLanguage::ZhTw => zh_tw::translate(key),
        ResolvedLanguage::Ru => ru::translate(key),
        ResolvedLanguage::Ko => ko::translate(key),
        ResolvedLanguage::Nl => nl::translate(key),
        ResolvedLanguage::Pl => pl::translate(key),
        ResolvedLanguage::Tr => tr::translate(key),
    }
}
