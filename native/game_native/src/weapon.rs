use crate::constants::{BULLET_DAMAGE, WEAPON_COOLDOWN};

pub const MAX_WEAPON_LEVEL: u32 = 8;
pub const MAX_WEAPON_SLOTS: usize = 6;

// ─── WeaponKind ───────────────────────────────────────────────
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WeaponKind {
    MagicWand,
    Axe,
    Cross,
}

impl WeaponKind {
    pub fn cooldown(&self) -> f32 {
        match self {
            WeaponKind::MagicWand => WEAPON_COOLDOWN,
            WeaponKind::Axe       => 1.5,
            WeaponKind::Cross     => 2.0,
        }
    }

    pub fn damage(&self) -> i32 {
        match self {
            WeaponKind::MagicWand => BULLET_DAMAGE,
            WeaponKind::Axe       => 25,
            WeaponKind::Cross     => 15,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            WeaponKind::MagicWand => "magic_wand",
            WeaponKind::Axe       => "axe",
            WeaponKind::Cross     => "cross",
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
