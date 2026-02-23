use crate::constants::{BULLET_DAMAGE, WEAPON_COOLDOWN};

pub const MAX_WEAPON_LEVEL: u32 = 8;
pub const MAX_WEAPON_SLOTS: usize = 6;

// ─── WeaponKind ───────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WeaponKind {
    MagicWand,
    Axe,
    Cross,
    /// 近距離扇状薙ぎ払い（弾丸を生成しない直接判定）
    Whip,
    /// 敵を貫通する炎弾
    Fireball,
    /// 最近接から連鎖する電撃（最大 chain_count 体）
    Lightning,
}

impl WeaponKind {
    pub fn cooldown(&self) -> f32 {
        match self {
            WeaponKind::MagicWand => WEAPON_COOLDOWN,
            WeaponKind::Axe       => 1.5,
            WeaponKind::Cross     => 2.0,
            WeaponKind::Whip      => 1.0,
            WeaponKind::Fireball  => 1.0,
            WeaponKind::Lightning => 1.0,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            WeaponKind::MagicWand => BULLET_DAMAGE,
            WeaponKind::Axe       => 25,
            WeaponKind::Cross     => 15,
            WeaponKind::Whip      => 30,
            WeaponKind::Fireball  => 20,
            WeaponKind::Lightning => 15,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            WeaponKind::MagicWand => "magic_wand",
            WeaponKind::Axe       => "axe",
            WeaponKind::Cross     => "cross",
            WeaponKind::Whip      => "whip",
            WeaponKind::Fireball  => "fireball",
            WeaponKind::Lightning => "lightning",
        }
    }

    /// Bullet count table indexed by level (1-based; index 0 unused).
    /// Each entry is the bullet count for that level.
    /// Weapons with a fixed count return None (caller uses 1).
    /// To add a new weapon with level-scaling, add its table here.
    pub fn bullet_count_table(&self) -> Option<&'static [usize]> {
        match self {
            // Lv1-2: 1, Lv3-4: 2, Lv5-6: 3, Lv7-8: 4
            WeaponKind::MagicWand => Some(&[0, 1, 1, 2, 2, 3, 3, 4, 4]),
            // Lv1-3: 4-way, Lv4-8: 8-way
            WeaponKind::Cross     => Some(&[0, 4, 4, 4, 8, 8, 8, 8, 8]),
            // Fixed single bullet
            _                     => None,
        }
    }

    /// Whip の扇状範囲（半径 px）: Lv1=120, 各レベル +20
    pub fn whip_range(&self, level: u32) -> f32 {
        120.0 + (level as f32 - 1.0) * 20.0
    }

    /// Lightning のチェーン数: Lv1=2, Lv2=2, Lv3=3, Lv4=3, ... Lv8=6
    pub fn lightning_chain_count(&self, level: u32) -> usize {
        2 + level as usize / 2
    }

    /// イベントバス用の u8 値（EnemyKilled の weapon_kind など）
    pub fn as_u8(&self) -> u8 {
        match self {
            WeaponKind::MagicWand => 0,
            WeaponKind::Axe => 1,
            WeaponKind::Cross => 2,
            WeaponKind::Whip => 3,
            WeaponKind::Fireball => 4,
            WeaponKind::Lightning => 5,
        }
    }
}

// ─── WeaponSlot ───────────────────────────────────────────────
pub struct WeaponSlot {
    pub kind:           WeaponKind,
    pub level:          u32,   // 1〜8
    pub cooldown_timer: f32,
}

impl WeaponSlot {
    pub fn new(kind: WeaponKind) -> Self {
        Self { kind, level: 1, cooldown_timer: 0.0 }
    }

    /// Level-scaled cooldown (Lv8 = 50% of base)
    pub fn effective_cooldown(&self) -> f32 {
        let base = self.kind.cooldown();
        (base * (1.0 - (self.level as f32 - 1.0) * 0.07)).max(base * 0.5)
    }

    /// Level-scaled damage
    pub fn effective_damage(&self) -> i32 {
        let base = self.kind.damage();
        base + (self.level as i32 - 1) * (base / 4).max(1)
    }

    /// Level-scaled bullet count, driven by each weapon's bullet_count_table.
    pub fn bullet_count(&self) -> usize {
        let lv = self.level.clamp(1, MAX_WEAPON_LEVEL) as usize;
        self.kind
            .bullet_count_table()
            .and_then(|table| table.get(lv).copied())
            .unwrap_or(1)
    }
}
