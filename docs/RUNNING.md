# Run the Solochain on Linux or macOS

This guide takes you from a fresh Linux or macOS machine to a running development chain that
you drive from your browser with [Polkadot-JS Apps](https://polkadot.js.org/apps/). By the end
you will have sent transactions to the `counter` and `ticketing` pallets and read their state
back.

Plan for the first build to take a while: it compiles about 1,200 crates. Later builds only
recompile what changed.

## What you need

| | Linux | macOS |
|---|---|---|
| Tested on | Ubuntu 22.04 and 24.10 (also under WSL2 on Windows) | macOS on Apple silicon (M1 Max, M4 Max) |
| Memory | 8 GB minimum, 16 GB recommended | 8 GB minimum, 16 GB recommended |
| Free disk | 15 GB or more for the `target` build folder | 15 GB or more for the `target` build folder |
| Browser | A recent Chrome, Edge, Firefox or Brave | A recent Chrome, Edge, Firefox or Brave (see [Safari](#safari)) |

## 1. Install the system packages

### Linux (Ubuntu and Debian)

```sh
sudo apt update
sudo apt install --assume-yes build-essential git clang curl libssl-dev llvm libclang-dev \
  libudev-dev make protobuf-compiler
```

For Fedora, Arch or openSUSE, use the package list for your distribution from the official
[Polkadot SDK install guide](https://developers.polkadot.com/parachains/install-polkadot-sdk/).

### macOS

1. Install the Xcode Command Line Tools. They provide `clang`, `make` and `git`:

   ```sh
   xcode-select --install
   ```

2. Install [Homebrew](https://brew.sh) if you don't have it, then the two libraries the build
   needs:

   ```sh
   brew update
   brew install protobuf openssl
   ```

## 2. Install Rust

Install `rustup`, the Rust toolchain manager, and accept the default installation:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

Then, from the root of this repository, install the toolchain the project pins in
[`rust-toolchain.toml`](../rust-toolchain.toml) (Rust 1.93.0 with both WebAssembly targets):

```sh
cd solochain-hack-starter
rustup toolchain install
```

You don't need to change your default toolchain. `rustup` switches to the pinned one whenever
you run `cargo` inside this folder.

## 3. Build the node

```sh
cargo build --release --locked
```

`--locked` builds exactly the dependency versions in `Cargo.lock`. The result is
`./target/release/solochain-starter-node`, which contains both the node and the runtime it runs.

## 4. Start a development chain

```sh
./target/release/solochain-starter-node --dev
```

`--dev` starts a single-node chain with Alice as the only block author and the sudo account. The
chain is ready when the log shows these lines, with a new block every 6 seconds:

```text
Running JSON-RPC server: addr=127.0.0.1:9944,[::1]:9944
🎁 Prepared block for proposing at 1 ...
🏆 Imported #1 (0x62a5…bcc9 → 0x57a7…5e34)
```

Leave this terminal open. The node listens on `ws://127.0.0.1:9944`, which is the address the
browser connects to.

| To | Run |
|---|---|
| Stop the chain | `Ctrl+C` in the node's terminal |
| Start over with a fresh chain | Restart with `--dev`. Its state lives in a temporary folder that is deleted on exit |
| Keep state between restarts | `./target/release/solochain-starter-node --dev --base-path ./chain-data` |
| Wipe that saved state | `./target/release/solochain-starter-node purge-chain --dev --base-path ./chain-data` |
| See more detail in the log | Add `-lruntime=debug`, or `-ldebug` for everything |

## 5. Open Polkadot-JS Apps

Open this link while the node is running:

**<https://polkadot.js.org/apps/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer>**

The `rpc` part of the link points Apps at your local node. You can also switch to it by hand:
click the network name in the top-left corner, open **Development**, select **Local Node** and
click **Switch**.

**Allow local network access when the browser asks.** Current Chrome, Edge, Firefox and Brave
versions ask before a website can connect to a program running on your own computer, and Apps is
a website connecting to your node. If you dismissed or blocked the prompt, open the site settings
for `polkadot.js.org` (the icon to the left of the address bar), allow local network access, and
reload the page. In Brave, also check `brave://settings/content/localhostAccess`.

The connection works when the Explorer shows the chain name **Development** in the top-left
corner and the block numbers increase every 6 seconds.

### Safari

Safari has historically blocked `ws://` connections to `127.0.0.1` from `https://` pages. If
Apps stays on "Connecting" in Safari, use another browser or run Apps locally as described below.

### Run Apps on your own machine (no prompts, works in every browser)

If the hosted Apps can't reach your node, run the official Apps image with
[Docker](https://docs.docker.com/get-docker/):

```sh
docker run --rm -it --name polkadot-ui -p 3000:80 jacogr/polkadot-js-apps:latest
```

Then open <http://localhost:3000/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer>. Because the page
itself comes from your machine, browsers don't treat the connection to the node as crossing into
your local network. The image is built for `amd64`, so on Apple silicon Docker Desktop runs it
through emulation and takes a little longer to start.

### Alternative: the Polkadot-API console

[dev.papi.how](https://dev.papi.how/) is a newer web console for Polkadot chains. Choose a
custom endpoint and enter `ws://127.0.0.1:9944`. The same browser permission applies.

## 6. Get to know the development accounts

Open **Accounts → Accounts**. On a development chain, Apps lists the well-known test accounts:
Alice, Bob, Charlie, Dave, Eve and Ferdie. Their keys are public, so never use them on a real
network.

Only **Alice**, **Bob** and their `//stash` accounts start with funds. To use another account,
send it some tokens first:

1. Click **Send** on Alice's row.
2. Pick **Charlie** as the recipient and enter `100` in the amount field (the token has 12
   decimals, so this is 100 UNIT).
3. Click **Make Transfer**, then **Sign and Submit**.

Alice is also the `sudo` account, so **Developer → Sudo** runs privileged calls such as
`counter.setCounterValue`.

## 7. Send your first transaction

1. Open **Developer → Extrinsics**.
2. Choose **Alice** as the account, `counter` as the pallet and `increment(amountToIncrement)` as
   the call, and enter `5`.
3. Click **Submit Transaction**, then **Sign and Submit**. A notification in the top-right corner
   reports whether the transaction was included in a block.
4. Open **Network → Explorer**. The recent events list shows `counter.CounterIncremented`.
5. Open **Developer → Chain state**, select `counter` → `counterValue()` and click **+**. The
   value is `5`.

## 8. Try the ticketing pallet

The ticketing pallet sells event tickets, caps resale prices, pays the organizer a royalty on
every resale and refunds tickets if the event is cancelled. Follow the
[step-by-step walkthrough in its README](../pallets/ticketing/README.md#try-it-in-polkadot-js-apps).
It uses the same screens as step 7.

To check the whole flow automatically instead, run the
[end-to-end script](../scripts/e2e/README.md) against your running node.

## Troubleshooting

| Problem | Fix |
|---|---|
| `failed to run custom build command for librocksdb-sys` on Linux | Install `clang` and `libclang-dev` (step 1). |
| `Library not loaded: @rpath/libclang.dylib` on macOS | Run `export LIBCLANG_PATH=/Library/Developer/CommandLineTools/usr/lib` and `export DYLD_FALLBACK_LIBRARY_PATH="$LIBCLANG_PATH:$DYLD_FALLBACK_LIBRARY_PATH"`, then build again. Add both lines to `~/.zshrc` to keep them. |
| `Could not find protoc` | Install `protobuf-compiler` (Linux) or `protobuf` (macOS). |
| The build is killed or the machine freezes while linking | Build with fewer parallel jobs: `cargo build --release --locked -j 2`. |
| `Address already in use` on port 9944 | Another node is still running. Stop it, or start this one with `--rpc-port 9945` and use that port in the Apps link. |
| Apps shows "Connecting" forever | Check that the node is running and that you allowed local network access (step 5). Otherwise run Apps locally with Docker. |
| Apps shows errors after you restarted the chain | Reload the page. A restarted `--dev` chain is a brand-new chain, and Apps still has the old one cached. |
| A transaction fails with `balances.InsufficientBalance` or `Inability to pay some fees` | The account has no funds. Send it some from Alice (step 6). |
| The node runs on another machine or a VM | Forward the port over SSH instead of exposing the RPC to the network: `ssh -L 9944:127.0.0.1:9944 user@host`, then use the links above unchanged. |
