"""
アトラス画像生成スクリプト（Step 23: スプライトアニメーション対応版）
1280x64 px の RGBA PNG を生成する:

  アニメーションキャラクター（各 64x64、複数フレーム）:
  [   0.. 255] プレイヤー歩行 4 フレーム（各 64x64）
  [ 256.. 511] Slime バウンス 4 フレーム（各 64x64）
  [ 512.. 639] Bat 羽ばたき 2 フレーム（各 64x64）
  [ 640.. 767] Golem 歩行 2 フレーム（各 64x64）

  静止スプライト（各 64x64）:
  [ 768.. 831] 弾丸 MagicWand/Axe/Cross（黄色い円）
  [ 832.. 895] パーティクル（白い円）
  [ 896.. 959] 経験値宝石（緑のダイヤ）
  [ 960..1023] 回復ポーション（赤い瓶）
  [1024..1087] 磁石（黄色い磁石）
  [1088..1151] Fireball 弾丸（赤橙の炎球）
  [1152..1215] Lightning 弾丸（水色の電撃球）
  [1216..1279] Whip エフェクト（黄緑の弧状）
"""

import struct
import zlib
import math

W, H = 1280, 64

pixels = bytearray(W * H * 4)

def set_pixel(x, y, r, g, b, a=255):
    if 0 <= x < W and 0 <= y < H:
        idx = (y * W + x) * 4
        pixels[idx]     = r
        pixels[idx + 1] = g
        pixels[idx + 2] = b
        pixels[idx + 3] = a

def fill_rect(x0, y0, x1, y1, r, g, b, a=255):
    for y in range(y0, y1):
        for x in range(x0, x1):
            set_pixel(x, y, r, g, b, a)

def fill_circle(cx, cy, radius, r, g, b, a=255):
    for y in range(cy - radius, cy + radius + 1):
        for x in range(cx - radius, cx + radius + 1):
            if (x - cx)**2 + (y - cy)**2 <= radius**2:
                set_pixel(x, y, r, g, b, a)

# ─── プレイヤー歩行 4 フレーム [0..255] ─────────────────────────────
# フレーム 0: 直立（基本ポーズ）
def draw_player_frame(ox, leg_offset=0, arm_offset=0):
    """プレイヤーを描画。ox はフレームの左端 X 座標。"""
    # 胴体
    fill_rect(ox+14, 18, ox+50, 50, 0, 160, 200)
    fill_rect(ox+16, 20, ox+48, 48, 30, 200, 240)
    # 頭
    fill_rect(ox+18, 6, ox+46, 20, 0, 160, 200)
    fill_rect(ox+20, 8, ox+44, 18, 30, 200, 240)
    # 目（白）
    fill_rect(ox+22, 10, ox+30, 16, 255, 255, 255)
    fill_rect(ox+34, 10, ox+42, 16, 255, 255, 255)
    # 瞳（黒）
    fill_rect(ox+24, 11, ox+28, 15, 20, 20, 40)
    fill_rect(ox+36, 11, ox+40, 15, 20, 20, 40)
    # 左脚（leg_offset で上下）
    fill_rect(ox+18, 48+leg_offset, ox+28, 58+leg_offset, 0, 130, 170)
    # 右脚（逆位相）
    fill_rect(ox+36, 48-leg_offset, ox+46, 58-leg_offset, 0, 130, 170)
    # 左腕（arm_offset で上下）
    fill_rect(ox+8, 22+arm_offset, ox+16, 36+arm_offset, 0, 140, 180)
    # 右腕（逆位相）
    fill_rect(ox+48, 22-arm_offset, ox+56, 36-arm_offset, 0, 140, 180)

draw_player_frame(0,   leg_offset=0,  arm_offset=0)   # フレーム 0: 直立
draw_player_frame(64,  leg_offset=4,  arm_offset=3)   # フレーム 1: 右足前
draw_player_frame(128, leg_offset=0,  arm_offset=0)   # フレーム 2: 直立（中間）
draw_player_frame(192, leg_offset=-4, arm_offset=-3)  # フレーム 3: 左足前

# ─── Slime バウンス 4 フレーム [256..511] ───────────────────────────
def draw_slime_frame_at(ox_base, squash=0):
    """Slime を描画。ox_base はフレームの左端 X 座標（絶対値）。"""
    y_top    = 12 + squash
    y_bot    = 58
    x_left   = ox_base + 6  - squash
    x_right  = ox_base + 58 + squash
    fill_rect(x_left,   y_top,   x_right,   y_bot,   40,  160, 40)
    fill_rect(x_left+2, y_top+2, x_right-2, y_bot-2, 60,  200, 60)
    fill_rect(x_left+10, y_top+2, x_left+30, y_top+14, 100, 230, 100)
    eye_y = y_top + 14 + max(0, squash) // 2
    fill_rect(x_left+12, eye_y,    x_left+22, eye_y+10, 255, 255, 255)
    fill_rect(x_left+38, eye_y,    x_left+48, eye_y+10, 255, 255, 255)
    fill_rect(x_left+15, eye_y+2,  x_left+20, eye_y+8,  20, 20, 20)
    fill_rect(x_left+41, eye_y+2,  x_left+46, eye_y+8,  20, 20, 20)

