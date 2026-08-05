#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
realtime_translate.py — OPTIONAL helper for the dfi18n framework

While the game runs, this script tails <game>/dfi18n-data/logs/dfi18n.log,
batch-translates newly captured untranslated entries through an OpenAI-
compatible chat API (DeepSeek by default, thinking disabled), and appends the
translations to the translation dictionary CSV (creating ai_fill.csv if the
data mod does not have one yet). Run `dfi18n reload` in the DFHack console to
hot-load the new entries.

This tool is optional: the framework works fully offline without it. It only
automates the "collect -> translate -> fill dictionary" loop.

Usage:
    python realtime_translate.py [options]

Options (highest precedence first):
    --config <path>      config file (JSON, see config.example.json)
    --log <path>         path to dfi18n.log
    --data-dir <path>    directory containing the dictionary CSVs
    --thinking on|off    enable/disable model reasoning (default off)
    --interval <sec>     log poll interval (default 3)
    --batch <n>          max strings per translation batch (default 40)

API key resolution (in order):
    1. environment variable DEEPSEEK_API_KEY
    2. ~/.dfi18n/api_key.txt
    3. interactive prompt (saved to ~/.dfi18n/api_key.txt)
"""
import json
import os
import re
import sys
import time
import urllib.request
import urllib.error

# ---------------- defaults & config ----------------
DEFAULT_CONFIG = {
    "log": None,             # auto-detected when None
    "data_dir": None,        # auto-detected when None
    "api": {
        "base_url": "https://api.deepseek.com/chat/completions",
        "model": "deepseek-v4-flash",
        "thinking": "disabled",   # {"type": "disabled"} -> no reasoning
        "temperature": 0.3,
    },
    "interval": 3,
    "batch_max": 40,
}

HEADER_RE = re.compile(r'========== [a-z_]+:"(.*)":[0-9a-f]{64}$', re.MULTILINE)

# ---------------- path helpers ----------------
def script_dir():
    return os.path.dirname(os.path.abspath(__file__))


def user_cfg_dir():
    return os.path.join(os.path.expanduser("~"), ".dfi18n")


def key_file_path():
    return os.path.join(user_cfg_dir(), "api_key.txt")


def detect_log_path():
    """Common install locations; override with --log/config if not found."""
    candidates = []
    pf = os.environ.get("ProgramFiles(x86)") or r"C:\Program Files (x86)"
    candidates.append(os.path.join(pf, "Steam", "steamapps", "common",
                                   "Dwarf Fortress", "dfi18n-data", "logs", "dfi18n.log"))
    pf64 = os.environ.get("ProgramFiles") or r"C:\Program Files"
    candidates.append(os.path.join(pf64, "Steam", "steamapps", "common",
                                   "Dwarf Fortress", "dfi18n-data", "logs", "dfi18n.log"))
    candidates.append(os.path.expanduser("~/dfi18n-data/logs/dfi18n.log"))
    for c in candidates:
        if os.path.isfile(c):
            return c
    return None


def detect_data_dir():
    """Common user-data locations; override with --data-dir/config if not found."""
    candidates = []
    apd = os.environ.get("APPDATA")
    if apd:
        base = os.path.join(apd, "Bay 12 Games", "Dwarf Fortress", "mods")
        candidates.append(base)
    xdg = os.environ.get("XDG_DATA_HOME")
    if xdg:
        candidates.append(os.path.join(xdg, "Bay 12 Games", "Dwarf Fortress", "mods"))
    candidates.append(os.path.expanduser("~/.local/share/Dwarf Fortress/mods"))
    for base in candidates:
        if not os.path.isdir(base):
            continue
        found = []
        for entry in sorted(os.listdir(base)):
            d = os.path.join(base, entry, "dfi18n-data", "simple", "zh-Hans")
            if os.path.isdir(d):
                found.append(d)
        if not found:
            continue
        # prefer a data dir that already has ai_fill.csv (an actively-used pack)
        for d in found:
            if os.path.isfile(os.path.join(d, "ai_fill.csv")):
                return d
        return found[0]
    return None


# ---------------- config loading ----------------
def load_config(args):
    cfg = json.loads(json.dumps(DEFAULT_CONFIG))  # deep copy
    # config file (cli --config > script dir > user cfg dir)
    for cand in ([args.get("config")] if args.get("config") else
                 [os.path.join(script_dir(), "realtime_translate.json"),
                  os.path.join(user_cfg_dir(), "realtime_translate.json")]):
        if cand and os.path.isfile(cand):
            try:
                with open(cand, encoding="utf-8") as f:
                    user = json.load(f)
                cfg.update({k: v for k, v in user.items() if v is not None})
                if "api" in user:
                    cfg["api"].update({k: v for k, v in user["api"].items() if v is not None})
                print(f"[配置] 已加载 {cand}")
                break
            except (OSError, ValueError) as e:
                print(f"[警告] 配置文件 {cand} 解析失败: {e}")
    # CLI overrides
    if args.get("log"):
        cfg["log"] = args["log"]
    if args.get("data_dir"):
        cfg["data_dir"] = args["data_dir"]
    if args.get("thinking"):
        cfg["api"]["thinking"] = "enabled" if args["thinking"].lower() in ("on", "enabled", "1", "true") else "disabled"
    if args.get("interval"):
        cfg["interval"] = int(args["interval"])
    if args.get("batch"):
        cfg["batch_max"] = int(args["batch"])
    # auto-detect missing paths
    if not cfg["log"]:
        cfg["log"] = detect_log_path()
    if not cfg["data_dir"]:
        cfg["data_dir"] = detect_data_dir()
    return cfg


# ---------------- API key ----------------
def load_api_key(prompt=True):
    env = os.environ.get("DEEPSEEK_API_KEY")
    if env and env.strip():
        return env.strip()
    path = key_file_path()
    if os.path.isfile(path):
        with open(path, encoding="utf-8") as f:
            k = f.read().strip()
        if k:
            return k
    if not prompt:
        return None  # daemon mode: no interactive prompt, caller decides
    print("未检测到 API Key。")
    print("设置环境变量 DEEPSEEK_API_KEY, 或将 Key 写入 %s," % key_file_path())
    print("或现在输入(将保存到本地, 不会进入任何项目):")
    k = input("请输入 API Key: ").strip()
    if not k:
        print("[错误] 未输入 Key")
        sys.exit(1)
    try:
        os.makedirs(user_cfg_dir(), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(k + "\n")
        print(f"[OK] Key 已保存到 {path}")
    except OSError as e:
        print(f"[警告] 保存 Key 失败({e}), 本次运行仍使用内存中的 Key")
    return k


# ---------------- dictionary ----------------
def load_dict(data_dir):
    keys = set()
    for fn in os.listdir(data_dir):
        if not fn.endswith(".csv"):
            continue
        with open(os.path.join(data_dir, fn), encoding="utf-8", errors="replace") as f:
            for line in f:
                if line.startswith("text,"):
                    continue
                m = re.match(r'^"((?:[^"]|"")*)"', line) or re.match(r"^([^,]+),", line)
                if m:
                    keys.add(m.group(1).replace('""', '"'))
    return keys


def unescape_content(s):
    return (s.replace('\\"', '"').replace("\\\\", "\\")
             .replace("\\n", "\n").replace("\\t", "\t")
             .replace("\\r", "\r").replace("\\0", "\0"))


def should_skip(s):
    if len(s) < 2 or s.startswith("FPS: "):
        return True
    if not any(c.isalpha() for c in s):
        return True
    return False


def read_new_strings(log_path, offset):
    try:
        size = os.path.getsize(log_path)
    except OSError:
        return [], offset
    if size <= offset:
        return [], size
    with open(log_path, encoding="utf-8", errors="replace") as f:
        f.seek(offset)
        new = f.read()
    strings = []
    for m in HEADER_RE.finditer(new):
        s = unescape_content(m.group(1))
        if not should_skip(s):
            strings.append(s)
    return strings, size


# ---------------- LLM API ----------------
def translate_batch(strings, cfg, api_key):
    prompt = (
        "你是矮人要塞(Dwarf Fortress)简体中文专业译者。把以下英文界面/描述/想法"
        "文本逐条翻译成简体中文。\n"
        "规则：\n"
        "- translations 是 JSON 数组, 数量与输入完全一致, 第 i 个元素是第 i 条翻译(严格按顺序)。\n"
        "- 保留占位/标记/数字/标点(如 [R]、-->、*、括号、's、斜杠)。\n"
        "- 内部标识/无意义串原样保留不译。\n"
        "- 专有名词可音译或保留英文。\n"
        "- 只输出 JSON 对象 {\"translations\": [...]}, 不要任何额外文字。\n\n"
        f"输入(JSON数组, {len(strings)}条)：\n{json.dumps(strings, ensure_ascii=False)}"
    )
    body = {
        "model": cfg["api"]["model"],
        "messages": [{"role": "user", "content": prompt}],
        "temperature": cfg["api"]["temperature"],
        "thinking": {"type": cfg["api"]["thinking"]},
    }
    req = urllib.request.Request(
        cfg["api"]["base_url"],
        data=json.dumps(body).encode("utf-8"),
        headers={"Content-Type": "application/json",
                 "Authorization": "Bearer " + api_key},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    content = data["choices"][0]["message"]["content"]
    m = re.search(r"\[.*\]", content, re.S)
    if not m:
        raise ValueError("响应中未找到 JSON 数组: " + content[:200])
    tr = json.loads(m.group(0))
    if not isinstance(tr, list) or len(tr) != len(strings):
        raise ValueError(f"数量不匹配: 输入{len(strings)} 输出{len(tr)}")
    return dict(zip(strings, tr))


def append_csv(data_dir, entries):
    """Append to ai_fill.csv in the data dir (create it with a header if missing)."""
    path = os.path.join(data_dir, "ai_fill.csv")
    if not os.path.isfile(path):
        with open(path, "w", encoding="utf-8", newline="") as f:
            f.write("text,translation,tags\n")

    def esc(v):
        return '"' + v.replace('"', '""') + '"' if ("," in v or '"' in v or "\n" in v) else v

    with open(path, "a", encoding="utf-8", newline="") as f:
        for s, t in entries:
            if not t or s == t:
                continue
            f.write(f"{esc(s)},{esc(t)},\n")


# ---------------- main ----------------
def game_running():
    """Check whether the game process is alive (Windows/Linux)."""
    try:
        if os.name == "nt":
            out = subprocess.run(
                ["tasklist", "/FI", "IMAGENAME eq Dwarf Fortress.exe"],
                capture_output=True, text=True, timeout=10)
            return "Dwarf Fortress.exe" in out.stdout
        else:
            out = subprocess.run(["pgrep", "-f", "Dwarf Fortress"],
                                 capture_output=True, text=True, timeout=10)
            return out.returncode == 0
    except Exception:
        return True  # assume alive on errors


def main():
    daemon = "--daemon" in sys.argv
    argv = [a for a in sys.argv[1:] if a != "--daemon"]
    args = {}
    for flag, dest in (("--config", "config"), ("--log", "log"), ("--data-dir", "data_dir"),
                       ("--thinking", "thinking"), ("--interval", "interval"), ("--batch", "batch")):
        if flag in argv:
            args[dest] = argv[argv.index(flag) + 1]

    cfg = load_config(args)
    if not cfg["log"] or not os.path.isfile(cfg["log"]):
        print("[错误] 找不到 dfi18n.log, 请用 --log <路径> 指定, 或确认游戏已启动过")
        sys.exit(1)
    if not cfg["data_dir"] or not os.path.isdir(cfg["data_dir"]):
        print("[错误] 找不到词典目录, 请用 --data-dir <路径> 指定")
        sys.exit(1)

    api_key = load_api_key(prompt=not daemon)
    if not api_key:
        print("[提示] 未配置 API Key (env DEEPSEEK_API_KEY 或 ~/.dfi18n/api_key.txt), 跳过")
        sys.exit(0)
    known = load_dict(cfg["data_dir"])
    print(f"[启动] 词典 {len(known)} 条 | 模型 {cfg['api']['model']} "
          f"(thinking={cfg['api']['thinking']}) | 轮询 {cfg['interval']}s"
          + (" | daemon" if daemon else ""))
    print(f"       log: {cfg['log']}")
    print(f"       data: {cfg['data_dir']}")
    # start from offset 0 so any already-captured (backlog) entries are
    # translated too; entries already in the dictionary are skipped
    offset = 0
    pending = []
    processed = set(known)
    total_added = 0
    last_flush = time.time()

    try:
        while True:
            if daemon and not game_running():
                print("[退出] 游戏已关闭, 停止实时翻译")
                break
            strings, offset = read_new_strings(cfg["log"], offset)
            new = [s for s in strings if s not in processed and s not in pending]
            if new:
                pending.extend(new)
                print(f"[捕获] +{len(new)} 条 (队列 {len(pending)})")
            if pending and (len(pending) >= cfg["batch_max"] or time.time() - last_flush >= 15):
                batch, pending = pending[:cfg["batch_max"]], pending[cfg["batch_max"]:]
                try:
                    t0 = time.time()
                    result = translate_batch(batch, cfg, api_key)
                    dt = time.time() - t0
                    real = [(s, result[s]) for s in batch if s in result and result[s] and s != result[s]]
                    append_csv(cfg["data_dir"], real)
                    for s in batch:
                        processed.add(s)
                    total_added += len(real)
                    print(f"[翻译] {len(batch)} 条 / {dt:.1f}s / 新增 {len(real)} 条 "
                          f"(累计 {total_added}) | 例: {real[0] if real else ''}")
                except (urllib.error.URLError, urllib.error.HTTPError, ValueError, KeyError, IndexError) as e:
                    print(f"[错误] {e} — 稍后重试这批")
                    pending = batch + pending
                last_flush = time.time()
            time.sleep(cfg["interval"])
    except KeyboardInterrupt:
        print(f"\n[退出] 本次会话新增 {total_added} 条。游戏中执行 `dfi18n reload` 可热加载。")


if __name__ == "__main__":
    main()
