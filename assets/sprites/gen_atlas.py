"""
アトラス画像生成スクリプト
384x64 px の RGBA PNG を生成する:
  [  0.. 63] x [0..63]: プレイヤー（水色の正方形）
  [ 64..127] x [0..63]: Slime（緑のスライム）
  [128..191] x [0..63]: Bat（紫のコウモリ）
  [192..255] x [0..63]: Golem（灰色のゴーレム）
  [256..319] x [0..63]: 弾丸（黄色い円）
  [320..383] x [0..63]: パーティクル（白い円）
"""

import struct
import zlib

W, H = 384, 64

pixels = bytearray(W * H * 4)

def set_pixel(x, y, r, g, b, a=255):
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
                if 0 <= x < W and 0 <= y < H:
                    set_pixel(x, y, r, g, b, a)

# プレイヤー: 水色の正方形（4px の明るい枠付き）
fill_rect(2, 2, 62, 62, 0, 180, 220)
fill_rect(4, 4, 60, 60, 30, 210, 255)
# 目（白い点）
fill_rect(16, 20, 26, 28, 255, 255, 255)
fill_rect(38, 20, 48, 28, 255, 255, 255)
# 瞳（黒い点）
fill_rect(19, 22, 24, 26, 20, 20, 40)
fill_rect(41, 22, 46, 26, 20, 20, 40)

# Slime: 緑のスライム（丸みのある楕円形）
fill_rect(66, 14, 126, 58, 40, 160, 40)
fill_rect(68, 12, 124, 56, 60, 200, 60)
# ハイライト（明るい緑）
fill_rect(76, 14, 96, 28, 100, 230, 100)
# 目（白）
fill_rect(78, 26, 88, 36, 255, 255, 255)
fill_rect(104, 26, 114, 36, 255, 255, 255)
# 瞳（黒）
fill_rect(81, 28, 86, 34, 20, 20, 20)
fill_rect(107, 28, 112, 34, 20, 20, 20)

# Bat: 紫のコウモリ（小さめ、翼付き）
# 胴体（中央）
fill_circle(160, 36, 12, 120, 40, 160)
fill_circle(160, 36, 10, 150, 60, 200)
# 左翼
fill_rect(132, 28, 150, 44, 100, 30, 140)
fill_rect(130, 30, 148, 42, 120, 40, 160)
# 右翼
fill_rect(170, 28, 188, 44, 100, 30, 140)
fill_rect(172, 30, 190, 42, 120, 40, 160)
# 耳（小さな三角形）
fill_rect(153, 20, 157, 28, 120, 40, 160)
fill_rect(163, 20, 167, 28, 120, 40, 160)
# 目（赤く光る）
fill_rect(154, 32, 158, 36, 255, 60, 60)
fill_rect(162, 32, 166, 36, 255, 60, 60)

# Golem: 灰色のゴーレム（大きく重厚）
fill_rect(194, 4, 254, 62, 80, 80, 80)
fill_rect(196, 6, 252, 60, 110, 110, 110)
# 岩の質感（暗い線）
fill_rect(200, 20, 250, 22, 70, 70, 70)
fill_rect(200, 38, 250, 40, 70, 70, 70)
fill_rect(218, 6, 220, 60, 70, 70, 70)
fill_rect(234, 6, 236, 60, 70, 70, 70)
# ハイライト（明るい灰）
fill_rect(198, 8, 218, 20, 140, 140, 140)
# 目（黄色く光る）
fill_rect(206, 26, 216, 34, 255, 200, 0)
fill_rect(238, 26, 248, 34, 255, 200, 0)
# 瞳（黒）
fill_rect(209, 28, 214, 32, 20, 20, 20)
fill_rect(241, 28, 246, 32, 20, 20, 20)

# 弾丸: 黄色い円
fill_circle(288, 32, 10, 255, 220, 0)
fill_circle(288, 32, 7, 255, 255, 100)

# パーティクル: 白い円（小さめ、ソフトエッジ）
fill_circle(352, 32, 12, 255, 255, 255)
fill_circle(352, 32, 8, 255, 255, 255)

# PNG エンコード（標準ライブラリのみ使用）
def make_png(width, height, rgba_data):
    def chunk(name, data):
        c = name + data
        return struct.pack('>I', len(data)) + c + struct.pack('>I', zlib.crc32(c) & 0xffffffff)

    # color_type=6: RGBA (bit_depth=8, compression=0, filter=0, interlace=0)
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

print("atlas.png generated (384x64 RGBA)")
print("  [  0.. 63] Player")
print("  [ 64..127] Slime (green)")
print("  [128..191] Bat   (purple)")
print("  [192..255] Golem (gray)")
print("  [256..319] Bullet (yellow)")
print("  [320..383] Particle (white)")
