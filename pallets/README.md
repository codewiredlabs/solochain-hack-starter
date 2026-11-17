# Pallets

ℹ️ A pallet is a unit of encapsulated logic, with a clearly defined responsibility. A pallet is analogous to a
module in the runtime.

💁 This project contains three custom pallets based on the FRAME framework:

- [`template`](./template/) - the minimal pallet that ships with the Polkadot SDK solochain template.
- [`counter`](./counter/) - a counter anyone can increment or decrement, a gentle first pallet.
- [`ticketing`](./ticketing/README.md) - event ticketing with escrow, capped resale, royalties, refunds and bounded storage cleanup.

👉 Learn more about FRAME
[here](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/polkadot_sdk/frame_runtime/index.html).

🧑‍🏫 Please refer to
[this guide](https://paritytech.github.io/polkadot-sdk/master/polkadot_sdk_docs/guides/your_first_pallet/index.html) for a step by step guide on building your first pallet.