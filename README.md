# ORAI SWAP V3

## build wasm sdk

`cd wasm && wasm-pack build --features tsify/js`

## convert all wasm_bingen to camel case

`node convert.js`

```js
const { readdirSync, readFileSync, writeFileSync } = require('fs');
const { join } = require('path');

function walk(dir, ext = '.rs') {
  return readdirSync(dir, { withFileTypes: true })
    .flatMap((file) => (file.isDirectory() ? walk(join(dir, file.name), ext) : file.name.endsWith(ext) ? join(dir, file.name) : null))
    .filter(Boolean);
}

const reg = /(?<=#\[wasm_bindgen)\]([\n\t\s])+(?=pub\s+fn\s+([\w_]+)\()/g;

const rustFiles = walk('wasm');

for (const file of rustFiles) {
  const fileContent = readFileSync(file)
    .toString()
    .replace(reg, (_, g1, g2) => {
      const fnName = g2
        .split('_')
        .map((part, i) => (i > 0 ? part[0].toUpperCase() + part.substr(1) : part))
        .join('');
      return `(js_name = ${fnName})]${g1}#[allow(non_snake_case)]${g1}`;
    });

  writeFileSync(file, fileContent);
}
```
