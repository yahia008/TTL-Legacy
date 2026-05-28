# TTL-Legacy API Reference

## Smart Contract API

### Vault Cloning

#### `clone_vault`

```rust
clone_vault(source_vault_id: u64, new_owner: Address, new_beneficiary: Address) -> u64
```

Clones all settings from a source vault into a new vault. All fields (interval, beneficiaries, metadata, token, release condition, spending limits) are copied verbatim.

---

#### `clone_vault_with_overrides`

```rust
clone_vault_with_overrides(
    source_vault_id: u64,
    new_owner: Address,
    new_beneficiary: Address,
    override_interval: Option<u64>,
    override_beneficiaries: Option<Vec<BeneficiaryEntry>>,
    override_metadata: Option<String>,
) -> u64
```

Clones a vault configuration into a new vault with selective parameter overrides. Fields passed as `None` are copied from the source vault unchanged.

**Parameters**

| Parameter                | Type                           | Description                                                                  |
|--------------------------|--------------------------------|------------------------------------------------------------------------------|
| `source_vault_id`        | `u64`                          | Template vault (must be Locked and owned by `new_owner`)                     |
| `new_owner`              | `Address`                      | Owner of the new vault (must authorize; must be source vault owner)          |
| `new_beneficiary`        | `Address`                      | Primary beneficiary for the new vault                                        |
| `override_interval`      | `Option<u64>`                  | Override check-in interval in seconds, or `None` to copy from source         |
| `override_beneficiaries` | `Option<Vec<BeneficiaryEntry>>`| Override multi-beneficiary split (BPS must sum to 10 000), or `None` to copy |
| `override_metadata`      | `Option<String>`               | Override metadata string (max 256 chars), or `None` to copy                  |

**Returns** the new vault ID.

**Errors**

| Error                | Code | Condition                                               |
|----------------------|------|---------------------------------------------------------|
| `NotOwner`           | 6    | Caller is not the source vault owner                    |
| `AlreadyReleased`    | 7    | Source vault is not in Locked status                    |
| `InvalidBeneficiary` | 17   | `new_owner == new_beneficiary`, or owner in BPS list    |
| `InvalidInterval`    | 2    | `override_interval` is `Some(0)`                        |
| `IntervalTooLow`     | 14   | `override_interval` below configured minimum            |
| `IntervalTooHigh`    | 15   | `override_interval` above configured maximum            |
| `InvalidBps`         | 12   | `override_beneficiaries` BPS sum ≠ 10 000               |
| `InvalidAmount`      | 5    | `override_metadata` exceeds 256 characters              |

**Event emitted:** `v_clo_ov` — `(source_vault_id, new_vault_id, new_beneficiary, check_in_interval)`

**Comparison with `clone_vault`**

| Feature                   | `clone_vault` | `clone_vault_with_overrides` |
|---------------------------|:---:|:---:|
| Copies interval           | ✅  | ✅ (or override) |
| Copies beneficiaries      | ✅  | ✅ (or override) |
| Copies metadata           | ✅  | ✅ (or override) |
| Copies token / conditions | ✅  | ✅               |
| Selective field override  | ❌  | ✅               |

---

## Vault State Snapshots (Disaster Recovery)

### `create_snapshot`

```rust
create_snapshot(vault_id: u64, caller: Address) -> u32
```

Creates a point-in-time snapshot of the vault's mutable state. Only the vault owner may call this. Up to 10 snapshots are retained per vault; once the limit is reached, the oldest slot is overwritten (slots cycle 1–10).

**Captured fields:** `balance`, `beneficiary`, `check_in_interval`, `last_check_in`, `metadata`, `is_paused`, `taken_at`.

**Returns** the snapshot ID (1–10).

**Errors**

| Error     | Code | Condition                    |
|-----------|------|------------------------------|
| `NotOwner`| 6    | Caller is not the vault owner|
| `Paused`  | 10   | Contract is globally paused  |

**Event emitted:** `snap_crt` — `(snapshot_id, taken_at)`

---

### `restore_from_snapshot`

```rust
restore_from_snapshot(vault_id: u64, caller: Address, snapshot_id: u32)
```

Restores the vault to a previously saved snapshot. Only the vault owner may call this. The vault must be in `Locked` status.

**Restored fields:** `balance`, `beneficiary`, `check_in_interval`, `last_check_in`, `metadata`, `is_paused`.

**Errors**

| Error            | Code | Condition                              |
|------------------|------|----------------------------------------|
| `NotOwner`       | 6    | Caller is not the vault owner          |
| `AlreadyReleased`| 7    | Vault is Released or Cancelled         |
| `VaultNotFound`  | 3    | `snapshot_id` does not exist           |
| `Paused`         | 10   | Contract is globally paused            |

