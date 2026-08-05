## Summary

Framework changes for DF 53.16 + DFHack 53.15-r3+ compatibility (verified on **DFHack 53.16-r1**): capture text the classic hooks never see, translate it in real time with a **built-in Rust translator (no Python dependency)**, and re-render already-drawn English text in place as Chinese. The function addresses are found by **runtime byte-pattern search**, so the framework adapts automatically across DF/DFHack versions instead of being pinned to one build.

### 1. dfhack_paint_string: symbol resolution instead of byte pattern
Since the DFHack 53.15-r3 refactor, `dfhooks_dfhack.dll` no longer contains the `DFHack::Screen::paintString` function, so the old byte-pattern search found 0 matches and `attach_all` reported `pointer not found for key: dfhack_paint_string`. Switched to resolving the exported symbol `?paintString@Screen@DFHack@@...` from `dfhack.dll` directly (same mechanism the Linux build already uses against `libdfhack.so`), and registered the `dfhack.dll` memory region on Windows. Verified resolving on DFHack 53.16-r1 (`dfhack.dll 0x7ffca3adc960`).

### 2. Capture text rendered via the game's new text path
On DF 53.15/53.16, unit-sheet descriptions and thought strings are composed by dedicated functions and rendered through a newer text path that does **not** write to `gps.screen` and never reaches the classic `addst` hooks — so they were neither translated nor logged (multiple sessions showed zero untranslated entries in `dfi18n.log`).

This PR adds two **read-only logging hooks** that capture those strings:

- `description_composer` (`fn(index, variant, out: *mut std::string)`; observed at `0x140664be0` on DF 53.16)
- `thought_composer` (8-arg ABI, output string at arg 2; observed at `0x140e2c460` on DF 53.16)

Both are located by runtime byte-pattern search (see `configs/functions.lua`) rather than a hardcoded address, so they attach on any version whose pattern still matches. Both call the original function, read the composed string from the output `std::string`, and feed it to `logging::log_text` (with a runtime backtrace at debug level) so untranslated entries show up in the log. They never modify the output, so game logic is untouched. Both attach **optionally**: if the pattern does not match (older/newer DF builds), they are skipped instead of failing `attach_all`.

### 3. Built-in realtime translation (Rust, no Python dependency)
`dfi18n/src/realtime_translate.rs` runs a background Tokio loop that drains the captured untranslated strings, batches them to an OpenAI-compatible chat API (DeepSeek by default, thinking disabled) over plain HTTPS, and writes the results back into the in-memory dictionary **and** appends them to `ai_fill.csv` for persistence. The API key is read from `~/.dfi18n/api_key.txt` or `DEEPSEEK_API_KEY`; without a key the feature disables itself gracefully. No external runtime is required — the previous Python-based tool stays in `tools/` as an optional standalone alternative.

### 4. Re-render already-drawn English text in place (auto redraw, no stutter)
Previously a captured string only became Chinese when the game re-issued `addst`. Now:

- `translator::translate` tries the **in-memory dictionary synchronously first** (plain HashMap lookup, no API, no stutter) and only falls back to the async task for strings the dictionary does not cover — so text that is already in the dictionary is Chinese from its **first frame**.
- `screen::retranslate_all` re-translates every already-rendered block (both screen layers) in place: plain blocks via `apply_translation`, colored/markup blocks (rendered via `addcoloredst`) are re-parsed through the markup engine so `[C:..]` color codes survive. The whole pass runs under one screen write lock (all in-memory work, sub-millisecond) so the game thread cannot clear blocks mid-pass — this also fixes a crash where a cached screen index went stale after a re-render.
- The realtime loop sweeps every tick (default 3 s), so a block that was drawn English on its first frame becomes Chinese within a few seconds without waiting for the game to redraw.

### Verification (DF 53.16 / DFHack 53.16-r1)
- All 16 hook pointers resolve (including `description_composer` at `0x140664be0`, `thought_composer` at `0x140e2c460`, `dfhack_paint_string` via symbol); no attach error.
- 122 untranslated entries captured in a single fortress session, including composed thought strings and book titles that previously never appeared in the log.
- Pipeline behavior implemented and exercised in-game: captured personality-description fragments (`[C:..]Like others in his culture, ...`, `[C:..]He is weak.`, ...) become Chinese — on first draw for strings already in the dictionary (synchronous lookup), and in place a few seconds after the realtime translator fills a newly captured dictionary entry (atomic retranslate sweep).

---

## 摘要（中文）

DF 53.16 / DFHack 53.15-r3+ 的兼容性修复（在 **DFHack 53.16-r1** 上验证），对经典钩子覆盖不到文本的捕获，以及**内置 Rust 实时翻译（零 Python 依赖）**与**已渲染文本就地重绘**。函数地址均由**运行时字节模式搜索**定位，框架自动适配不同 DF/DFHack 版本，不锁定单一构建：

1. **dfhack_paint_string 改为符号解析**：DFHack 53.15-r3 重构后 `dfhooks_dfhack.dll` 不再包含该函数，旧字节模式失配。改为直接从 `dfhack.dll` 解析导出符号（与 Linux 端机制一致），并注册 `dfhack.dll` 内存区域。已在 DFHack 53.16-r1 上验证解析成功。

2. **捕获新渲染路径的文本**：DF 53.15/53.16 的单位档案描述与想法句由专用函数组合、经新文本路径渲染，不写 `gps.screen`、不经过经典 `addst` 钩子——既不翻译也不进日志。新增两个**只读日志钩子**（调用原函数后读取组合串送入 `logging::log_text`，不改输出；debug 级别附真实调用栈）：
   - `description_composer`（DF 53.16 上观测地址 `0x140664be0`）
   - `thought_composer`（DF 53.16 上观测地址 `0x140e2c460`）
   两者均经**运行时字节模式搜索**定位（见 `configs/functions.lua`），不依赖硬编码地址；均为**可选附加**：模式不匹配（其它版本）时跳过，不影响 `attach_all`。

3. **内置实时翻译（Rust）**：后台 Tokio 循环批量调用 OpenAI 兼容接口（默认 DeepSeek，thinking 关闭），译文即写回内存词典并追加到 `ai_fill.csv`。密钥读自 `~/.dfi18n/api_key.txt` 或环境变量，无密钥自动禁用；无需安装 Python。原 Python 工具保留在 `tools/` 作独立可选方案。

4. **已渲染英文文本就地重绘（不卡顿）**：
   - `translate` 先**同步查内存词典**（纯 HashMap 查找）再走异步——词典已有的文本**首帧即中文**；
   - `retranslate_all` 就地重译双层屏幕内所有已渲染块：普通块 `apply_translation`，彩色/markup 块（`addcoloredst` 路径）经 markup 引擎重建以保留 `[C:..]` 颜色码；全程单写锁（纯内存操作、亚毫秒），杜绝游戏线程中途清块的竞争（同时修复了因屏幕索引过期导致的崩溃）；
   - 实时循环每 tick（默认 3 秒）清扫一次，首帧英文块几秒内自动变中文，无需等游戏重绘。

5. **验证（DF 53.16 / DFHack 53.16-r1）**：16 个钩子指针全部解析（含 `description_composer`@`0x140664be0`、`thought_composer`@`0x140e2c460`、符号解析的 `dfhack_paint_string`），无附加错误；单次会话捕获 122 条未翻译词条（含组合想法句与书名）；管线行为已在游戏中实现并验证——性格描述片段（`[C:..]Like others in his culture, ...`、`[C:..]He is weak.` 等）在词典已有译文时首帧即中文（同步查找），实时翻译补上新词条后几秒内就地重绘（原子清扫）。
