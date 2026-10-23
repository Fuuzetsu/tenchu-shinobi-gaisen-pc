use egui::Color32;
use tenchu_assets::script::Language;

pub fn mission(language: Language, mission: usize) -> String {
    let index = mission.saturating_sub(1).min(9);
    let (prefix, titles) = match language {
        Language::English => (
            "Level",
            [
                "Punish The Evil Merchant",
                "Deliver The Secret Message",
                "Rescue The Captive Ninja",
                "Cross The Checkpoint",
                "Execute The Corrupt Minister",
                "Infiltrate The Manji Cult",
                "Destroy The Foreign Pirate",
                "Cure The Princess",
                "Reclaim The Castle",
                "Free The Princess",
            ],
        ),
        Language::French => (
            "Niveau",
            [
                "Punir l'infâme marchand",
                "Délivrer le message secret",
                "Sauver le prisonnier Ninja",
                "Passer le poste de contrôle",
                "Exécuter le ministre corrompu",
                "Infiltrer la secte Manji",
                "Éliminer les pirates ennemis",
                "Guérir la princesse",
                "Reprendre possession du château",
                "Libérer la princesse",
            ],
        ),
        Language::Italian => (
            "Livello",
            [
                "Punire il malvagio mercante",
                "Consegnare il messaggio segreto",
                "Liberare il ninja prigioniero",
                "Passare il posto di blocco",
                "Eliminare il ministro corrotto",
                "Infiltrarsi nella setta Manji",
                "Distruggere i pirati spagnoli",
                "Curare la principessa",
                "Recuperare il castello",
                "Liberare la principessa",
            ],
        ),
        Language::Japanese => {
            return format!(
                "任務 {}乃巻　{}",
                ["一", "二", "三", "四", "五", "六", "七", "八", "九", "十"][index],
                [
                    "悪徳商人を成敗せよ",
                    "密書を運べ",
                    "捕虜を助け出せ",
                    "関所を越えろ",
                    "悪の奉行を討伐せよ",
                    "邪教卍教の御神体",
                    "南蛮海賊の来襲",
                    "姫の大病",
                    "鬼の城",
                    "闇の城",
                ][index]
            );
        }
    };
    format!("{prefix} {} {}", index + 1, titles[index])
}

pub fn rank(language: Language, grade: usize) -> &'static str {
    (match language {
        Language::English => ["Thug", "Novice", "Ninja", "Master Ninja", "Grand Master"],
        Language::Japanese => ["門前払い", "下忍", "中忍", "上忍", "忍術皆伝"],
        Language::French => ["Voyou", "Novice", "Ninja", "Maître Ninja", "Grand Maître"],
        Language::Italian => [
            "Malvivente",
            "Novizio",
            "Ninja",
            "Maestro Ninja",
            "Gran Maestro",
        ],
    })[grade.min(4)]
}

pub fn rank_color(grade: usize) -> Color32 {
    let [r, g, b] = [
        [68, 172, 188],
        [76, 177, 116],
        [133, 139, 227],
        [210, 133, 202],
        [234, 196, 80],
    ][grade.min(4)];
    Color32::from_rgb(r, g, b)
}

#[test]
fn embedded_font_covers_all_mission_and_rank_labels() {
    let context = egui::Context::default();
    crate::fonts::install(&context);
    let _ = context.run(Default::default(), |context| {
        context.fonts(|fonts| {
            let font = crate::fonts::id(12.0);
            for language in [
                Language::English,
                Language::Japanese,
                Language::French,
                Language::Italian,
            ] {
                for number in 1..=10 {
                    let text = mission(language, number);
                    assert!(fonts.has_glyphs(&font, &text), "Missing glyph in {text}");
                }
                for grade in 0..5 {
                    let text = rank(language, grade);
                    assert!(fonts.has_glyphs(&font, text), "Missing glyph in {text}");
                }
            }
        });
    });
}