**Event emitted:** `snap_rst` — `(snapshot_id, taken_at)`

---

### `get_snapshot`

```rust
get_snapshot(vault_id: u64, snapshot_id: u32) -> Option<VaultSnapshot>
```

Returns the snapshot data for the given slot, or `None` if it does not exist.

---

### `get_snapshot_count`

```rust
get_snapshot_count(vault_id: u64) -> u32
```

Returns the total number of snapshots ever taken for the vault (not capped at 10). Use `count % 10 + 1` to find the next slot that will be written.

---

### `VaultSnapshot` type

```rust
pub struct VaultSnapshot {
    pub snapshot_id: u32,
    pub vault_id: u64,
    pub taken_at: u64,
    pub balance: i128,
    pub beneficiary: Address,
    pub check_in_interval: u64,
    pub last_check_in: u64,
    pub metadata: String,
    pub is_paused: bool,
}
```

Base URL: `http://localhost:3000`

### Reminder Preferences

#### POST `/api/vaults/{vault_id}/reminder-preferences`

Create or update reminder preferences for a vault.

**Request Body** (`application/json`)

| Field                 | Type            | Required | Description                                           |
|-----------------------|-----------------|----------|-------------------------------------------------------|
| `channels`            | array of string | Yes      | One or more of: `"email"`, `"sms"`, `"push"`         |
| `hours_before_expiry` | integer (> 0)   | Yes      | Hours before TTL expiry to send the first reminder    |
| `frequency`           | string          | Yes      | `"once"`, `"daily"`, or `"hourly"`                   |

**Responses:** `200` saved object · `422` validation error · `500` server error

---

#### GET `/api/vaults/{vault_id}/reminder-preferences`

Retrieve current reminder preferences for a vault.

**Responses:** `200` preferences object · `404` not found · `500` server error

---

### Scheduler Behaviour

The background scheduler polls every 60 seconds. For each vault with stored preferences it:

1. Fetches the vault's TTL remaining (hours) from the Stellar RPC.
2. Compares against `hours_before_expiry`.
3. Fires reminders on the configured channels according to `frequency`:
   - `once` — fires exactly once when TTL enters the window.
   - `daily` — fires every 24 hours while inside the window.
   - `hourly` — fires every hour while inside the window.

Preferences are stored off-chain in the backend SQLite database and are never written to the Soroban contract.


---

## Configurable Countdown Notifications

### `set_countdown_config`

```rust
set_countdown_config(vault_id: u64, caller: Address, thresholds: Vec<u64>)
```

Sets the countdown notification thresholds for a vault. Only the vault owner may call this. Each threshold is a number of seconds before expiry at which `check_countdown` will emit a `cd_notif` event. Pass an empty vec to disable notifications. Calling this also clears any previously fired threshold flags, so all thresholds will fire fresh on the next countdown cycle.

**Errors**

| Error     | Code | Condition                    |
|-----------|------|------------------------------|
| `NotOwner`| 6    | Caller is not the vault owner|

**Event emitted:** `set_cd` — `(thresholds)`

---

### `get_countdown_config`

```rust
get_countdown_config(vault_id: u64) -> CountdownConfig
```

Returns the countdown config for a vault. If not explicitly set, returns the default thresholds: 604800 (7 days), 259200 (3 days), 86400 (1 day).

---

### `check_countdown`

```rust
check_countdown(vault_id: u64) -> u64
```

Checks the vault's remaining TTL against its configured thresholds and emits a `cd_notif` event for each threshold that has been crossed since the last check-in or config reset. Each threshold fires **at most once** per countdown cycle. Fired flags are cleared automatically when the owner calls `check_in` or `set_countdown_config`.

Can be called by anyone — intended for off-chain keepers, cron jobs, or reminder services.

**Returns** the remaining TTL in seconds (0 if expired or vault is not Locked).

**Event emitted:** `cd_notif` — `(threshold_seconds, ttl_remaining)` — one per newly crossed threshold.

---

### `CountdownConfig` type

```rust
pub struct CountdownConfig {
    /// Sorted descending list of thresholds in seconds.
    /// Default: [604800, 259200, 86400] (7d, 3d, 1d)
    pub thresholds: Vec<u64>,
}
```

### Integration example

An off-chain keeper calls `check_countdown` on a schedule (e.g. every hour). When the vault TTL drops below a threshold, the contract emits a `cd_notif` event. The keeper reads the event and dispatches email/SMS/push reminders via the backend reminder service.
