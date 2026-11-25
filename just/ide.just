set working-directory := '..'

mod tooling
mod wasm

port := '8080'

# List the available recipes.
[default]
list:
    @just --list ide

# Run every check of the page, the VS Code extension, and the sample workspace.
test: page-test vscode-test sample-check

# Run the page's tests in Node.
page-test: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust node --test 'ide/page/tests/*.test.mjs'

# Show the page in a browser with a stub host at http://127.0.0.1:8080/ide/page/bench/, and its race checks at http://127.0.0.1:8080/ide/page/bench/checks.html.
page-open: wasm::build
    TOOLING_PORT={{ port }} ./scripts/docker/run-tooling.sh kaalang-rust node ide/page/bench/serve.mjs . {{ port }}

# Type-check the VS Code extension and run its Node and Cargo integration tests.
vscode-test: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust sh -c 'cd ide/vscode && npm ci --cache ../../.cache/npm && npx tsc && node --test tests/*.test.mjs'

# Build the VS Code extension into ide/vscode/out, with the page and the bindings.
vscode-build: wasm::build (tooling::build "fmt")
    rm -rf ide/vscode/out
    ./scripts/docker/run-tooling.sh kaalang-fmt sh -c 'cd ide/vscode && npm ci --cache ../../.cache/npm && npx tsc'
    mkdir -p ide/vscode/out/page ide/vscode/out/wasm
    cp ide/page/*.js ide/page/page.css ide/vscode/out/page/
    cp ide/vscode/src/webview.js ide/vscode/out/
    cp target/wasm/web/kaalang_js.js ide/vscode/out/wasm/kaalang_js.mjs
    cp target/wasm/web/kaalang_js_bg.wasm ide/vscode/out/wasm/

# Open the sample workspace in VS Code with the extension from ide/vscode.
vscode-open: vscode-build
    code --extensionDevelopmentPath="$PWD/ide/vscode" ide/sample

# Check that the sample workspace builds with each feature set its cases use.
sample-check: (tooling::build "rust")
    ./scripts/docker/run-tooling.sh kaalang-rust cargo check --manifest-path ide/sample/Cargo.toml --workspace --all-targets
    ./scripts/docker/run-tooling.sh kaalang-rust cargo check --manifest-path ide/sample/Cargo.toml --workspace --all-targets --all-features
    ./scripts/docker/run-tooling.sh kaalang-rust cargo check --manifest-path ide/sample/Cargo.toml --workspace --all-targets --no-default-features
