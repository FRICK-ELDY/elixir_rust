use crate::world::GameWorldInner;

/// 1.7.5: レベルアップ時の武器選択肢を計算（未所持優先 → 低レベル順、Lv8 除外）
pub(crate) fn compute_weapon_choices(w: &GameWorldInner) -> Vec<String> {
    const ALL: &[(&str, u8)] = &[
        ("magic_wand", 0), ("axe", 1), ("cross", 2),
        ("whip", 3), ("fireball", 4), ("lightning", 5),
    ];

    let mut choices: Vec<(i32, String)> = ALL
        .iter()
        .filter_map(|(name, wid)| {
            let lv = w
                .weapon_slots
                .iter()
                .find(|s| s.kind_id == *wid)
                .map(|s| s.level)
                .unwrap_or(0);
            if lv >= 8 {
                return None;
            }
            let sort_key = if lv == 0 { -1i32 } else { lv as i32 };
            Some((sort_key, (*name).to_string()))
        })
        .collect();

    choices.sort_by_key(|(k, _)| *k);
    choices.into_iter().take(3).map(|(_, n)| n).collect()
}
