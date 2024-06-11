
# build web
wasm-pack build --target web --features tsify/js
cp pkg/oraiswap_v3_wasm.js pkg/oraiswap_v3_wasm.d.ts pkg/oraiswap_v3_wasm_bg.wasm npm

# build nodejs
wasm-pack build --target nodejs --features tsify/js
cp pkg/oraiswap_v3_wasm.js npm/oraiswap_v3_wasm_main.js 
cp pkg/oraiswap_v3_wasm_bg.wasm npm/oraiswap_v3_wasm_bg_main.wasm 
sed -i '' 's|oraiswap_v3_wasm_bg.wasm|oraiswap_v3_wasm_bg_main.wasm|g' npm/oraiswap_v3_wasm_main.js

# # deploy npm
# cd npm 
# yarn publish --access public --patch
