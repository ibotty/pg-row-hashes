# PostgreSQL Extension for fingerprints

This `pgrx` PostgreSQL extension provides a couple of functions to calculate a fingerprint of given data.

The following types of functions are provided.

 * `id_`: will fingerprint the variadic arguments in order,
 * `checksum_` will fingerprint `jsonb_build_object`-alike constructed map of `key` -> `value`.
   - `checksum_[..]_extendable` will skip `NULL`-values, i.e. `checksum_farmhash_extendable('key1', 'value', 'key2', NULL)` will have the same fingerprint as `checksum_farmhash_extendable('key1', 'value')`, while
   - `checksum_[..]` will keep `key2` in the fingerprint.

It provides [seahash](https://ticki.github.io/blog/seahash-explained/) and [farmhash](https://github.com/google/farmhash) variants of every function.

Seahash will return a `BIGINT` (64 bit) and Farmhash a `UUID` (128 bit).

