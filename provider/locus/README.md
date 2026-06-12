# locus-provider

`locus-provider` owns generic Locus graph binding primitives and the Locus-over-D-Bus provider implementation.

Schema-specific model markers, path constants, relation constants, and convenience helpers are generated in consuming crates. The development schema used by `dev-widgets` can be regenerated or verified from the workspace root:

```sh
sh scripts/dev-widgets-locus-schema generate
sh scripts/dev-widgets-locus-schema check
```

Set `LOCUS_REPO`, `LOCUS_SCHEMA`, or `LOCUS_CODEGEN` when using a non-default local Locus checkout or binary.
