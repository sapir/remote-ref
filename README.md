# remote-ref

This library allows sharing references to objects across thread boundaries,
even when those objects aren't `Send` or `Sync`. The objects themselves are
held in an `ObjectStore` struct that isn't necessarily `Send`/`Sync`, and so
the objects can still only be actually used on the owning thread.

This differs from some other crates such as
[`fragile`](https://crates.io/crates/fragile) or
[`send_wrapper`](https://crates.io/crates/send_wrapper) in that the access rule
is enforced at compile time, and that the `ObjectStore` (currently) requires an
extra garbage collection function to be called manually.
