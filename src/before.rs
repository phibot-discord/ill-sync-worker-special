//! What the Phase 1 map shows for three entries until the save flag is set: string literals of
//! `Chapter9Phase1Control`'s static constructor, kept here as the APK holds them and checked against its
//! `global-metadata.dat`

use phi_unpackd::collection::{self, Text};
use std::collections::BTreeMap;

/// The save flag the pre-unlock texts wait for
pub const FLAG: &str = "C9AboutTheUniverseUnlocked";
/// What the Phase 1 map shows for these entries while `FLAG` is unset: string literals of
/// `Chapter9Phase1Control`'s static constructor in `collection::LANGS` order, byte for byte as the APK holds them
const BEFORE: [(&str, [&str; 5]); 3] = [
    (
        "poyidomegino0",
        [
            "“只要拿着就可以吗，还是说需要进行某些操作？”\r\n“理论上说这东西是自动生效的，掏出来捏手里就行。不过，我们不能保证它吸引来的完全都是我们的数据，毕竟<b><color=#00e4ff>人类</color></b>的大脑嘛——你懂得，缠点儿什么，避免不了。”",
            "「只要拿著就可以嗎，還是說需要進行某些操作？」\r\n「理論上說這東西是自動生效的，掏出來捏手裡就行。不過，我們不能保證它吸引來的完全都是我們的數據，畢竟<b><color=#00e4ff>人類</color></b>的大腦嘛——你懂得，纏點兒什麼，避免不了。」",
            "\"Do I just hold it, or do I need to do something?\"\r\n\"Theoretically it activates on its own; just take it out and hold it. Still, we can't guarantee that everything it attracts is our data. <b><color=#00e4ff>Human</color></b> brains, you know - getting out cleanly isn't that easy. But it won't be dangerous, at least.\"",
            "「手に持っているだけでいいのか？ それとも何か操作が必要か？」\r\n「理論上は自動で効力を発揮するはずだ、取り出して手に握っていればいい。ただ、引き寄せられるものが100%我々のデータだけとは保証できないがね。なにせ<b><color=#00e4ff>人間</color></b>の脳だ――わかるだろう？ 完全にクリーンな状態で持ち出すのは簡単じゃない。まあ、危険はないはずだが」",
            "'그냥 들고 있으면 되는 거야, 아니면 무슨 조작이 필요한 거야?'\r\n'이론상 이건 자동으로 작동해. 꺼내서 손에 쥐고 있기만 하면 돼. 다만 이게 끌어오는 게 전부 우리 데이터라고는 장담할 수 없어. 인간의 뇌라니까. 알잖아. 깨끗하게 빼내는 게 그리 쉽지는 않거든. 그래도 위험한 일은 아니야.'",
        ],
    ),
    (
        "poyidomejiu0",
        [
            "曾经的她，在像素塔中用自己关于“隼”的记忆，制造了一个新的“隼”。但如今，这个隼已经被污染得面目全非，不再是她记忆里的兄长，而是一个独立存在的“<b><color=#00e4ff>人类</color></b>”，以至于她不再认识他，甚至放弃了继续维持他的存在。\r\n“你在哪……”",
            "曾經的她，在像素塔中用自己關於“隼”的記憶，製造了一個新的“隼”。但如今，這個隼已經被污染得面目全非，不再是她記憶裡的兄長，而是一個獨立存在的“<b><color=#00e4ff>人類</color></b>”，以至於她不再認識他，甚至放棄了繼續維持他的存在。\r\n“你在哪……”",
            "Once, inside the Pixel Tower, she had used her memories of \"Peregrinus\" to create a new \"Peregrinus.\" But now, that Peregrinus had been corrupted beyond recognition - no longer the brother she remembered, but an independent \"<b><color=#00e4ff>human</color></b>\" - she could no longer recognize him, and had even given up maintaining his existence.\r\n\"Where is the real you...\"",
            "かつての彼女は、ピクセルタワーの中で自分の中にある「隼」の記憶を利用して、新たな「隼」を作り出した。しかし今や、その隼は原型を留めないほどに汚染され、もはや記憶の中の兄ではなく、独立した一人の「<b><color=#00e4ff>人間</color></b>」となってしまった。その結果、彼女は彼を認識できなくなり、彼という存在を維持することすら諦めてしまったのだ。\r\n「本当のあなたは……一体、どこにいるの……」",
            "한때 그녀는 픽셀 타워 안에서 자신이 가진 ‘팔콘’에 대한 기억으로 새로운 ‘팔콘’을 만들어 냈다. 그러나 지금의 팔콘은 이미 오염되어 본래 모습을 알아볼 수 없게 되어 있었다. 더 이상 그녀의 기억 속에 있는 오빠가 아니라, 독립적으로 존재하는 하나의 ‘인간’이 되어 버린 것이다. 그 때문에 그녀는 더 이상 그를 알아보지 못했고, 심지어 그의 존재를 계속 유지하는 것마저 포기했다.\r\n“진짜 너는, 대체 어디에……”",
        ],
    ),
    (
        "nizhidaoma1",
        [
            "七百七十年过去，塔组织已达到了模拟目标，他们决定保存数据，关闭像素塔，以防技术泄漏。于是，他们启动了像素塔的清扫程序——幽蓝边界。\r\n吞噬一切的境界线下，<b><color=#00e4ff>人类</color></b>建起的巴别塔再次倒塌。",
            "七百七十年過去，塔組織已達到了模擬目標，他們決定保存數據，關閉像素塔，以防技術洩漏。於是，他們啟動了像素塔的清掃程序——幽藍邊界。\r\n吞噬一切的境界線下，<b><color=#00e4ff>人類</color></b>建起的巴別塔再次倒塌。",
            "770 years later, the \"Tower\" organization had achieved the goal of its simulation. They decided to save the data and shut down the Pixel Tower to prevent technology leakage. And so they activated the Pixel Tower's cleanup protocol - the Mazarine End.\r\nBeneath the boundary that devours everything, the Tower of Babel that <b><color=#00e4ff>humanity</color></b> had raised collapsed again.",
            "770年が過ぎ、タワー組織はついにシミュレーション目標を達成した。技術の流出を防ぐため、彼らはデータを保存し、ピクセルタワーを停止させることを決定した。こうして、ピクセルタワーの消去プログラム「マザリノ境界」が起動された。\r\nすべてを呑み込む境界線のもと、<b><color=#00e4ff>人々</color></b>が築き上げたバベルの塔は、再び崩れ去った。",
            "770년이 흐른 뒤, 탑 조직은 시뮬레이션의 목표에 도달했다. 그들은 기술 유출을 막기 위해 데이터를 보존하고 픽셀 타워를 폐쇄하기로 결정했다. 그리고 픽셀 타워의 정리 프로그램——마자린 엔드——를 가동했다.\r\n모든 것을 집어삼키는 경계선 아래에서, <b><color=#00e4ff>인간</color></b>들이 세운 바벨탑은 다시 무너져 내렸다.",
        ],
    ),
];

