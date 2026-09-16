const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join, resolve } = require("node:path");

const root = resolve(__dirname, "..");
const addonPath = process.env.DQ_NODE_ADDON ?? join(root, "target", "node", "dq.node");
const dq = require(addonPath);
const source = readFileSync(join(root, "tests/assets/source/commerce-platform.d2"), "utf8");
const all = readFileSync(join(root, "tests/assets/perspectives/all.dl"), "utf8");
const dataPath = readFileSync(join(root, "tests/assets/perspectives/d2-data-path.dl"), "utf8");
const adversarialD2 = readFileSync(join(root, "tests/assets/source/adversarial-d2.d2"), "utf8");
const adversarialMermaid = readFileSync(
  join(root, "tests/assets/source/adversarial-mermaid.txt"),
  "utf8",
);
const adversarial = readFileSync(
  join(root, "tests/assets/perspectives/adversarial-all.dl"),
  "utf8",
);
const contractStress = readFileSync(
  join(root, "tests/assets/perspectives/contract-stress.dl"),
  "utf8",
);

assert.equal(
  dq.transform(source, all, "d2", "mermaid"),
  readFileSync(join(root, "tests/assets/expect/d2-all.mmd"), "utf8"),
);

const document = new dq.Document(source, "d2");
assert.equal(
  document.transform(dataPath, "d2"),
  readFileSync(join(root, "tests/assets/expect/d2-data-path.d2"), "utf8"),
);

assert.equal(
  dq.transform(adversarialD2, adversarial, "d2", "mermaid"),
  readFileSync(join(root, "tests/assets/expect/adversarial-d2-all.mmd"), "utf8"),
);

assert.equal(
  dq.transform(adversarialD2, contractStress, "d2", "d2"),
  readFileSync(join(root, "tests/assets/expect/contract-stress.d2"), "utf8"),
);

const adversarialDocument = new dq.Document(adversarialD2, "d2");
assert.equal(
  adversarialDocument.transform(contractStress, "mermaid"),
  readFileSync(join(root, "tests/assets/expect/contract-stress.mmd"), "utf8"),
);

const mermaidDocument = new dq.Document(adversarialMermaid, "mermaid");
assert.equal(
  mermaidDocument.transform(adversarial, "d2"),
  readFileSync(join(root, "tests/assets/expect/adversarial-mermaid-all.d2"), "utf8"),
);

function assertRejected(callback, message) {
  let error;
  try {
    callback();
  } catch (caught) {
    error = caught;
  }
  assert.equal(error?.message, message);
}

assertRejected(
  () => dq.transform(source, all, "dot", "d2"),
  "input_format must be d2|mermaid",
);
assertRejected(
  () => dq.transform(source, all, "d2", "json"),
  "output_format must be d2|mermaid",
);

console.log("Node N-API fixture smoke passed");
