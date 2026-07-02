// Runnable Node.js example for the zhhz npm package.
// Run with:    npm install && npm start
//
// Demonstrates the full npm surface (which is strictly richer than opencc-js):
//   convert / convert_with_custom / detect / listConfigs / listLocales
//   / configForRegionPair / Converter (factory instance) / Converter.forRegion
//   / Converter.withCustom
//
// Each case prints one line. The script exits 0 only if every case matches.

import {
  convert,
  convert_with_custom,
  detect,
  listConfigs,
  listLocales,
  configForRegionPair,
  Converter,
} from "zhhz";

let passed = 0;
let failed = 0;

function check(label, actual, expected) {
  const ok = actual === expected;
  if (ok) {
    console.log(`  ok  ${label} -> ${JSON.stringify(actual)}`);
    passed++;
  } else {
    console.error(`  FAIL ${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    failed++;
  }
}

console.log("zhhz example — exercising the npm API surface\n");

// 1. One-shot conversion: cn-s -> cn-t (OpenCC standard).
check("convert s2t", convert("汉字计算机软件", "s2t"), "漢字計算機軟件");

// 2. One-shot conversion: cn-tw (with phrases) -> cn-s.
//    Note A -> 甲 is Taiwan-style.
check("convert tw2sp", convert("他感染了A型肝炎", "tw2sp"), "他感染了甲型肝炎");

// 3. Script-variant detection — zhhz-only (opencc-js has no equivalent).
const d = detect("他去了西維珍尼亞州");
check("detect.region cn-hk", d && d.region, "cn-hk");
check(
  "detect.confidence >= 30",
  d && d.confidence >= 30,
  true,
);

// 4. Custom words — array form.
check(
  "convert_with_custom (array)",
  convert_with_custom("买软件", "s2t", [["软件", "軟體"]]),
  "買軟體",
);

// 5. Custom words — string form (opencc-js DictLike compat).
check(
  "convert_with_custom (string)",
  convert_with_custom("香蕉", "s2t", "香蕉 banana|蘋果 apple"),
  "banana",
);

// 6. Introspection.
check("listConfigs().length", listConfigs().length, 16);
check("listConfigs includes s2twp", listConfigs().includes("s2twp"), true);
check("listLocales().length", listLocales().length, 6);
check("listLocales includes cn-tw", listLocales().includes("cn-tw"), true);

// 7. Semantic region flags.
check(
  "configForRegionPair(cn-s, cn-tw)",
  configForRegionPair("cn-s", "cn-tw"),
  "s2twp",
);
check(
  "configForRegionPair(cn-s, cn-hk)",
  configForRegionPair("cn-s", "cn-hk"),
  "s2hkp",
);

// 8. Converter class — strictly better than opencc-js's closure factory:
//    exposes .config, .convertWithCustom, .withCustom.
const c = new Converter("s2twp");
check("Converter c.config", c.config, "s2twp");
check("Converter c.convert 信息", c.convert("信息"), "資訊");
check(
  "Converter c.convertWithCustom",
  c.convertWithCustom("买软件", [["软件", "軟體"]]),
  "買軟體",
);

const cRegion = Converter.forRegion("cn-s", "cn-tw");
check("Converter.forRegion config", cRegion.config, "s2twp");
check("Converter.forRegion convert 鼠标", cRegion.convert("鼠标"), "滑鼠");

const cBaked = c.withCustom([["软件", "軟體"]]);
check("Converter.withCustom convert", cBaked.convert("买软件"), "買軟體");

console.log(`\n${passed} passed, ${failed} failed`);
process.exit(failed === 0 ? 0 : 1);