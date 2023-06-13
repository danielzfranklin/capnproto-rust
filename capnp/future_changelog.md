## v0.18

- Generate methods to initialize and set list fields

```rust
let value = [0, 1, 2, 3];
builder.init_value_to(&value);

struct UserId(Uuid);

let value = [uuid!("25897e42-092e-11ee-8b5e-bb6556dbd039")];
builder.init_value_to_with(&value, |builder, uuid| {
    let (d1, d2, d3, d4) = uuid.to_fields_le();
    .set_d1(d1);
    .set_d2(d2);
    .set_d3(d3);
    .set_d4(u64::from_le_bytes(*d4));
})?;
```
