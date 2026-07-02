// Standalone smoke test for the published WebAssembly artifact.
// Loaded by .github/workflows/wasm.yml (no shell-quoting risk in a
// YAML heredoc). Exits 0 only when every assertion holds.
//
// Run locally after `wasm-pack build --target bundler --features wasm`:
//   node tests/wasm-smoke.mjs

import {
  convert,
  convert_with_custom,
  detect,
  listConfigs,
  listLocales,
  configForRegionPair,
  Converter,
} from "../pkg/zhhz.js";

let passed = 0;
let failed = 0;

function assert(label, actual, expected) {
  if (actual === expected) {
    console.log(`  ok  ${label} -> ${JSON.stringify(actual)}`);
    passed++;
  } else {
    console.error(
      `  FAIL ${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
    failed++;
  }
}

console.log("zhhz wasm smoke test\n");

// 1. One-shot conversions.
assert("s2t", convert("汉字计算机软件", "s2t"), "漢字計算機軟件");
assert("tw2sp", convert("他感染了A型肝炎", "tw2sp"), "他感染了甲型肝炎");
assert("s2twp 信息", convert("信息", "s2twp"), "資訊");
assert("s2tw 鼠标", convert("鼠标", "s2tw"), "鼠標");
assert("s2twp 鼠标", convert("鼠标", "s2twp"), "滑鼠");
assert("s2hk 鼠标", convert("鼠标", "s2hk"), "鼠標");

// 2. detect — zhhz-only vs opencc-js.
const d = detect("他去了西維珍尼亞州");
assert("detect region cn-hk", d && d.region, "cn-hk");
assert(
  "detect confidence >= 30",
  d && d.confidence >= 30,
  true,
);

// 3. Custom words — both array and string form.
assert(
  "convert_with_custom array",
  convert_with_custom("买软件", "s2t", [["软件", "軟體"]]),
  "買軟體",
);
assert(
  "convert_with_custom string",
  convert_with_custom("香蕉", "s2t", "香蕉 banana|蘋果 apple"),
  "banana",
);

// 4. Introspection.
assert("listConfigs length 16", listConfigs().length, 16);
assert("listConfigs includes s2twp", listConfigs().includes("s2twp"), true);
assert("listLocales length 6", listLocales().length, 6);
assert("listLocales includes cn-tw", listLocales().includes("cn-tw"), true);

// 5. Semantic region -> config.
assert("configForRegionPair cn-s->cn-tw", configForRegionPair("cn-s", "cn-tw"), "s2twp");
assert("configForRegionPair cn-s->cn-hk", configForRegionPair("cn-s", "cn-hk"), "s2hkp");
assert("configForRegionPair cn-t->cn-s", configForRegionPair("cn-t", "cn-s"), "t2s");

// 6. Converter class — strictly better than opencc-js's closure factory.
const c = new Converter("s2twp");
assert("Converter c.config", c.config, "s2twp");
assert("Converter c.convert 信息", c.convert("信息"), "資訊");
assert(
  "Converter c.convertWithCustom",
  c.convertWithCustom("买软件", [["软件", "軟體"]]),
  "買軟體",
);

const cRegion = Converter.forRegion("cn-s", "cn-tw");
assert("Converter.forRegion config", cRegion.config, "s2twp");
assert("Converter.forRegion convert 鼠标", cRegion.convert("鼠标"), "滑鼠");

const cBaked = c.withCustom([["软件", "軟體"]]);
assert("Converter.withCustom convert", cBaked.convert("买软件"), "買軟體");

// 7. Error paths.
let threw = false;
try {
  convert("hello", "bogus");
} catch {
  threw = true;
}
assert("bad config throws", threw, true);

threw = false;
try {
  convert_with_custom("hello", "s2t", "");
} catch {
  threw = true;
}
assert("empty custom dict throws", threw, true);

threw = false;
try {
  configForRegionPair("cn-s", "xx-yy");
} catch {
  threw = true;
}
assert("bad locale throws", threw, true);

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);