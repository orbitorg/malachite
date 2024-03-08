# Validator Set Updates

Principles:

- The validator set used by Starknet does not change within an epoch.
- The installation of a new, updated validator set is delayed by two epochs.
- Updates in Starnet validator set are produced by Ethereum transactions and contracts
- Validators must acknowledge to Ethereum the inclusion of validator set
  updates in committed blocks

The Starkware team has produced a document with the Starknet specification and
requirements on [Google docs][starkware-spec].

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

In the [Starknet spec][starkware-spec] a different nomenclature is adopted:

- Epoch `e` is represented by "Starknet Validator Epochs" or _SVE_;
- The length of an epoch `E` is represented by _N_, the number of blocks in a _SVE_.

### Use of Epochs

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
blockchain:

- `S(H_0)` is known a priori by all participants (e.g., from genesis);
- For every height `H > H_0`, `S(H) = S(H-1) + updates(H)`.

We use the `+` operator to represent the application to `S(H)` of validator set
updates from `updates(H)`.
Another possible representation would be `S(H) = apply(S(H-1), updates(H))`,
where `apply` is a deterministic function.
The validator set updates in `updates(H)` are applied to `S(H)` in the order in
which they appear in block `H`.

The first height of the blockchain `H_0` is typically `H_0 = 0` for new
blockchains.
Due to the possibility of [forks][forks-spec], we consider a fork as a new
instance of the blockchain, which can start from an arbitrary height `H_0`.


## Validator Sets

We denote by `V(e)` the validator set adopted in heights of an epoch `e`.
This means that instances of consensus from height `first(e) = e * E` to
height `last(e) = (e + 1) * E - 1` use the same validator set `V(e)`.

The validator set `V(e)` used in epoch `e` is the validator set computed two
epochs before `e`, namely the value of the state variable `S` in the last
height of epoch `e - 2`.
In the case where epoch `e - 2` does not exist or it is not part of the 
current branch of the blockchain, the initial state `S(H_0)` is considered.
More precisely:

- `V(e) = S(max{H', H_0})`, where height `H' = last(e - 2) = (e - 1) * E - 1`

The validator set of the first epoch present in a branch of the blockchain is
the validator set `S(H_0)`, where `H_0` is the first height of that branch.
This distinction is important, in terms of validator sets, because when there
is a [fork][forks-spec] in the blockchain, the new branch starting from height
`H_0` has is validator set _reset_ to `S(H_0)`.
This means that `S(H_0)` is not computed in the usual way, i.e., derived from
`S(H_0 - 1)`, but externally _enforced_.

> Example: if the current branch starts in height `H_0 = 3` and `E = 3`,
> then the validator sets:
>  - `V(3) = S(5)`, as `5` is the last height of epoch `e - 2 = 1`, formed by
>    heights `3, 4, 5`.
>  - `V(2) = S(3)`, the initial validator set `S(H_0)` of the branch,
>    as `last(e - 2) = 2 < H_0 = 3`.
>  - `V(1) = S(3)`, the initial validator set `S(H_0)` of the branch,
>    as epoch `e - 2 = -1` does not exist.

It is possible to simplify the algorithm to compute the validator set for a
regular epoch, i.e., that is not the first one of a branch, if we introduce an
intermediate variable `nextV(e)`, which stores validator set for epoch `e + 1`.
At the beginning of a new epoch `e`, in height `H = first(e) = e * E`,
we update the state variables as follows:

- `V(e) = nextV(e - 1)`, i.e., the current validator set is the previous next
  validator set.
- `nextV(e) = S(H - 1)`, i.e., the next validator set is the one just computed
  in height `H - 1 = last(e - 1)`.

At the end, this algorithm is very similar to the one adopted in CometBFT,
if we replace heights by epochs.

In the [Starknet spec][starkware-spec] a different nomenclature is adopted:

- `V(e)`, the current validator set, is referred to as _V_;
- `nextV(e)` is referred to in the text as _Staged Updates_;
- `S(H)`, where `e(H) = e` is reflected to in the text as _Pending Updates_.

## Interaction with L1

Up to here, the specification considers the protocol for configuring and
updating the validator set in Starknet, a Tendermint-based blockchain.
This is the second layer (L2) of the protocol.
The first layer (L1) of the protocol is implemented by contracts in the
Ethereum blockchain.
This section overviews the interaction between L1 and L2.

