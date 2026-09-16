const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join, resolve } = require("node:path");

const root = resolve(__dirname, "..");
const packageDir = process.env.DQ_WASM_PACKAGE ?? join(root, "target", "wasm");
const dq = require(join(packageDir, "dq_wasm.js"));

function asset(path) {
  return readFileSync(join(root, "tests/assets", path), "utf8");
}

const source = asset("source/commerce-platform.d2");
const all = asset("perspectives/all.dl");
const dataPath = asset("perspectives/d2-data-path.dl");

assert.equal(dq.transform(source, all, "d2", "mermaid"), asset("expect/d2-all.mmd"));

const document = new dq.Document(source, "d2");
assert.equal(document.transform(dataPath, "d2"), asset("expect/d2-data-path.d2"));

const adversarialD2 = asset("source/adversarial-d2.d2");
const adversarial = asset("perspectives/adversarial-all.dl");
const contractStress = asset("perspectives/contract-stress.dl");
assert.equal(
  dq.transform(adversarialD2, adversarial, "d2", "mermaid"),
  asset("expect/adversarial-d2-all.mmd"),
);

assert.equal(
  dq.transform(adversarialD2, contractStress, "d2", "d2"),
  asset("expect/contract-stress.d2"),
);

const adversarialDocument = new dq.Document(adversarialD2, "d2");
assert.equal(
  adversarialDocument.transform(contractStress, "mermaid"),
  asset("expect/contract-stress.mmd"),
);

const escapedQuery = String.raw`
view_new_node("multiline", "annotation", "Line 1\nLine 2", "rounded").
`;
assert.ok(
  dq.transform(source, escapedQuery, "d2", "mermaid").includes("Line 1<br/>Line 2"),
);

const adversarialMermaid = asset("source/adversarial-mermaid.txt");
const mermaidDocument = new dq.Document(adversarialMermaid, "mermaid");
assert.equal(
  mermaidDocument.transform(adversarial, "d2"),
  asset("expect/adversarial-mermaid-all.d2"),
);

function assertRejected(callback, message) {
  let error;
  try {
    callback();
  } catch (caught) {
    error = caught;
  }
  assert.equal(error?.message ?? String(error), message);
}

assertRejected(
  () => dq.transform(source, all, "dot", "d2"),
  "input_format must be d2|mermaid",
);
assertRejected(
  () => dq.transform(source, all, "d2", "json"),
  "output_format must be d2|mermaid",
);

console.log("wasm-bindgen fixture smoke passed");
