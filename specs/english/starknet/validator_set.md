# Validator Set Updates

Principles:

- The validator set used by Starknet does not change within an epoch.
- The installation of a new, updated validator set is delayed by two epochs.
- Updates in Starnet validator set are produced by Ethereum transactions and contracts
- Validators must acknowledge to Ethereum the inclusion of validator set
  updates in committed blocks

## Epochs

The heights of the blockchain are divided into sub-sequences with a predefined
length, that we call an epoch.
We use `e` to identify and epoch, and the constant `E` to represent the length,
in blocks or heights, of any epoch:

- Each height `H` belongs to an epoch `e(H) = H / E`;
- The first height of an epoch `e` is `first(e) = e * E`;
- The last height of an epoch `e` is `last(e) = (e + 1) * E - 1`.

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

## Interaction with L1

Up to here, the specification considers the protocol for configuring and
updating the validator set in Starknet, a Tendermint-based blockchain.
This is the second layer (L2) of the protocol.
The first layer (L1) of the protocol is implemented by contracts in the
Ethereum blockchain.
This section overviews the interaction between L1 and L2.

### Accounts and Staking

Starknet contracts on Ethereum manage Starknet accounts and balances in
the Starknet token STRK.
In order to become a Starknet validator, a node have to transfer tokens from
its account to the Starknet staking contract.
The amount deposited by a validator in Ethereum (L1) defines its voting power
in the Starknet blockchain (L2).

### Updates

Updates in the validator set adopted by Starknet (L2) are produced by contracts
in the Ethereum blockchain (L1).
Transactions that deposit or withdraw funds to or from the staking contract
are reflected in validator set updates to be applied in Starknet,
intended to add, remove, or update the voting power of validators.

Starknet validators are supposed to monitor the Ethereum contract for
transactions that update the validator set.
Once a validator becomes the proposer of a height of consensus,
it includes in the proposed block all outstanding (i.e., not yet included in
previous blocks) validator set updates retrieved from the Ethereum contract.

> Not sure if transactions committed to Ethereum are included as is in Starknet
> blocks, or if Starknet validator updates are derived from the Ethereum
> transaction. This should not have impact on this specification.

The set `updates(H)` introduced in the previous [Updates section](#updates) is
formed by the above defined transactions.

### Acknowledgments

The Ethereum contracts expect to receive from Starknet validators proofs for
every committed block.
The proofs of Starknet blocks including validator set updates play the role of
acknowledging their reception and application.

So when a validator set update transaction `u` is committed to Ethereum, `u` is
added to a list of pending updates in the associated Ethereum contract.
Once a proof of a Starknet block `H` containing `u` is received and validated,
the Ethereum contract that has produced `u` marks the validator set update `u`
as completed.
