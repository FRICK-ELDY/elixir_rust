"""
アトラス画像生成スクリプト（共通・assets/_shared）
出力先を第1引数で指定可能。例:
    python assets/_shared/gen_atlas.py assets/vampire_survivor/sprites
"""
import sys
import os
sys.path.insert(0, os.path.join(os.path.dirname(__file__), '..'))
# sprites/gen_atlas.py をそのまま実行（モジュールとして読むと __main__ が違うため、exec で実行）
with open(os.path.join(os.path.dirname(__file__), '..', 'sprites', 'gen_atlas.py'), encoding='utf-8') as f:
    exec(compile(f.read(), 'gen_atlas.py', 'exec'))
