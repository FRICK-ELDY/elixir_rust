"""
音声ファイル生成スクリプト（共通・assets/_shared）
出力先を第1引数で指定可能。例:
    python assets/_shared/gen_audio.py assets/vampire_survivor/audio
"""
import sys
import os
with open(os.path.join(os.path.dirname(__file__), '..', 'audio', 'gen_audio.py'), encoding='utf-8') as f:
    exec(compile(f.read(), 'gen_audio.py', 'exec'))
