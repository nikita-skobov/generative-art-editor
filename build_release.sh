cargo build --release --target wasm32-unknown-unknown
cp ./target/wasm32-unknown-unknown/release/generative-art-editor.wasm ./game.wasm