draw_slime_frame_at(256,  squash=0)
draw_slime_frame_at(320,  squash=4)
draw_slime_frame_at(384,  squash=0)
draw_slime_frame_at(448,  squash=-3)

# ─── Bat 羽ばたき 2 フレーム [512..639] ─────────────────────────────
def draw_bat_frame(ox, wing_up=True):
    """Bat を描画。wing_up=True で翼を上げた状態。"""
    cx = ox + 32
    # 胴体
    fill_circle(cx, 36, 12, 120, 40, 160)
    fill_circle(cx, 36, 10, 150, 60, 200)
    if wing_up:
        # 翼を上げた状態
        fill_rect(cx-30, 16, cx-14, 36, 100, 30, 140)
        fill_rect(cx-28, 14, cx-16, 34, 120, 40, 160)
        fill_rect(cx+14, 16, cx+30, 36, 100, 30, 140)
        fill_rect(cx+16, 14, cx+28, 34, 120, 40, 160)
    else:
        # 翼を下げた状態
        fill_rect(cx-30, 32, cx-14, 52, 100, 30, 140)
        fill_rect(cx-28, 34, cx-16, 50, 120, 40, 160)
        fill_rect(cx+14, 32, cx+30, 52, 100, 30, 140)
        fill_rect(cx+16, 34, cx+28, 50, 120, 40, 160)
    # 耳
    fill_rect(cx-11, 20, cx-7, 28, 120, 40, 160)
    fill_rect(cx+7,  20, cx+11, 28, 120, 40, 160)
    # 目（赤く光る）
    fill_rect(cx-8, 32, cx-4, 36, 255, 60, 60)
    fill_rect(cx+4, 32, cx+8, 36, 255, 60, 60)

draw_bat_frame(512, wing_up=True)
draw_bat_frame(576, wing_up=False)

# ─── Golem 歩行 2 フレーム [640..767] ───────────────────────────────
def draw_golem_frame(ox, step=0):
    """Golem を描画。step=0 or 1 で足の位置を変える。"""
    # 胴体
    fill_rect(ox+2,  4, ox+62, 52, 80, 80, 80)
    fill_rect(ox+4,  6, ox+60, 50, 110, 110, 110)
    # 岩の質感
    fill_rect(ox+8,  20, ox+58, 22, 70, 70, 70)
    fill_rect(ox+8,  36, ox+58, 38, 70, 70, 70)
    fill_rect(ox+26, 6,  ox+28, 50, 70, 70, 70)
    fill_rect(ox+40, 6,  ox+42, 50, 70, 70, 70)
    # ハイライト
    fill_rect(ox+6,  8,  ox+26, 20, 140, 140, 140)
    # 目（黄色く光る）
    fill_rect(ox+12, 26, ox+22, 34, 255, 200, 0)
    fill_rect(ox+42, 26, ox+52, 34, 255, 200, 0)
    # 瞳（黒）
    fill_rect(ox+15, 28, ox+20, 32, 20, 20, 20)
    fill_rect(ox+45, 28, ox+50, 32, 20, 20, 20)
    # 足（step で左右交互に上げる）
    if step == 0:
        fill_rect(ox+10, 50, ox+26, 62, 90, 90, 90)
        fill_rect(ox+38, 52, ox+54, 62, 90, 90, 90)
    else:
        fill_rect(ox+10, 52, ox+26, 62, 90, 90, 90)
        fill_rect(ox+38, 50, ox+54, 62, 90, 90, 90)

draw_golem_frame(640, step=0)
draw_golem_frame(704, step=1)

# ─── 弾丸: 黄色い円 [768..831] ──────────────────────────────────────
fill_circle(800, 32, 10, 255, 220, 0)
fill_circle(800, 32, 7, 255, 255, 100)

# ─── パーティクル: 白い円 [832..895] ────────────────────────────────
fill_circle(864, 32, 12, 255, 255, 255)
fill_circle(864, 32, 8, 255, 255, 255)

# ─── 経験値宝石: 緑のダイヤ形 [896..959] ────────────────────────────
for dy in range(-18, 19):
    for dx in range(-18, 19):
        if abs(dx) + abs(dy) <= 18:
            set_pixel(928 + dx, 32 + dy, 0, 140, 60)
for dy in range(-14, 15):
    for dx in range(-14, 15):
        if abs(dx) + abs(dy) <= 14:
            set_pixel(928 + dx, 32 + dy, 60, 220, 100)
