"""Builds data/texts.json, the game texts phi-specials publishes, from the 2026-10-17 APK text research
(missing-texts.json): every line no page of the site showed, minus what has since got a page of its own (loading
tips, collection properties, illustrators, avatar names). Usage: python3 -I tools/build_texts.py <missing-texts.json>"""

import json
import re
import sys

LANGS = ["zh-CN", "zh-TW", "en", "ja", "ko"]
SKIP_STATUS = {"rendered", "trivial", "not-visible"}
# the PV clip each transcript belongs to: the PV page links to it
PV = {"qd": "qd", "afterdeduction": "after-deduction-video"}
# QDEventControl.<Start> shows these in this order, one string per language (pv-overlay's recipes.py reads the same code);
# the research only had the lines it could not find elsewhere, with a guessed language each
LINES = {"qd": [
    {"zh-CN": "...九....鸟..", "zh-TW": "...九....鳥..", "en": "..GGGG.... Eee...o....", "ja": "...九....鳥..", "ko": "...지....지오.."},
    {"zh-CN": "...鸠...!", "zh-TW": "...鳩...!", "en": "...Geo...!", "ja": "...鳩...!", "ko": "...지오...!"},
    {"zh-CN": "......喂？", "zh-TW": "......喂？", "en": "......Hello?", "ja": "......もしもし？", "ko": "......여보세요?"},
    {"zh-CN": "...听得到吗？", "zh-TW": "...聽得到嗎？", "en": "...Can you hear me?", "ja": "...聞こえてるか？", "ko": "...내 말 들려?"},
    {"zh-CN": "鸠？！", "zh-TW": "鳩？！", "en": "Geo?!", "ja": "鳩？！", "ko": "지오?!"},
], "afterdeduction": [
    {"zh-CN": "鸠：骗子……", "zh-TW": "鳩：騙子……", "en": "Geopelia: Liar…", "ja": "鳩：嘘つき……。", "ko": "지오펠리아: 거짓말쟁이……"},
    {"zh-CN": "鸠：……", "zh-TW": "鳩：……", "en": "Geopelia: …", "ja": "鳩：……", "ko": "지오펠리아: ……"},
    {"zh-CN": "????：请问……", "zh-TW": "????：請問……", "en": "????: Excuse me…", "ja": "????：あの……", "ko": "????: 저기……"},
    {"zh-CN": "Gino：这是你的蝴蝶吗？", "zh-TW": "Gino：這是你的蝴蝶嗎？", "en": "Gino: Is this yours?", "ja": "Gino：これ、君の蝶？", "ko": "지노: 이거 네 나비야?"},
]}
# groups the research has no lines for (the site showed them elsewhere), placed before another story group
ADDED = {"afterdeduction": "staffroll"}
# Covered by their own pages since the research: tips, properties, illustrators, avatar names (and the result line
# built from them)
SKIP_GROUPS = {
    "level0.TipsProvider.tips.zh-TW",
    "level0.TipsProvider.tips.en",
    "level0.TipsProvider.tips.ja",
    "level0.TipsProvider.tips.ko",
    "tpl.collection.properties-split",
    "tpl.level.intro.illustrator",
    "level0.GetCollectionControl.avatars",
    "tpl.result.new-avatar",
}
SONG_GROUPS = ("level0.GameInformation.song.",)

SCENES = {
    "BootScene": ("启动", "Boot"),
    "SongSelector": ("选曲", "Song select"),
    "Setting": ("设置", "Settings"),
    "LevelOver": ("结算", "Results"),
    "ChapterSelector": ("章节选择", "Chapter select"),
    "SaturnOS": ("SaturnOS", "SaturnOS"),
    "AboutUs": ("关于我们", "About us"),
    "Chapter8SelectMusic": ("第八章选曲", "Chapter 8 song select"),
    "Chapter9SelectMusic": ("第九章地图", "Chapter 9 map"),
    "Chapter9Phase2": ("第九章第二阶段", "Chapter 9 Phase 2"),
    "Chapter9FakeChallengeSongSelector": ("第九章课题选曲", "Chapter 9 challenge select"),
    "Chapter9Puzzle": ("第九章拼图", "Chapter 9 puzzle"),
    "Level": ("游玩", "Gameplay"),
    "SplashScene": ("开场", "Splash"),
}

