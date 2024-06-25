# OpenRPC Specification for the Filecoin Common Node API

This is an accessory [this FIP](TODO(aatifsyed): add link)

The main document is the [`spec.json`](./spec.json), which is a description of a
set of [JSON-RPC](https://www.jsonrpc.org/) methods in [OpenRPC](https://spec.open-rpc.org/).

Most of the functionality in [`tool`](./tool/) was used to generate the above
document, but also of note is a `diff` function which can summarize differences
between two different OpenRPC specifications, which you may wish to use for
conformance checking.

To use `tool`, you should [install rust](https://www.rust-lang.org/tools/install),
and get started with the following from the root of the repository
```console
$ cargo run --manifest-path ./tool/Cargo.toml -- openrpc --help
```
