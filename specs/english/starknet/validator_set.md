# Validator Set Updates

Principles:

- The validator set used by Starknet does not change within an epoch.
- The installation of a new, updated validator set is delayed by two epochs.

## Epochs

The heights of the blockchain are divided into sub-sequences with a predefined
length, that we call an epoch.
We use `e` to identify and epoch, and the constant `E` to represent the length,
in blocks or heights, of any epoch:

- Each height `H` belongs to an epoch `e(H) = H / E`;
- And the first height of an epoch `e` is `first(e) = e * E`.

> Example: if `E = 3`, heights `0, 1, 2` belong to epoch `0`,
> heights `3, 4, 5` belong to epoch `1`, etc.

In the context of validator sets and validator set updates, epochs are relevant
for two reasons:

1. All heights belonging to the same epoch `e` use the same validator set `V(e)`.
2. The validator set `V(e)` used in epoch `e` is defined by the end of epoch `e - 2`.

The operation of validator set updates is described in the following.

## Updates

Lets `updates(H)` be the (ordered) set of validator set updates included in the
block committed at height `H`.
The validator set updates can be a subset of the block's transactions or be
stored in a separate block field.

Let `S` be a state variable that represents a validator set and whose value is
updated when blocks are committed to the blockchain.
We denote by `S(H)` the value of `S` at the end of height `H`.
This means that `S(H)` reflects the processing of the block, or of the
transactions included in the block, committed at height `H`.

> Note that `S(H)` is **not** the validator set adopted in height `H`.
> The role of the state variable `S` is to define the validator set that will
> be adopted in future heights, belonging to future epochs.

We define `S(H)` as follows, where `H_0` denotes the first height of the
current branch of the blockchain:

- `S(H_0)` is known a priori by all participants (e.g., from genesis);
- For every height `H > H_0`, `S(H) = S(H-1) + updates(H)`.

We use the `+` operator to represent the application to `S(H)` of validator set
updates from `updates(H)`.
Another possible representation would be `S(H) = apply(S(H-1), updates(H))`,
where `apply` is a deterministic function.
The validator set updates in `updates(H)` are applied to `S(H)` in the order in
which they appear in block `H`.

## Validator Sets

We denote by `V(e)` the validator set adopted in heights of an epoch `e`.
This means that instances of consensus from height `first(e) = e * E` to
height `last(e) = (e + 1) * E - 1` use the same validator set `V(e)`.

The validator set `V(e)` used in epoch `e` is the validator set computed two
epochs before `e`, namely the value of the state variable `S` in the last
height of epoch `e - 2`.
In the case where epoch `e - 2` does not exist or it is not part of the 
current branch of the blockchain, the initial state `S(H0)` is considered.
More precisely:

- `V(e) = S(max{H', H0})`, where height `H' = last(e - 2) = (e - 1) * E - 1`

The validator set of the first epoch in the current execution is always the
"genesis" validator set `S(H0)`.

> Example: if `E = 3` and the current branch of the blockchain starts at height
> `H0 = 3`, then the validator sets:
>  - `V(3) = S(5)`, as `5` is the last height of epoch `e - 2 = 1`, formed by
>    heights `3, 4, 5`.
>  - `V(2) = S(3)`, the "genesis" state `S(H0)`, as `2 < H0` is the last height
>    of epoch `0`.
>  - `V(1) = S(3)`, the "genesis" state `S(H0)`, as epoch `e - 2 = -1` does not exist.

It is possible to simplify the algorithm to compute the validator set for an
epoch if we introduce an intermediate variable `nextV(e)`, which stores
validator set for epoch `e + 1`.
At the beginning of a new epoch `e`, that is, in height `H = first(e) = e * E`,
we update the current and next validator sets as follows:

- `V(e) = nextV(e - 1)`, i.e., the current validator set is the previous next
  validator set.
- `nextV(e) = S(H - 1)`, i.e., the next validator set is the one just computed
  in height `H - 1 = last(e - 1)`.

At the end, this algorithm is very similar to the one adopted in CometBFT,
if we replace heights by epochs.