STORY = {
    "il2cpp.c9.DSUnlockControl..cctor.0": (
        ("第九章 · Baseline Challenge 独白", "Chapter 9 · Baseline Challenge monologue"),
        ("第二阶段的 Baseline Challenge 中，ハテ 结束后、Desultory Signals 开始前，逐句点按显示。",
         "Shown one line per tap in Phase 2's Baseline Challenge, after ハテ ends and before Desultory Signals."),
    ),
    "il2cpp.c9.Phigros2.Chapter9.Chapter9Phase1Control..cctor.2": (
        ("第九章 · 解锁提示：Implexrough 与 Evanescent", "Chapter 9 · Unlock popup: Implexrough and Evanescent"),
        ("两首曲目都解锁后，回到第一阶段地图时显示。", "Shown back on the Phase 1 map once both songs are unlocked."),
    ),
    "il2cpp.c9.Phigros2.Chapter9.Chapter9Phase1Control..cctor.3": (
        ("第九章 · 解锁提示：Exoplanetary Mirage", "Chapter 9 · Unlock popup: Exoplanetary Mirage"),
        ("游玩 Exoplanetary Mirage 后回到地图时显示。", "Shown back on the map after Exoplanetary Mirage."),
    ),
    "qd": (
        ("QD 活动 · 过场字幕", "QD event · Cutscene captions"),
        ("QD 活动过场中的字幕。", "The captions of the QD event cutscene."),
    ),
    "level21.StaffList": (
        ("关于我们 · 制作人员名单", "About us · Staff roll"),
        ("设置 > 关于我们 中滚动显示。", "Scrolls in Settings > About us."),
    ),
    "afterdeduction": (
        ("第九章 · After Deduction 字幕", "Chapter 9 · After Deduction captions"),
        ("第九章推演结束后的视频中、制作人员名单之前显示。", "Shown in the video after Chapter 9's Deduction, before the staff roll."),
    ),
    "staffroll": (
        ("第九章 · 结局制作人员名单", "Chapter 9 · Ending staff roll"),
        ("第九章推演结束后滚动显示。", "Scrolls after Chapter 9's Deduction."),
    ),
}
# The Collection chapter each story group is shown with on the site
CHAPTERS = {
    "il2cpp.c9.DSUnlockControl..cctor.0": "c9",
    "il2cpp.c9.Phigros2.Chapter9.Chapter9Phase1Control..cctor.2": "c9",
    "il2cpp.c9.Phigros2.Chapter9.Chapter9Phase1Control..cctor.3": "c9",
    "afterdeduction": "c9",
    "staffroll": "c9",
    "qd": "c8",
}
# Friendly names for the groups and classes most players would look for
TITLES = {
    "songnames": ("游戏内的曲名写法", "Song names as the game spells them"),
    "level0.GameInformation.chapters": ("章节横幅", "Chapter banners"),
    "il2cpp.SongSelector..cctor.text": ("随机版本的谱师", "Charters of the random versions"),
    "tpl.level.intro.charter": ("开场卡片上的谱师", "Charters on the level intro card"),
    "il2cpp-retriage.GameInformation.GetSelfIntro@1cbc470.shown": ("默认自我介绍", "Default self-introduction"),
    "il2cpp.GameInformation..ctor.text": ("开发备注", "Developer note"),
    "ch8hints": ("第八章 · 提示属性", "Chapter 8 · Hint properties"),
    "globalgamemanagers.PlayerSettings": ("应用名称", "App name"),
    "level0.GetCollectionControl.collectionItems": ("收藏品解锁参数", "Collection unlock parameters"),
    "level16.RrharilEventLevelStart150Control": ("Rrhar'il 活动 · 难度标签（150）", "Rrhar'il event · Level labels (150)"),
    "level17.RrharilEventLevelStart100Control": ("Rrhar'il 活动 · 难度标签（100）", "Rrhar'il event · Level labels (100)"),
    "tpl.result.new-illustration": ("结算 · 获得新曲绘", "Results · New illustration"),
    "baked.c9.map-tile.news-article": ("第九章地图图块：现实（新闻报道）", "Chapter 9 map tile: Reality (news article)"),
    "baked.c9.map-tile.exoplanetary-mirage": ("第九章地图图块：系外幻象", "Chapter 9 map tile: Exoplanetary Mirage"),
    "baked.c9.map-tile.project-phigros": ("第九章地图图块：Project PhigrOS", "Chapter 9 map tile: Project PhigrOS"),
    "baked.c9.input-password-title": ("第九章 · 密码输入标题", "Chapter 9 · Passcode title"),
    "baked.c9-puzzle.hate-particle": ("第九章拼图 · ハテ 粒子", "Chapter 9 puzzle · ハテ particle"),
    "baked.promo.qq-guild": ("章节选择 · QQ 频道宣传图", "Chapter select · QQ guild promo"),
    "baked.promo.rizline-cn": ("章节选择 · Rizline 国服宣传图", "Chapter select · Rizline CN promo"),
    "baked.promo.rizline-global": ("章节选择 · Rizline 国际服宣传图", "Chapter select · Rizline global promo"),
    "baked.banner.qq-guild": ("章节选择 · QQ 频道横幅", "Chapter select · QQ guild banner"),
    "baked.banner.rizline-cn": ("章节选择 · Rizline 国服横幅", "Chapter select · Rizline CN banner"),
    "baked.banner.rizline-global": ("章节选择 · Rizline 国际服横幅", "Chapter select · Rizline global banners"),
    "baked.life-badges": ("选曲 · 生命值徽章", "Song select · Life badges"),
    "baked.splash4.station-signage": ("开场 · 车站标识", "Splash · Station signs"),
    "baked.ds-intro.difficulty-freeze": ("Desultory Signals 开场 · 难度画面", "Desultory Signals intro · Difficulty frames"),
    "baked.boot-warning": ("启动 · 光敏性警告", "Boot · Photosensitivity warning"),
    "baked.rrharil150.false-rank": ("Rrhar'il 活动 · False 评级", "Rrhar'il event · False rank"),
    "baked.song-select-icon-os": ("选曲 · OS 图标", "Song select · OS icon"),
    "baked.soundwavefrenzy.participated": ("Sound Wave Frenzy · 参演艺人收录曲目", "Sound Wave Frenzy · Performing artists' songs"),
    "baked.soundwavefrenzy.partners": ("Sound Wave Frenzy · 合作方艺人收录曲目", "Sound Wave Frenzy · Partner artists' songs"),
    "baked.rrharil100.buttons": ("Rrhar'il 活动 · 按钮", "Rrhar'il event · Buttons"),
    "baked.brand-logos": ("标志", "Logos"),
    "baked.saturnos-launch-logo": ("SaturnOS · 启动标志", "SaturnOS · Launch logo"),
    "baked.saturnos.limbo-wiki": ("SaturnOS · Limbo 百科标题", "SaturnOS · Limbo wiki header"),
    "baked.songname.hyakki-yakou": ("选曲 · 百鬼夜行 曲名图", "Song select · 百鬼夜行 title art"),
}
CLASSES = {
    "Chapter9SongDetailsLockedControl": ("未解锁曲目的提示", "Hints for locked songs"),
    "Chapter9PuzzlePopupControl": ("拼图弹窗", "Puzzle popup"),
    "Chapter9InputPasswordControl": ("密码输入", "Passcode input"),
    "VisionReplayButton": ("再演按钮", "Revisit button"),
    "PopupControl": ("SaturnOS 弹窗", "SaturnOS popup"),
    "LifeUnlockControl": ("生命值限制解锁", "Life limit unlock"),
    "LifeBadgeControl": ("生命值徽章", "Life badges"),
    "Money": ("价格", "Prices"),
    "ScoreControl": ("分数", "Score"),
    "Setting": ("设置", "Settings"),
    "LevelOverControl": ("结算", "Results"),
    "SaturnOSControl": ("SaturnOS", "SaturnOS"),
    "SaturnOSLaunchControl": ("SaturnOS 启动", "SaturnOS launch"),
    "StatsFrameController": ("统计与云存档", "Stats and cloud save"),
    "CloudSaveManager": ("云存档", "Cloud save"),
    "SelectSavePopup": ("选择存档", "Choose a save"),
    "AntiAddiction": ("防沉迷", "Anti-addiction"),
    "TapSDKWrapper": ("TapTap 登录", "TapTap login"),
    "Chapter8HintControl": ("第八章提示", "Chapter 8 hints"),
    "ChapterSelector": ("章节选择", "Chapter select"),
    "FolderSummaryControl": ("SaturnOS 文件夹", "SaturnOS folder"),
    "GameInformation": ("游戏信息", "Game information"),
    "SplashScene": ("开场", "Splash"),
    "ShowTips": ("加载提示", "Loading tips"),
    "MultiLanguageTextBase": ("通用界面文字", "Shared interface text"),
    "Chapter9SongLevelStartControl": ("第九章开场", "Chapter 9 level start"),
    "Chapter9Phase1Control": ("第九章第一阶段", "Chapter 9 Phase 1"),
    "Chapter9Phase2Control": ("第九章第二阶段", "Chapter 9 Phase 2"),
}

