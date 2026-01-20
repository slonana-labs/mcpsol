# Contracts

This feature does not introduce new external APIs or modify existing contracts.

The `list_tools` instruction interface remains unchanged:

```
Input:  [8-byte discriminator][optional 1-byte cursor]
Output: JSON schema via set_return_data (≤1024 bytes)
```

All changes are internal optimizations to the library implementation.
