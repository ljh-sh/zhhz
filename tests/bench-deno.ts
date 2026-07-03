// Deno benchmark for zhhz — exercises the npm: specifier path (WASM).
// Run:  deno run --allow-read --allow-env --allow-net tests/bench-deno.ts [config]
//
// Imports the published zhhz@0.7.9 (WASM) and measures convert() MB/s
// on the same 5.18 MiB mixed CJK / Latin corpus used in tests/bench-node.mjs.
// Allows direct comparison: Deno's WASM vs Node.js's WASM.

import { convert, Converter } from "npm:zhhz@0.7.9";

const config = Deno.args[0] ?? "s2twp";

// Realistic Chinese corpus — same as tests/bench-node.mjs.
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

function buildCorpus(seeds: string[], targetBytes: number): string {
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
const bytes = new TextEncoder().encode(text).length;
console.log(
  `corpus: ${bytes} bytes (${(bytes / 1024 / 1024).toFixed(2)} MiB), config=${config}`,
);

const converter = new Converter(config);

// Warmup
for (let i = 0; i < 3; i++) converter.convert(text);

// Measure
const runs = 5;
const times: number[] = [];
for (let i = 0; i < runs; i++) {
  const t0 = performance.now();
  const out = converter.convert(text);
  const t1 = performance.now();
  times.push(t1 - t0);
}

const best = Math.min(...times);
const median = [...times].sort()[Math.floor(times.length / 2)];
const worst = Math.max(...times);
const mb = bytes / 1024 / 1024;

console.log("\nresults (Converter.convert, sync, 5 runs):");
console.log(`  best:   ${best.toFixed(2)} ms  (${(mb / (best / 1000)).toFixed(2)} MB/s)`);
console.log(`  median: ${median.toFixed(2)} ms  (${(mb / (median / 1000)).toFixed(2)} MB/s)`);
console.log(`  worst:  ${worst.toFixed(2)} ms  (${(mb / (worst / 1000)).toFixed(2)} MB/s)`);

// Compare to one-shot convert
const t0 = performance.now();
const r = convert(text, config);
const t1 = performance.now();
console.log(
  `\none-shot convert (${mb.toFixed(2)} MiB): ${(t1 - t0).toFixed(2)} ms (${
    (mb / ((t1 - t0) / 1000)).toFixed(2)
  } MB/s)`,
);
console.log(
  `ratio (one-shot / instance): ${((t1 - t0) / median).toFixed(1)}x slower`,
);