MERGE = {
    "il2cpp.QDEventControl/<Start>d__5.MoveNext.text": "qd",
    "il2cpp-retriage.QDEventControl/<Start>d__5.MoveNext@1cd5fe0.shown": "qd",
}

SECTIONS = [
    ("story", ("剧情", "Story"), ("只存在于游戏代码或场景里、其他页面没有收录的剧情文本。",
                                 "Story text that only the game's code or scenes hold, and no other page shows.")),
    ("chapter9", ("第九章界面", "Chapter 9 screens"), ("第九章各画面上的界面文字：密码输入、提示、拼图警告、再演等。",
                                                   "Interface text on Chapter 9's screens: the passcode, hints, the puzzle warning, Revisit and more.")),
    ("ui", ("游戏界面", "Game interface"), ("设置、选曲、结算、SaturnOS 等画面上的文字。",
                                         "Text on the settings, song select, results, SaturnOS and other screens.")),
    ("data", ("资料", "Data"), ("章节横幅、随机版本的谱师、游戏内的曲名写法等数据。",
                                 "Chapter banners, the charters of random versions, how the game spells song names, and other data.")),
    ("images", ("图片中的文字", "Text in images"), ("画在图片里的文字，附游戏内的原图。",
                                                   "Text drawn into the game's images, shown with the image itself.")),
    ("sdk", ("第三方 SDK", "Third-party SDKs"), ("游戏内置的 TapTap、防沉迷、Google Play 等第三方组件的文字。",
                                                "Text from the third-party parts the game ships: TapTap, anti-addiction, Google Play and others.")),
]