/// The pre-unlock texts whose every language is a string of this APK (`present` looks for the bytes in its
/// `global-metadata.dat`), by collection id, line breaks normalised as the Collection's bodies are. A text the APK
/// no longer holds is left out with a warning
pub fn before_texts(present: impl Fn(&[u8]) -> bool) -> (BTreeMap<String, Text>, Vec<String>) {
    let mut texts = BTreeMap::new();
    let mut warnings = Vec::new();
    for (id, langs) in BEFORE {
        let missing: Vec<&str> = collection::LANGS
            .iter()
            .zip(langs)
            .filter(|(_, text)| !present(text.as_bytes()))
            .map(|(lang, _)| *lang)
            .collect();
        if missing.is_empty() {
            texts.insert(id.to_string(), langs.map(collection::clean));
        } else {
            warnings.push(format!(
                "the pre-unlock text of {id} is not in this APK ({})",
                missing.join(", ")
            ));
        }
    }
    (texts, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixture;

    #[test]
    fn keeps_a_pre_unlock_text_only_while_the_apk_holds_every_language() {
        assert!(BEFORE
            .iter()
            .all(|(_, langs)| langs.iter().all(|text| text.contains("\r\n"))));
        let (texts, warnings) = before_texts(|_| true);
        assert!(warnings.is_empty());
        let ids: Vec<&str> = texts.keys().map(String::as_str).collect();
        assert_eq!(ids, ["nizhidaoma1", "poyidomegino0", "poyidomejiu0"]);
        let en = &texts["nizhidaoma1"][2];
        assert!(en.starts_with("770 years later") && en.contains("End.\nBeneath"));
        assert!(en.contains("<b><color=#00e4ff>humanity</color></b>") && !en.contains('\r'));
        assert!(texts["poyidomegino0"][0].starts_with("“只要拿着就可以吗"));
        let gone = BEFORE[1].1[3].as_bytes();
        let (texts, warnings) = before_texts(|text| text != gone);
        assert_eq!(texts.len(), 2);
        assert!(!texts.contains_key("poyidomejiu0"));
        assert_eq!(
            warnings,
            ["the pre-unlock text of poyidomejiu0 is not in this APK (ja)"]
        );
    }

    #[test]
    fn the_401_apk_holds_every_pre_unlock_text() {
        let Some(metadata) = fixture("global-metadata.dat") else {
            return;
        };
        let (texts, warnings) =
            before_texts(|text| metadata.windows(text.len()).any(|w| w == text));
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(texts.len(), 3);
    }
}
