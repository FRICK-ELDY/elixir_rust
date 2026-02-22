"""
アトラス画像生成スクリプト
320x64 px の RGBA PNG を生成する:
  [  0.. 63] x [0..63]: プレイヤー（水色の正方形）
  [ 64..127] x [0..63]: 敵スライム（赤い正方形）
  [128..191] x [0..63]: 弾丸（黄色い円）
  [192..255] x [0..63]: パーティクル（白い円）
  [256..319] x [0..63]: 予備（透明）
"""

import struct
import zlib

W, H = 320, 64

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

# 敵スライム: 緑がかった赤い楕円形
fill_rect(66, 10, 126, 58, 180, 40, 40)
fill_rect(68, 8, 124, 60, 200, 60, 60)
# 目（白）
fill_rect(78, 22, 88, 32, 255, 255, 255)
fill_rect(104, 22, 114, 32, 255, 255, 255)
# 瞳（黒）
fill_rect(81, 24, 86, 30, 20, 20, 20)
fill_rect(107, 24, 112, 30, 20, 20, 20)

# 弾丸: 黄色い円
fill_circle(160, 32, 10, 255, 220, 0)
fill_circle(160, 32, 7, 255, 255, 100)

# パーティクル: 白い円（小さめ、ソフトエッジ）
fill_circle(224, 32, 12, 255, 255, 255)
fill_circle(224, 32, 8, 255, 255, 255)

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

print("atlas.png generated (320x64 RGBA)")