def scene_name(where):
    m = re.search(r"Assets/[^ ]*/([^/ ]+)\.unity", where) or re.search(r"\(scene \d+ ([A-Za-z0-9]+)\)", where)
    return m.group(1) if m else None


def area(group):
    gid, where = group["id"], group["where"]
    m = re.search(r"Assembly-CSharp ([A-Za-z0-9_.<>/`]+?)(?:::|\s|$)", where)
    if gid.startswith(("il2cpp", "tpl.")) and m:
        cls = m.group(1).split("/")[0].split(".")[-1]
        return CLASSES.get(cls, (cls, cls))
    m = re.search(r"libil2cpp\.so ([A-Za-z0-9_]+)", where)
    if gid.startswith("tpl.") and m:
        return (m.group(1), m.group(1))
    scene = scene_name(where)
    if scene:
        zh, en = SCENES.get(scene, (scene, scene))
        path = re.search(r"(?:/ |m_Text at \d+=|pathID \d+ )([^\[;(]+)", where)
        tail = "/".join(path.group(1).strip().split("/")[-2:]) if path else ""
        return (f"{zh} · {tail}" if tail else zh, f"{en} · {tail}" if tail else en)
    m = re.search(r"resources\.assets / (?:TextAsset \")?([A-Za-z0-9_]+)", where)
    if m:
        return (m.group(1), m.group(1))
    return (gid.split(".", 1)[-1], gid.split(".", 1)[-1])


def check_of(gid):
    if gid.startswith(("il2cpp", "c9.puzzle")) and not gid.startswith("il2cpp-retriage"):
        return "metadata" if gid.startswith("il2cpp") else "scene"
    if gid.startswith("il2cpp-retriage"):
        return "metadata"
    if gid.startswith(("level", "l2", "l3", "sharedassets", "globalgamemanagers", "resources.assets", "c9.", "ch8.")):
        return "scene"
    if gid.startswith("android."):
        return "apk"
    return "none"


