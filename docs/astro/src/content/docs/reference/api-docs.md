---
title: Rust API documentation
description: Build the generated Rust API reference from the Serenade workspace.
---

Browse the [hosted Rust API reference](https://interchouette-itc.github.io/Serenade/). Rustdoc is
generated from the source code and is separate from these guides.

To build the API reference locally, run this from the Serenade repository root:

```sh
make doc
```

Open `docs/api-rust/index.html` after the command completes. The generated API reference is
maintained by the Rust documentation build; do not hand-edit its generated pages.

For build details, see the
[Rust API documentation guide](https://github.com/Interchouette-ITC/Serenade/blob/dev/docs/api-rust/README.md).
