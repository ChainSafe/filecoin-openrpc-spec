# Filecoin Common Node API Specification

This repo is an appendix to the Filecoin Common Node API FIP.

The main document is the [`spec.json`](./spec.json), which is a description of a
set of [JSON-RPC](https://www.jsonrpc.org/) methods as an [OpenRPC](https://spec.open-rpc.org/)
document.

[`tool`](./tool/) contains a binary which was used to generate the above
document, but also of note is a `diff` subcommand which can summarize differences
between two different OpenRPC specifications, which you may wish to use for
conformance checking.

To use `tool`, you should [install rust](https://www.rust-lang.org/tools/install),
and get an overview of the subcommands by running the following from the root of
the repository.
```console
$ cargo run --manifest-path ./tool/Cargo.toml -- openrpc --help
```