def section_of(group):
    gid, rs = group["id"], group["reportSection"]
    if rs == "b-story":
        return "story"
    if rs == "f-baked-image":
        return "images"
    if rs == "e-sdk":
        return "sdk"
    if rs == "c-data":
        return "data"
    scene = scene_name(group["where"]) or ""
    if ".c9." in gid or gid.startswith(("c9.", "l29", "l3")) or "Chapter9" in scene or "Chapter9" in group["where"][:160]:
        return "chapter9"
    return "ui"


def clean_line(line):
    out = {k: line[k] for k in LANGS if isinstance(line.get(k), str)}
    if out:
        return {k: v for k, v in out.items() if v.strip()}
    text = line.get("text")
    if not isinstance(text, str) or not text.strip():
        return None
    lang = line.get("lang")
    lang = {"zh": None, "text": None}.get(lang, lang)
    return {"text": text, **({"lang": lang} if lang in LANGS else {})}


def images_of(where):
    found = []
    for file, names in re.findall(r"(sharedassets\d+\.assets)/(\{[^}]*\}|[^(,;{<]+?)(?=\s*\(|\s*<-|[,;]|$)", where):
        for name in names.strip("{}").split(","):
            name = name.strip()
            if name and {"file": file, "name": name} not in found:
                found.append({"file": file, "name": name})
    return found


def main(path):
    research = json.load(open(path, encoding="utf-8"))
    sections = {sid: [] for sid, _, _ in SECTIONS}
    merged = {}
    for group in research["groups"]:
        gid = group["id"]
        if gid in SKIP_GROUPS:
            continue
        lines = []
        for line in group["lines"]:
            if line.get("status") in SKIP_STATUS:
                continue
            if gid.startswith(SONG_GROUPS) and line.get("status") != "display-form-differs":
                continue
            cleaned = clean_line(line)
            if cleaned and cleaned not in lines:
                lines.append(cleaned)
        if not lines:
            continue
        key = MERGE.get(gid) or (
            "staffroll" if gid.startswith("c9.afterdeduction.staffroll")
            else "songnames" if gid.startswith(SONG_GROUPS)
            else "ch8hints" if gid.startswith("ch8.hint.")
            else gid
        )
        if key in merged:
            merged[key]["lines"].extend(l for l in lines if l not in merged[key]["lines"])
            continue
        story = STORY.get(key)
        title = story[0] if story else TITLES.get(key) or area(group)
        note = story[1] if story else None
        entry = {
            "id": key.replace("/", ".").replace("<", "").replace(">", ""),
            "title": {"zh": title[0], "en": title[1]},
            "note": {"zh": note[0], "en": note[1]} if note else None,
            "check": check_of(gid),
            "lines": lines,
        }
        if key in CHAPTERS:
            entry["chapter"] = CHAPTERS[key]
        if group["reportSection"] == "f-baked-image":
            entry["check"] = "none"
            entry["images"] = images_of(group["where"])
        merged[key] = entry
        sections[section_of(group)].append(entry)
    for key, before in ADDED.items():
        (title, note), story = STORY[key], sections["story"]
        at = next((n for n, g in enumerate(story) if g["id"] == before), len(story))
        merged[key] = {"id": key, "title": {"zh": title[0], "en": title[1]},
                       "note": {"zh": note[0], "en": note[1]}, "check": "metadata", "lines": [],
                       **({"chapter": CHAPTERS[key]} if key in CHAPTERS else {})}
        story.insert(at, merged[key])
    for key, lines in LINES.items():
        merged[key]["lines"] = lines
        if key in PV:
            merged[key]["pv"] = PV[key]
    out = {"v": 1, "sections": []}
    for sid, title, lede in SECTIONS:
        out["sections"].append({
            "id": sid,
            "title": {"zh": title[0], "en": title[1]},
            "lede": {"zh": lede[0], "en": lede[1]},
            "groups": sections[sid],
        })
    json.dump(out, sys.stdout, ensure_ascii=False, indent=1)
    sys.stdout.write("\n")


main(sys.argv[1])
