name=oraiswap_v3_wasm
# build web
wasm-pack build --target web --features tsify/js
cp pkg/${name}.js pkg/${name}.d.ts pkg/${name}_bg.wasm npm

# build nodejs
wasm-pack build --target nodejs --features tsify/js
cp pkg/${name}.js npm/${name}_main.js 
cp pkg/${name}.d.ts npm/${name}_main.d.ts 
cp pkg/${name}_bg.wasm npm/${name}_bg_main.wasm 
sed -i '' 's|${name}_bg.wasm|${name}_bg_main.wasm|g' npm/${name}_main.js

# # deploy npm
# cd npm 
# yarn publish --access public --patch
