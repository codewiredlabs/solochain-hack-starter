# Substrate Node Hackathon Starter Boilerplate

A fresh [Substrate](https://polkadot.com/platform/sdk) node, ready for hacking :rocket:

A standalone version of the official polkadot template was forked to create this boilerplate, and it is available in the consolidated [Polkadot SDK Repository](https://github.com/paritytech/polkadot-sdk-solochain-template). 

Polkadot Engineers suggests using this stand-alone version to start a new Relaychain Node project. All bugs, suggestions, and feature requests should be made upstream in the [Polkadot SDK](https://github.com/paritytech/polkadot-sdk/tree/master/substrate) repository.

* 🔧 This standalone versions' runtime is configured with a single default pallet (pallet-template) as a starting point, and a handful of ready-made substrate framework pallets such as a [Balances pallet](https://paritytech.github.io/polkadot-sdk/master/pallet_balances/index.html). 

* 👉 We took it one step further to demystify Substrate Pallets for intermediate and senior Rust developers who would love to get there hands dirty with creating custom Pallets on Substrate's FRAME (Framework for Runtime Aggregation of Modularized Entities), which is the business logic runtime engine that drives the polkadot network of Substrate blockchains. For this initial commit, we have added a basic Counter Pallet (pallet-counter) to get your feet wet and gently introduce fellow Rustaceans to the amazing world of decentralized web using Substrate Extrinsics and the Substrate FRAME. The second pallet is a real world use case: [Event Ticketing (pallet-ticketing)](./pallets/ticketing/README.md), with escrowed payments, capped resale with organizer royalties, check-in, cancellation with refunds, and settlement. A `Product Supply Chain` pallet may follow.

## High Level Code Base Structure

A Polkadot SDK based project such as this one consists of:

* 💿 a [Node](./node/README.md) - the binary application.
* 🧮 the [Runtime](./runtime/README.md) - the core logic of the solochain node.
* 🎨 the [Pallets](./pallets/README.md) - from which the runtime is constructed.

## Versions

| Component | Version |
|---|---|
| Polkadot SDK | `polkadot-stable2609-2` (node v1.25.2) |
| Rust | `1.93.0`, pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) |

`rustup` reads `rust-toolchain.toml` and installs the pinned compiler, the `rust-src` component and both Wasm targets
the first time you run `cargo` in this folder, so the manual `rustup target add` steps below are only needed if you
want those targets on your default toolchain as well.

## Getting Started

* 🦀 This project is using the Rust Programming language.

* 🛠️ Depending on your operating system and Rust version, there might be additional
packages required to successfully compile this code base - We have outlined the most up to date steps for Linux and macOS users below.

* 👉 Check this
[Rust installation instructions](https://www.rust-lang.org/tools/install) and [Polkadot SDK Dependencies](https://developers.polkadot.com/parachains/install-polkadot-sdk/) for your operating system if yours isn't covered below.

### Setup Linux (Tested on Ubuntu 24.10 and on Ubuntu 22.04 under WSL2)
Rust supports most Linux distributions. Depending on the specific distribution and version of the operating system you use, you might need to add some software dependencies to your environment. In general, your development environment should include a linker or C-compatible compiler, such as clang and an appropriate integrated development environment (IDE) like Visual Studio Code.

We have compiled all of the Polkadot dependencies and every component of the Rust Toolchain you will need for Ubuntu 24.10 below:

Install Polkadot Dependencies:
```sh
sudo apt update
sudo apt install --assume-yes build-essential git clang curl libssl-dev llvm libclang-dev libudev-dev make protobuf-compiler
```
Download the rustup installation program and use it to install Rust:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Update your current shell to include Cargo, the Rust Package Manager:
```sh
source $HOME/.cargo/env
rustc --version
```
Configure the rust toolchain to default stable and nightly versions and also the standard library sources
```sh
rustup default stable
rustup update
rustup target add wasm32-unknown-unknown --toolchain stable-x86_64-unknown-linux-gnu
rustup component add rust-src --toolchain stable-x86_64-unknown-linux-gnu
rustup target add wasm32v1-none --toolchain stable-x86_64-unknown-linux-gnu

rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup component add rust-src --toolchain nightly
rustup target add wasm32v1-none --toolchain nightly
```

### Setup macOS (Tested on macOS M1 Max and M4 Max)

You can install Rust and set up a Substrate development environment on Apple macOS computers with Intel or Apple M1 processors.

Before you install Rust and set up your development environment on macOS, verify that your computer meets the following basic requirements:

* Operating system version is 10.7 Lion or later
* Processor speed of at least 2 GHz. Note that 3 GHz is recommended
* Memory of at least 8 GB RAM. Note that 16 GB is recommended
* Storage of at least 10 GB of available space
* Broadband Internet connection

In most cases, you should use Homebrew to install and manage packages on macOS computers. If you don't already have Homebrew installed on your local computer, you should download and install it before continuing.

To install Homebrew, run the following command in a Terminal with admin access:
```sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/master/install.sh)"
```
Verify your installation `brew --version`

For Apple Silicon users, Protobuf must be installed before the build process can begin. To install it, run the following command `brew install protobuf`

Because the blockchain requires standard cryptography to support the generation of public/private key pairs and the validation of transaction signatures, you must also have a package that provides cryptography, such as openssl. To install openssl and the Rust toolchain on macOS run the following commands in the same order listed:

```sh
brew update
brew install openssl cmake
```
Download the rustup installation program and use it to install Rust by running the following command and follow the prompts displayed to proceed with a default installation:
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
Update your current shell to include Cargo by running the following command `source ~/.cargo/env`.
Configure the rust toolchain to default stable and nightly versions and standard library sources
```sh
rustup default stable
rustup update
rustup target add wasm32-unknown-unknown --toolchain stable-aarch64-apple-darwin
rustup component add rust-src --toolchain stable-aarch64-apple-darwin
rustup target add wasm32v1-none --toolchain stable-aarch64-apple-darwin

rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup component add rust-src --toolchain nightly
rustup target add wasm32v1-none --toolchain nightly
```

## Single-Node Development Chain
Cargo will be our build tool of choice. It is the package manager for the Rust programming language, used to manage dependencies, build, and distribute Rust packages and libraries.

### Compile and Validate the Runtime

🔨 Change directory to the solochain-hackathon-starter folder or whatever you named it after cloning the repo and run the following command to build the node without launching it:
```sh
cargo build --release
```

🐳 Alternatively, build the docker image. As of this update, the Polkadot Images only builds for and works on Linux/amd64 Hosts.
From the project's root folder run the docker command below to build an image to package the most current version of the source code in this repo.
```sh
docker build -t solochain-hackathon-starter .
```

### Run Unit Tests For Your Pallets

It is important to factor in development time to write unit tests when you build Extrinsics that alter the blockchain runtime. Because we are big fans of unit testing, we made sure to add unit tests and benchmarking tests.

To run the unit tests for a pallet, run one of the following commands from the root folder:

```sh
cargo test --package pallet-counter
cargo test --package pallet-ticketing
# include the benchmark tests
cargo test --package pallet-ticketing --features runtime-benchmarks
```

### Start the chain and start producing blocks

The following command starts a single-node development chain that doesn't persist state:
```sh
./target/release/solochain-starter-node --dev
```

To purge the development chain's state, run the following command:
```sh
./target/release/solochain-starter-node purge-chain --dev
```

To start the development chain with detailed logging, run the following command:
```sh
RUST_BACKTRACE=1 ./target/release/solochain-starter-node -ldebug --dev
```

Development chains:

- Maintain state in a `tmp` folder while the node is running.
- Use the **Alice** and **Bob** accounts as default validator authorities.
- Use the **Alice** account as the default `sudo` account.
- Are pre configured with a genesis state (`/node/src/chain_spec.rs`) that includes several pre-funded development accounts.


To persist chain state between runs, specify a base path by running a command similar to the following:

```sh
// Create a folder to use as the db base path
$ mkdir solo-chain-state

// Make use of that folder to store the chain state
$ ./target/release/solochain-starter-node --dev --base-path ./solo-chain-state/

// Check the folder structure created inside the base path after running the chain
$ ls ./solo-chain-state
chains
$ ls ./solo-chain-state/chains/
dev
$ ls ./solo-chain-state/chains/dev
db keystore network
```

### Connect with Polkadot-JS Apps Front-End

After you start the Node locally, you should confirm that the starter node is creating blocks by making sure that the block finalized #No is increasing. You can then interact with it using the hosted version of the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944) front-end by connecting to the local node endpoint. 

Use the menu to Navigate to 'Developer -> Extrinsics' page  and under the Submission tab, click on the drop down that says 'submit the following extrinsic' and select the counter pallet. You can try to decrement or increment the counter by adding your desired amount and clicking on the 'Submit Transaction' button. Complete the transaction by clicking on the 'Sign and Submit' button on the modal dialog that shows up and look out for Success or Failure messages to give you a hint of what went down. You can go back to the Network Explorer 'Network -> Explorer' and take a look at the recent blockchain events to see the changes you made to the blockchain state. Well Done!

A hosted version is also available on [IPFS](https://dotapps.io/). You can also find the source code and instructions for hosting your own instance in the
[`polkadot-js/apps`](https://github.com/polkadot-js/apps) repository.

### Multi-Node Local Testnet

If you want to see the multi-node consensus algorithm in action, see [Simulate a network](https://developers.polkadot.com/reference/tools/zombienet/). Our next Hackathon starter on this same GitHub space will give you the full source code and instructions on how to create a production ready Parachain Node with and NFT pallet that can be deployed as a ParaThread to the [Polkadot Paseo Testnet](https://polkadot.js.org/apps/?rpc=wss%3A%2F%2Fpaseo-rpc.dwellir.com#/explorer). Stay tuned!

## Detailed Template Structure

A Substrate Node project such as this consists of a number of components that are spread across a few directories.

### Node

A blockchain node is an application that allows users to participate in a blockchain network. Substrate-based blockchain nodes expose a number of
capabilities:

- Networking: Substrate nodes use the [`libp2p`](https://libp2p.io/) networking stack to allow the nodes in the network to communicate with one another.
- Consensus: Blockchains must have a way to come to [consensus](https://developers.polkadot.com/reference/parachains/consensus/) on the state of the network. Substrate makes it possible to supply custom consensus engines and also ships with several consensus mechanisms that have been built on top of [Web3 Foundation research](https://research.web3.foundation/Polkadot/protocols/NPoS).
- RPC Server: A remote procedure call (RPC) server is used to interact with Substrate nodes.

There are several files in the `node` directory. Take special note of the following:

- [`chain_spec.rs`](./node/src/chain_spec.rs): A [chain specification](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/reference_docs/chain_spec_genesis/index.html) is a source code file that defines a Substrate chain's initial (genesis) state. Chain specifications are useful for development and testing, and critical when you architect the launch of a production chain. Take note of the `development_config` and `testnet_genesis` functions. These functions are used to define the genesis state for the local development chain configuration. These functions identify some [well-known accounts](https://paritytech.github.io/polkadot-sdk/master/subkey/index.html) and use them to configure the blockchain's initial state.
- [`service.rs`](./node/src/service.rs): This file defines the node implementation. Take note of the libraries that this file imports and the names of the functions it invokes. In particular, there are references to consensus-related topics, such as the [block finalization and forks](https://developers.polkadot.com/reference/parachains/consensus/) and other [consensus mechanisms](https://developers.polkadot.com/reference/parachains/consensus/) such as Aura for block authoring and GRANDPA for finality.

### Runtime

In Substrate, the terms "runtime" and "state transition function" are analogous. Both terms refer to the core logic of the blockchain that is responsible for
validating blocks and executing the state changes they define. The Substrate project in this repository uses [FRAME](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html) to construct a blockchain runtime. FRAME allows runtime developers to declare domain-specific logic in modules called "pallets". At the heart of FRAME is a helpful [macro language](https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/index.html) that makes it easy to create pallets and flexibly compose them to create blockchains that can address [a variety of needs](https://polkadot.com/platform/sdk).

Review the [FRAME runtime implementation](./runtime/src/lib.rs) included in this boilerplate and note the following:

- This file configures several pallets to include in the runtime. Each pallet configuration is defined by a code block that begins with `impl
  $PALLET_NAME::Config for Runtime`.
- The pallets are composed into a single runtime by way of the [#[runtime]](https://paritytech.github.io/polkadot-sdk/master/frame_support/attr.runtime.html)
macro, which is part of the [core FRAME pallet library](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/frame).

### Pallets

The runtime in this project is constructed using many FRAME pallets that ship with [the Substrate repository](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/frame) and a template pallet that is [defined in the `pallets`](./pallets/template/src/lib.rs) directory. Our backend Rust engineers added another pallet from scratch which was also [defined in the `pallets`](./pallets/counter/src/lib.rs) directory. We made a simple counter pallet that allows users to decrement and increment a counter that triggers state changes to the Runtime's transition logic. The [ticketing pallet](./pallets/ticketing/README.md) builds on that with money flows: per-event escrow accounts, capped resale, royalties, refunds and benchmarked weights.

A FRAME pallet is comprised of a number of blockchain primitives, including:

- Storage: FRAME defines a rich set of powerful [storage abstractions](https://paritytech.github.io/polkadot-sdk/master/frame_support/storage/index.html) that makes it easy to use Substrate's efficient key-value database to manage the evolving state of a blockchain.
- Dispatchables: FRAME pallets define special types of functions that can be invoked (dispatched) from outside of the runtime in order to update its state.
- Events: Substrate uses [events](https://paritytech.github.io/polkadot-sdk/master/frame_support/pallet_macros/attr.event.html) to notify users of significant state changes.
- Errors: When a dispatchable fails, it returns an error.

Each pallet has its own `Config` trait which serves as a configuration interface to generically define the types and parameters it depends on.

## Alternatives Installations

If you face any challenge when installing dependencies and building this source directly, consider the following alternatives that uses `Nix`.

Install [nix](https://nixos.org/) and [nix-direnv](https://github.com/nix-community/nix-direnv) for a fully plug-and-play experience for setting up the development environment. To get all the correct dependencies, activate direnv `direnv allow`.

## Getting Help

* 🧑‍🏫 To learn about Polkadot in general, [Polkadot.com](https://polkadot.com/) website is a good starting point.

* 🧑‍🔧 For technical introduction, [here](https://github.com/paritytech/polkadot-sdk#-documentation) are the Polkadot SDK documentation resources.

* 👥 Additionally, there are [GitHub issues](https://github.com/paritytech/polkadot-sdk/issues) and [Substrate StackExchange](https://substrate.stackexchange.com/).

## Contributing
Anyone can contribute to this repository, Fork this repository, make changes, create and issue and reference your issue No. in your Pull Request message and our dedicated team members will handle it. We will also try as much as possible to stay up to date with the mainstream repository by updating this boiler plate with all the Core updates and Releases from the Polkadot SDK Team.
