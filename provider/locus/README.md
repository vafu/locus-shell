# locus-provider

`locus-provider` owns typed Locus graph contracts and the Locus-over-D-Bus provider implementation.

The `src/generated.rs` file is generated from `~/proj/locus/schema.yaml` with `locus-codegen`. Regenerate or verify it from the workspace root:

```sh
sh scripts/locus-provider-schema generate
sh scripts/locus-provider-schema check
```

Set `LOCUS_REPO`, `LOCUS_SCHEMA`, or `LOCUS_CODEGEN` when using a non-default local Locus checkout or binary.