for dy in range(-6, 4):
    for dx in range(-6, 1):
        if abs(dx) + abs(dy) <= 6:
            set_pixel(928 + dx - 3, 32 + dy - 4, 200, 255, 200)

# ─── 回復ポーション: 赤い瓶 [960..1023] ─────────────────────────────
fill_rect(964, 24, 988, 56, 180, 30, 30)
fill_rect(966, 26, 986, 54, 220, 60, 60)
fill_rect(970, 16, 982, 26, 160, 40, 40)
fill_rect(972, 14, 980, 18, 140, 140, 140)
fill_rect(968, 30, 974, 50, 255, 100, 100)
fill_rect(968, 28, 974, 36, 255, 200, 200)

# ─── 磁石: 黄色いU字型磁石 [1024..1087] ─────────────────────────────
fill_rect(1032, 28, 1048, 56, 180, 140, 0)
fill_rect(1034, 30, 1046, 54, 240, 200, 0)
fill_rect(1064, 28, 1080, 56, 180, 140, 0)
fill_rect(1066, 30, 1078, 54, 240, 200, 0)
fill_rect(1032, 16, 1080, 32, 180, 140, 0)
fill_rect(1034, 18, 1078, 30, 240, 200, 0)
fill_rect(1032, 48, 1048, 58, 220, 40, 40)
fill_rect(1064, 48, 1080, 58, 40, 80, 220)
fill_rect(1036, 20, 1044, 26, 255, 240, 120)

# ─── Fireball 弾丸 [1088..1151] ──────────────────────────────────────
fill_circle(1120, 32, 14, 200, 40, 0)
fill_circle(1120, 32, 10, 255, 120, 0)
fill_circle(1120, 32, 6, 255, 200, 50)
fill_circle(1120, 32, 3, 255, 255, 200)
fill_circle(1120, 24, 5, 255, 140, 20)
fill_circle(1120, 20, 3, 255, 180, 60)

# ─── Lightning 弾丸 [1152..1215] ─────────────────────────────────────
fill_circle(1184, 32, 13, 30, 60, 180)
fill_circle(1184, 32, 9, 60, 160, 255)
fill_circle(1184, 32, 5, 150, 220, 255)
fill_circle(1184, 32, 2, 240, 250, 255)
for i in range(-12, 13):
    if abs(i) > 2:
        alpha_val = max(0, 200 - abs(i) * 14)
        set_pixel(1184 + i, 32, 100, 200, 255, alpha_val)
        set_pixel(1184, 32 + i, 100, 200, 255, alpha_val)

# ─── Whip エフェクト [1216..1279] ────────────────────────────────────
for dy in range(-8, 9):
    for dx in range(-20, 21):
        if (dx / 20.0)**2 + (dy / 8.0)**2 <= 1.0:
            set_pixel(1248 + dx, 32 + dy, 80, 180, 20)
for dy in range(-5, 6):
    for dx in range(-16, 17):
        if (dx / 16.0)**2 + (dy / 5.0)**2 <= 1.0:
            set_pixel(1248 + dx, 32 + dy, 160, 240, 60)
for dy in range(-2, 3):
    for dx in range(-8, 9):
        if (dx / 8.0)**2 + (dy / 2.0)**2 <= 1.0:
            set_pixel(1248 + dx, 32 + dy, 220, 255, 180)

# ─── PNG エンコード ───────────────────────────────────────────────────
def make_png(width, height, rgba_data):
    def chunk(name, data):
        c = name + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)

    ihdr = struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0)

    raw_rows = bytearray()
    for y in range(height):
        raw_rows += b'\x00'
        raw_rows += rgba_data[y * width * 4:(y + 1) * width * 4]

    compressed = zlib.compress(bytes(raw_rows), 9)

    png = b'\x89PNG\r\n\x1a\n'
    png += chunk(b'IHDR', ihdr)
    png += chunk(b'IDAT', compressed)
    png += chunk(b'IEND', b'')
    return png

with open('atlas.png', 'wb') as f:
    f.write(make_png(W, H, pixels))

print(f"atlas.png generated ({W}x{H} RGBA)")
print("  [   0.. 255] Player walk  4 frames (64x64 each)")
print("  [ 256.. 511] Slime bounce 4 frames (64x64 each)")
print("  [ 512.. 639] Bat flap     2 frames (64x64 each)")
print("  [ 640.. 767] Golem walk   2 frames (64x64 each)")
print("  [ 768.. 831] Bullet MagicWand/Axe/Cross (yellow)")
print("  [ 832.. 895] Particle (white)")
print("  [ 896.. 959] Gem (green diamond)")
print("  [ 960..1023] Potion (red bottle)")
print("  [1024..1087] Magnet (yellow U-shape)")
print("  [1088..1151] Fireball bullet (red-orange flame)")
print("  [1152..1215] Lightning bullet (cyan electric)")
print("  [1216..1279] Whip effect (yellow-green arc)")
