
# persistent-json

Persistent JSON Collection for Rust

A persistent JSON collection for Rust. The advantages to this is that clones are fast; O(1),
regardless of the size of the structure. This advantage comes at a cost, however as the structure is
not thread-safe at the moment. Additionally, the data structure is slightly slower than its non-
persistent counterpart.