The [Starknet spec][starkware-spec] contains a much more detailed description
of this interaction, in particular details about the operation of L1.
In the section we focus of the components more relevant for validator set
updates in L2.

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

> The inclusion of validator set updates produced by L1 in Starknet blocks are
> represented by green arrows in diagrams illustrating the operation of the
> protocol in the [Starknet specification][starkware-spec].

The set `updates(H)` introduced in the previous [Updates section](#updates) is
formed by the above defined transactions.

> Not sure if transactions committed to Ethereum are included as is in Starknet
> blocks, or if Starknet validator updates are derived from the Ethereum
> transaction. This should not have impact on this specification.

### Acknowledgments

The Ethereum contracts expect to receive from Starknet validators proofs for
every committed block.
The proofs of Starknet blocks including validator set updates play the role of
acknowledging that they are committed to L2.

When a validator set update transaction `u` is committed to Ethereum, `u` is
added to a list of pending updates in the associated Ethereum contract.
Once a proof of a Starknet block `H` containing `u` is received and validated,
the Ethereum contract that has produced `u` marks the validator set update `u`
as _completed_.

> The submission of proofs of L2 blocks including validator set updates to L1
> are represented by blue arrows in diagrams illustrating the operation of the
> protocol in the [Starknet specification][starkware-spec].

### Deadlines

Once contracts in Ethereum produce a validator set update transaction, they
also define a _deadline_ for receiving an acknowledgment that the validator set
update was committed to Starknet.
When Starknet validators, from the current validator set, fail to produce an
acknowledgment by the established deadline, the protocol for initiating a
fork in the Starknet blockchain is triggered.
Refer to the [Fork Protocol specification][forks-spec] for details.

> The deadlines for L1 to receive acknowledgments from L2 for a given validator
> set update are represented by dashed-red arrows in diagrams illustrating the
> operation of the protocol in the [Starknet specification][starkware-spec].

The deadlines in L1 are defined based on the duration of L1 epochs, referred as
"Ethereum Validator Epochs" or _EVE_ in the [Starknet specification][starkware-spec].
At the current state of this specification, the duration of an _EVE_ is given
by _T_, given in time.
The timestamps of Ethereum blocks are thus assumed to be a trustful source of
real time.

More specifically, the deadline for a validator set update transaction produced
during an Ethereum epoch _EVE_ is set to the beginning of the Ethereum epoch
_EVE + D_, where _D_ is a parameter.
Notice that validator set updates produced in different Ethereum blocks
belonging to the same _EVE_ have the same deadline _EVE + D_.

### Enforcement

There is a second mechanism implemented by Ethereum contracts (L1) to enforce
that the produced validator set updates are indeed applied by Starknet (L2).
It is complementary and works together with the deadlines.

When a Starknet validator proves a block, the proof is signed by that validator
and somehow includes the validator set that has produced and signed the block.
The Ethereum contracts know the validator sets that should produce and sign
blocks and proofs at any Starknet epoch.
A proof, therefore, is only accepted by L1 if:

1. The proof was produced at height `H` by a validator belonging to the
   expected validator set for epoch `e(H)`;
2. And the proven block, with height `H' < H`,  is signed by the expected
   validator set for epoch `e(H')`.

If the Starknet validator set (L2) is not being updated according to the
validator set updates produced by L1, then proofs submitted by L2 will start
being rejected by L1, from some point in time or from some Starknet height.
The time component is associated to the deadlines for receiving L2
acknowledgments that validator set updates were committed to Starknet.
The height component is associated to the epochs at which scheduled (as in
committed) validator set updates should be applied to Starknet so that the
expected validator sets become effective.

The main point here is that if Starknet validators do not behave as expected by
L1, i.e., do not update the validator set accordingly, the blocks and block
proofs produced by these validators will eventually be rejected by L1.
Since L1 is the "source of truth", these blocks and blocks proof become
useless, as they will not be accepted by L1.

[starkware-spec]: https://docs.google.com/document/d/1OaYLh9o10DIsGpW0GTRhWl-IJiVyjRsy7UttHs9_1Fw
[forks-spec]: ./forks-TODO.md
