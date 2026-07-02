// Benchmark zhhz via its Node.js / WASM entry point.
// Run:  node tests/bench-node.mjs [config]
//
// Prints MB/s for repeated convert() calls over a realistic 2 MiB
// corpus. Designed to mirror the native CLI's bench_baseline.sh shape
// so the npm and CLI numbers can be compared directly.
//
// Uses init() with an explicit .wasm path because the wasm-pack
// bundler target's default `import` syntax needs a real bundler to
// resolve. Reading the bytes ourselves makes the benchmark runnable
// from a plain Node script.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const wasmBytes = readFileSync(resolve(__dirname, "../pkg/zhhz_bg.wasm"));

const { default: init, convert, Converter } = await import("../pkg/zhhz.js");
// wasm-bindgen 0.2's `init(bytes)` is the deprecated calling convention;
// the new style is `init({ module_or_path: bytes })`. Either works, but
// the object form silences the deprecation warning.
await init({ module_or_path: wasmBytes });

const config = process.argv[2] ?? "s2twp";

// Realistic Chinese corpus — a representative mix of news + literary
// text with mixed CJK / Latin / digit characters.
const seeds = [
  "汉字计算机软件在过去的几十年里发生了翻天覆地的变化,从最初的命令行界面到现在的图形用户界面,用户体验不断提升。",
  "随着移动互联网的快速发展,人们获取信息的方式也在发生深刻的变化,从传统的报纸杂志到如今的社交媒体平台。",
  "他感染了A型肝炎,医生建议他多休息,多喝水,按时服药,避免过度劳累,保持良好的生活习惯。",
  "鼠标和键盘是计算机最基本的输入设备,无论是台式机还是笔记本电脑,都离不开这两样东西。",
  "近年来,人工智能技术突飞猛进,从图像识别到自然语言处理,从自动驾驶到智能客服,AI正在改变着我们的生活方式。",
  "她去了西維珍尼亞州,那裡的空氣清新,風景優美,是個適合度假的好地方,每年都吸引著大量遊客。",
  "正则表达式是一种强大的文本处理工具,可以用来匹配、查找、替换符合特定模式的字符串。",
  "鼠标是计算机最常用的输入设备之一,无论是工作还是娱乐,都离不开它。",
  "他说:\"我们需要更加努力,才能在这个竞争激烈的时代中立于不败之地。\"",
  "The quick brown fox jumps over the lazy dog. 0123456789 !@#$%^&*()",
];

function buildCorpus(seeds, targetBytes) {
  let s = "";
  let i = 0;
  while (s.length < targetBytes) {
    s += seeds[i % seeds.length];
    i++;
  }
  return s.slice(0, targetBytes);
}

const targetBytes = 2 * 1024 * 1024; // 2 MiB
const text = buildCorpus(seeds, targetBytes);
const bytes = Buffer.byteLength(text, "utf8");
console.log(`corpus: ${bytes} bytes (${(bytes / 1024 / 1024).toFixed(2)} MiB), config=${config}`);

const converter = new Converter(config);

// Warmup
for (let i = 0; i < 3; i++) converter.convert(text);

// Measure
const runs = 5;
const times = [];
for (let i = 0; i < runs; i++) {
  const t0 = process.hrtime.bigint();
  const out = converter.convert(text);
  const t1 = process.hrtime.bigint();
  times.push(Number(t1 - t0) / 1e6); // ms
}

const best = Math.min(...times);
const median = [...times].sort()[Math.floor(times.length / 2)];
const worst = Math.max(...times);
const mb = bytes / 1024 / 1024;

console.log("\nresults (Converter.convert, sync, 5 runs):");
console.log(`  best:   ${best.toFixed(2)} ms  (${(mb / (best / 1000)).toFixed(2)} MB/s)`);
console.log(`  median: ${median.toFixed(2)} ms  (${(mb / (median / 1000)).toFixed(2)} MB/s)`);
console.log(`  worst:  ${worst.toFixed(2)} ms  (${(mb / (worst / 1000)).toFixed(2)} MB/s)`);

// Compare one-shot vs instance: one-shot allocates a fresh Converter
// per call (slower, like calling convert() in a hot loop).
const t0 = process.hrtime.bigint();
const r = convert(text, config);
const t1 = process.hrtime.bigint();
console.log(`\none-shot convert (${mb.toFixed(2)} MiB): ${(Number(t1 - t0) / 1e6).toFixed(2)} ms (${(mb / ((Number(t1 - t0)) / 1e9)).toFixed(2)} MB/s)`);
console.log(`ratio (one-shot / instance): ${(Number(t1 - t0) / 1e6 / median).toFixed(1)}x slower`);