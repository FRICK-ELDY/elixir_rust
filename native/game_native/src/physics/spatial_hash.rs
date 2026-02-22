use std::collections::HashMap;

// ─── Spatial Hash ─────────────────────────────────────────────
pub struct SpatialHash {
    pub cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn insert(&mut self, id: usize, x: f32, y: f32) {
        let key = self.cell_key(x, y);
        self.cells.entry(key).or_default().push(id);
    }

    fn cell_key(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x / self.cell_size).floor() as i32,
            (y / self.cell_size).floor() as i32,
        )
    }

    /// 指定円の範囲内にあるエンティティ ID を返す（距離フィルタなし）
    pub fn query_nearby(&self, x: f32, y: f32, radius: f32) -> Vec<usize> {
        let r = (radius / self.cell_size).ceil() as i32;
        let cx = (x / self.cell_size).floor() as i32;
        let cy = (y / self.cell_size).floor() as i32;
        let mut result = Vec::new();
        for ix in (cx - r)..=(cx + r) {
            for iy in (cy - r)..=(cy + r) {
                if let Some(ids) = self.cells.get(&(ix, iy)) {
                    result.extend_from_slice(ids);
                }
            }
        }
        result
    }
}

// ─── Collision World ──────────────────────────────────────────
/// 動的オブジェクト（敵・プレイヤー）を管理する Spatial Hash
pub struct CollisionWorld {
    pub dynamic: SpatialHash,
}

impl CollisionWorld {
    /// cell_size = 当たり判定半径の 2〜3 倍（例: 32px → 80px）
    pub fn new(cell_size: f32) -> Self {
        Self {
            dynamic: SpatialHash::new(cell_size),
        }
    }

    /// 毎フレーム: 生存している敵を全て再登録
    pub fn rebuild(&mut self, positions_x: &[f32], positions_y: &[f32], alive: &[bool]) {
        self.dynamic.clear();
        for i in 0..alive.len() {
            if alive[i] {
                self.dynamic.insert(i, positions_x[i], positions_y[i]);
            }
        }
    }
}
