set working-directory := '..'

mod tooling
mod wasm

port := '8080'

# List the available recipes.
[default]
list:
    @just --list ide

# Run the page's tests in Node.
test: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust node --test 'ide/page/tests/*.test.mjs'

# Type-check the VS Code extension and run its tests in Node.
check: (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt sh -c 'cd ide/vscode && npm ci && npx tsc && node --test tests/*.test.mjs'


# Build the VS Code extension into ide/vscode/out, with the page and the bindings.
vscode-build: wasm::build (tooling::build "fmt")
    ./scripts/docker/run-tooling.sh kaalang-fmt sh -c 'cd ide/vscode && npm ci && npx tsc'
    mkdir -p ide/vscode/out/page ide/vscode/out/wasm
    cp ide/page/*.js ide/page/page.css ide/vscode/out/page/
    cp ide/vscode/src/webview.js ide/vscode/out/
    cp target/wasm/web/kaalang_js.js ide/vscode/out/wasm/kaalang_js.mjs
    cp target/wasm/web/kaalang_js_bg.wasm ide/vscode/out/wasm/

# Open this repository in VS Code with the extension from ide/vscode.
vscode: vscode-build
    code --extensionDevelopmentPath="$PWD/ide/vscode" .

# Serve the page's test bench at http://127.0.0.1:8080/ide/page/bench/ and its race checks at http://127.0.0.1:8080/ide/page/bench/checks.html.
bench: wasm::build
    TOOLING_PORT={{ port }} ./scripts/docker/run-tooling.sh kaalang-rust node ide/page/bench/serve.mjs . {{ port }}
